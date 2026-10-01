<script lang="ts">
  import { onMount } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import {
    loadSettings,
    saveSettings,
    scanSkills,
    prepareOperation,
    executeOperation,
    type OperationAction,
    type Harness,
    type OperationEvent,
    type PrepareResponse,
    type ScanResponse,
    type Settings,
    type SkillRow,
  } from './api'

  const harnesses: { id: Harness; label: string }[] = [
    { id: 'codex', label: 'Codex' },
    { id: 'claude', label: 'Claude Code' },
    { id: 'antigravity', label: 'Antigravity IDE' },
    { id: 'open_code', label: 'OpenCode' },
  ]

  let settings: Settings = { source: null, destinations: {}, theme: 'dark' }
  let settingsFile = ''
  let settingsError = ''
  let loadError = ''
  let scan: ScanResponse | null = null
  let activeHarness: Harness = 'codex'
  let search = ''
  let statusFilter = 'all'
  let focusedFolder = ''
  let loading = false
  let saving = false
  let operationPreparing = false
  let operationRunning = false
  let operationError = ''
  let operationEvents: OperationEvent[] = []
  let preparedInstall: PrepareResponse | null = null
  let acknowledgeLinks = false
  let installDialog: HTMLDialogElement
  let scanSequence = 0
  let choosing = false
  let selections: Record<Harness, string[]> = {
    codex: [], claude: [], antigravity: [], open_code: [],
  }

  $: filteredSkills = (scan?.skills ?? []).filter((skill) => {
    const query = search.trim().toLocaleLowerCase()
    const matchesSearch = !query || skill.folderName.toLocaleLowerCase().includes(query)
      || skill.name?.toLocaleLowerCase().includes(query)
    return matchesSearch && (statusFilter === 'all' || skill.status === statusFilter)
  })
  $: focusedSkill = scan?.skills.find((skill) => skill.folderName === focusedFolder) ?? null
  $: selectedCount = selections[activeHarness].length
  $: installSelectedCount = selections[activeHarness].filter((name) => scan?.skills.some((skill) => skill.folderName === name && skill.eligibleActions.includes('install'))).length
  $: updateSelectedCount = selections[activeHarness].filter((name) => scan?.skills.some((skill) => skill.folderName === name && skill.eligibleActions.includes('update'))).length
  $: uninstallSelectedCount = selections[activeHarness].filter((name) => scan?.skills.some((skill) => skill.folderName === name && skill.eligibleActions.includes('uninstall'))).length
  $: controlsDisabled = choosing || saving || operationPreparing || operationRunning
  $: document.documentElement.dataset.theme = settings.theme

  onMount(async () => {
    try {
      const response = await loadSettings()
      settings = response.settings
      settingsFile = response.settingsFile
      settingsError = response.error ?? ''
      if (settings.source) focusedFolder = ''
      await refresh()
    } catch (error) {
      loadError = String(error)
    }
  })

  async function refresh() {
    if (operationRunning || operationPreparing) return
    const sequence = ++scanSequence
    loading = true
    loadError = ''
    try {
      const response = await scanSkills(activeHarness)
      if (sequence !== scanSequence) return
      scan = response
      selections[activeHarness] = selections[activeHarness].filter((name) => response.skills.some((skill) => skill.folderName === name))
      if (!response.skills.some((skill) => skill.folderName === focusedFolder)) {
        focusedFolder = response.skills[0]?.folderName ?? ''
      }
    } catch (error) {
      if (sequence === scanSequence) {
        scan = null
        loadError = String(error)
      }
    } finally {
      if (sequence === scanSequence) loading = false
    }
  }

  async function chooseSource() {
    if (controlsDisabled) return
    choosing = true
    loadError = ''
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Choose skill source folder' })
      if (typeof selected === 'string') await persist({ ...settings, source: selected }, 'source')
    } catch (error) {
      loadError = String(error)
    } finally {
      choosing = false
    }
  }

  async function chooseDestination() {
    if (controlsDisabled) return
    choosing = true
    loadError = ''
    try {
      const selected = await open({ directory: true, multiple: false, title: `Choose ${labelFor(activeHarness)} destination` })
      if (typeof selected === 'string') await persist({ ...settings, destinations: { ...settings.destinations, [activeHarness]: selected } }, 'destination')
    } catch (error) {
      loadError = String(error)
    } finally {
      choosing = false
    }
  }

  async function persist(next: Settings, changed: 'source' | 'destination' | 'theme') {
    if (saving || operationPreparing || operationRunning) return
    saving = true
    loadError = ''
    try {
      await saveSettings(next)
      settings = next
      settingsError = ''
      if (changed === 'source') selections = { codex: [], claude: [], antigravity: [], open_code: [] }
      else if (changed === 'destination') selections[activeHarness] = []
      focusedFolder = ''
      await refresh()
    } catch (error) {
      loadError = String(error)
    } finally {
      saving = false
    }
  }

  async function switchHarness(harness: Harness) {
    if (harness === activeHarness || controlsDisabled) return
    activeHarness = harness
    search = ''
    statusFilter = 'all'
    focusedFolder = ''
    await refresh()
  }

  function toggleSelection(skill: SkillRow) {
    const selected = new Set(selections[activeHarness])
    if (selected.has(skill.folderName)) selected.delete(skill.folderName)
    else selected.add(skill.folderName)
    selections[activeHarness] = [...selected]
  }

  async function toggleTheme() {
    if (controlsDisabled) return
    await persist({ ...settings, theme: settings.theme === 'dark' ? 'light' : 'dark' }, 'theme')
  }

  function labelFor(harness: Harness) {
    return harnesses.find((item) => item.id === harness)?.label ?? harness
  }

  function statusLabel(status: string) {
    return ({
      missing: 'Missing',
      identical: 'Identical',
      different: 'Different',
      installed_only: 'Installed only',
      invalid_source: 'Invalid source',
      ambiguous: 'Ambiguous',
      error: 'Scan error',
    } as Record<string, string>)[status] ?? status
  }

  function formatPath(path: string | null | undefined) {
    return path || 'Not configured'
  }

  async function prepareAction(action: OperationAction) {
    const eligibleCount = action === 'install' ? installSelectedCount : action === 'update' ? updateSelectedCount : uninstallSelectedCount
    if (!scan || !eligibleCount || loading || controlsDisabled) return
    operationPreparing = true
    operationError = ''
    try {
      preparedInstall = await prepareOperation(action, activeHarness, scan.revision, selections[activeHarness])
      acknowledgeLinks = false
      operationEvents = []
      installDialog.showModal()
    } catch (error) {
      loadError = String(error)
    } finally {
      operationPreparing = false
    }
  }

  async function runOperation() {
    if (!preparedInstall || operationRunning) return
    if (preparedInstall.eligible.some((skill) => skill.linkWarnings.length) && !acknowledgeLinks) return
    operationRunning = true
    operationError = ''
    operationEvents = []
    try {
      await executeOperation(preparedInstall.token, acknowledgeLinks, (event) => {
        operationEvents = [...operationEvents, event]
      })
      selections[activeHarness] = []
    } catch (error) {
      operationError = String(error)
    } finally {
      await refreshAfterOperation()
      operationRunning = false
    }
  }

  async function refreshAfterOperation() {
    const sequence = ++scanSequence
    loading = true
    try {
      const response = await scanSkills(activeHarness)
      if (sequence === scanSequence) {
        scan = response
        focusedFolder = response.skills.some((skill) => skill.folderName === focusedFolder)
          ? focusedFolder
          : response.skills[0]?.folderName ?? ''
      }
    } catch (error) {
      loadError = String(error)
    } finally {
      if (sequence === scanSequence) loading = false
    }
  }

  function closeInstallDialog() {
    if (!operationRunning && installDialog?.open) installDialog.close()
    if (!operationRunning) preparedInstall = null
  }
</script>

<svelte:head>
  <title>AI Skill Manager</title>
  <meta name="description" content="Compare local skills across AI coding harnesses" />
</svelte:head>

<div class="app-shell">
  <header class="topbar">
    <a class="brand" href="#main" aria-label="AI Skill Manager home">
      <span class="brand-mark" aria-hidden="true">S</span>
      <span><strong>Skill Manager</strong><small>Local library comparison</small></span>
    </a>
    <div class="topbar-actions">
      <span class="local-indicator"><i></i> Local workspace</span>
      <button class="icon-button" type="button" onclick={toggleTheme} disabled={controlsDisabled} aria-label={settings.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'} title="Change theme">
        {#if settings.theme === 'dark'}☼{:else}☾{/if}
      </button>
    </div>
  </header>

  <main id="main">
    <section class="source-card" aria-labelledby="source-title">
      <div class="source-icon" aria-hidden="true">⌘</div>
      <div class="source-copy">
        <span class="eyebrow">Skill library</span>
        <h1 id="source-title">Compare your skills</h1>
        <p>Choose a parent folder. Each immediate subfolder is scanned as one skill.</p>
        <code class:empty={!settings.source}>{formatPath(settings.source)}</code>
      </div>
      <button class="button button-primary" type="button" onclick={chooseSource} disabled={controlsDisabled}>
        <span aria-hidden="true">＋</span> Choose folder…
      </button>
    </section>

    {#if settingsError}
      <div class="notice notice-error" role="alert">
        <span class="notice-icon" aria-hidden="true">!</span>
        <span>{settingsError}<small>Settings file: {settingsFile || 'unavailable'}</small></span>
      </div>
    {/if}
    {#if loadError}
      <div class="notice notice-error" role="alert">
        <span class="notice-icon" aria-hidden="true">!</span><span>{loadError}</span>
      </div>
    {/if}

    <section class="workspace" aria-label="Harness comparison">
      <div class="harness-tabs" aria-label="Harness destinations" role="tablist" aria-orientation="horizontal" tabindex="0">
        {#each harnesses as harness (harness.id)}
          <button
            id="tab-{harness.id}"
            class:active={activeHarness === harness.id}
            type="button"
            role="tab"
            aria-selected={activeHarness === harness.id}
            aria-controls="comparison-panel"
            onclick={() => switchHarness(harness.id)}
            onkeydown={(event) => {
              if (event.key === 'ArrowRight' || event.key === 'ArrowLeft') {
                event.preventDefault()
                const direction = event.key === 'ArrowRight' ? 1 : -1
                const next = harnesses[(harnesses.findIndex((item) => item.id === activeHarness) + direction + harnesses.length) % harnesses.length]
                void switchHarness(next.id)
                document.getElementById(`tab-${next.id}`)?.focus()
              }
            }}
            disabled={controlsDisabled}
          >
            <span class="harness-glyph" aria-hidden="true">{harness.id === 'codex' ? '◈' : harness.id === 'claude' ? '✳' : harness.id === 'antigravity' ? '◉' : '⌘'}</span>
            {harness.label}
          </button>
        {/each}
      </div>

      <div class="destination-bar">
        <div class="destination-label"><span class="eyebrow">Managed destination</span><code>{formatPath(scan?.destinationPath ?? settings.destinations[activeHarness])}</code></div>
        <button class="button button-secondary" type="button" onclick={chooseDestination} disabled={controlsDisabled}>Choose destination…</button>
      </div>

      {#if scan?.warnings.length}
        <div class="scan-notices">
          {#each scan.warnings as warning}
            <p class="notice notice-info"><span class="notice-icon" aria-hidden="true">i</span>{warning}</p>
          {/each}
        </div>
      {/if}

      <div id="comparison-panel" role="tabpanel" aria-labelledby="tab-{activeHarness}" class="comparison-panel">
        <div class="list-pane">
          <div class="list-toolbar">
            <div><h2>Skills</h2><span class="result-count">{filteredSkills.length} {filteredSkills.length === 1 ? 'result' : 'results'}</span></div>
            <div class="operation-actions">
              <button class="button button-primary install-button" type="button" onclick={() => prepareAction('install')} disabled={!installSelectedCount || loading || controlsDisabled} aria-label="Review selected skills for installation">Install{installSelectedCount ? ` (${installSelectedCount})` : ''}</button>
              <button class="button button-secondary install-button" type="button" onclick={() => prepareAction('update')} disabled={!updateSelectedCount || loading || controlsDisabled} aria-label="Review selected skills for update">Update{updateSelectedCount ? ` (${updateSelectedCount})` : ''}</button>
              <button class="button button-quiet install-button" type="button" onclick={() => prepareAction('uninstall')} disabled={!uninstallSelectedCount || loading || controlsDisabled} aria-label="Review selected skills for uninstallation">Uninstall{uninstallSelectedCount ? ` (${uninstallSelectedCount})` : ''}</button>
              <button class="button button-quiet" type="button" onclick={refresh} disabled={loading || controlsDisabled} aria-label="Refresh comparison">↻ <span>Refresh</span></button>
            </div>
          </div>
          <div class="filters">
            <label class="search-field"><span aria-hidden="true">⌕</span><span class="sr-only">Search skills</span><input bind:value={search} placeholder="Search skills…" /></label>
            <label class="filter-field"><span class="sr-only">Filter by status</span>
              <select bind:value={statusFilter} aria-label="Filter by status">
                <option value="all">All statuses</option>
                <option value="missing">Missing</option>
                <option value="identical">Identical</option>
                <option value="different">Different</option>
                <option value="installed_only">Installed only</option>
                <option value="invalid_source">Invalid source</option>
                <option value="error">Scan error</option>
                <option value="ambiguous">Ambiguous</option>
              </select>
            </label>
          </div>

          {#if loading && !scan}
            <div class="empty-state"><span class="spinner" aria-hidden="true"></span><p>Scanning folders…</p></div>
          {:else if !settings.source && !(scan?.skills.length)}
            <div class="empty-state"><span class="empty-icon" aria-hidden="true">▧</span><h3>Choose a skill library</h3><p>Select a folder to compare its skills with {labelFor(activeHarness)}.</p></div>
          {:else if filteredSkills.length === 0}
            <div class="empty-state"><span class="empty-icon" aria-hidden="true">⌕</span><h3>No skills found</h3><p>Try another search or check the selected folders.</p></div>
          {:else}
            <div class="skill-list" aria-label="Comparison results">
              {#each filteredSkills as skill (skill.folderName)}
                <div class="skill-row" class:focused={focusedFolder === skill.folderName}>
                  <input type="checkbox" checked={selections[activeHarness].includes(skill.folderName)} onchange={() => toggleSelection(skill)} aria-label="Select {skill.folderName}" disabled={controlsDisabled} />
                  <button class="skill-summary" type="button" onclick={() => focusedFolder = skill.folderName} aria-label="Show details for {skill.folderName}">
                    <span class="skill-title">{skill.name || skill.folderName}</span>
                    <span class="skill-folder">{skill.folderName}</span>
                  </button>
                  <span class="status status-{skill.status}"><i></i>{statusLabel(skill.status)}</span>
                </div>
              {/each}
            </div>
          {/if}
          {#if selectedCount > 0}<p class="selection-note">{selectedCount} selected for review</p>{/if}
        </div>

        <aside class="details-pane" aria-label="Skill details">
          {#if focusedSkill}
            <div class="details-header">
              <div><span class="eyebrow">Skill details</span><h2>{focusedSkill.name || focusedSkill.folderName}</h2></div>
              <span class="status status-{focusedSkill.status}"><i></i>{statusLabel(focusedSkill.status)}</span>
            </div>
            {#if focusedSkill.description}<p class="description">{focusedSkill.description}</p>{/if}
            <dl class="path-list">
              <div><dt>Folder identity</dt><dd><code>{focusedSkill.folderName}</code></dd></div>
              <div><dt>Source path</dt><dd title={focusedSkill.sourcePath ?? ''}><code>{formatPath(focusedSkill.sourcePath)}</code></dd></div>
              <div><dt>Destination path</dt><dd title={focusedSkill.destinationPath ?? ''}><code>{formatPath(focusedSkill.destinationPath)}</code></dd></div>
            </dl>
            {#if focusedSkill.error}<div class="detail-warning error-text"><strong>Scan issue</strong><p>{focusedSkill.error}</p></div>{/if}
            {#if focusedSkill.warnings.length}
              <div class="detail-warning"><strong>Metadata and folder warnings</strong>
                {#each focusedSkill.warnings as warning}<p>{warning}</p>{/each}
              </div>
            {/if}
            {#if focusedSkill.linkWarnings.length}
              <div class="detail-warning link-list"><strong>Links and junctions</strong>
                {#each focusedSkill.linkWarnings as link}<p><code>{link.path}</code><span>→</span><code>{link.target}</code></p>{/each}
              </div>
            {/if}
            {#if focusedSkill.differences.length}
              <div class="differences"><div class="section-heading"><strong>Tree differences</strong><span>{focusedSkill.differences.length}</span></div>
                <ul>{#each focusedSkill.differences as difference}<li><span class="diff-kind diff-{difference.kind}">{difference.kind.replace('_', ' ')}</span><code>{difference.path}</code></li>{/each}</ul>
              </div>
            {:else if focusedSkill.status === 'identical'}
              <div class="identical-note"><span aria-hidden="true">✓</span> Complete folder trees match</div>
            {/if}
          {:else}
            <div class="details-empty"><span class="empty-icon" aria-hidden="true">⌑</span><h3>Skill details</h3><p>Select a skill row to see its paths, warnings, and file-level differences.</p></div>
          {/if}
        </aside>
      </div>
    </section>
    <footer><span>Local skill folder manager</span><span>Files and scripts stay on this device</span></footer>
  </main>

  <dialog bind:this={installDialog} class="operation-dialog" aria-labelledby="install-title" oncancel={(event) => { if (operationRunning) event.preventDefault() }} onclose={() => { if (!operationRunning) preparedInstall = null }}>
    {#if preparedInstall}
      <div class="dialog-heading"><span class="eyebrow">Confirm {preparedInstall.action}</span><h2 id="install-title">{preparedInstall.action === 'install' ? 'Install complete skill folders?' : preparedInstall.action === 'update' ? 'Replace complete skill folders?' : 'Move skill folders to the Recycle Bin?'}</h2>
        <p>{preparedInstall.action === 'install' ? 'Selected folders will be copied to this destination:' : preparedInstall.action === 'update' ? 'The existing complete folders will move to the Recycle Bin before replacements are placed:' : 'Selected complete folders will move to the Windows Recycle Bin from:'}</p><code>{preparedInstall.destinationPath}</code>
      </div>
      {#if preparedInstall.action !== 'install'}
        <div class="dialog-warning"><strong>Recovery and refresh</strong><p>Windows controls Recycle Bin retention. Restore items manually from Recycle Bin; external harnesses may need a refresh or restart.</p></div>
      {/if}
      {#if preparedInstall.warnings.length}
        <div class="dialog-warning"><strong>Review these warnings</strong>
          {#each preparedInstall.warnings as warning}<p>{warning}</p>{/each}
        </div>
      {/if}
      <ul class="install-list">
        {#each preparedInstall.eligible as skill (skill.folderName)}
          <li><strong>{skill.folderName}</strong>
            {#each skill.warnings as warning}<small>{warning}</small>{/each}
            {#each skill.linkWarnings as link}<small><code>{link.path}</code> points to <code>{link.target}</code></small>{/each}
          </li>
        {/each}
      </ul>
      {#if preparedInstall.skipped.length}
        <div class="dialog-skipped"><strong>Skipped selections</strong>{#each preparedInstall.skipped as item}<p>{item}</p>{/each}</div>
      {/if}
      {#if preparedInstall.eligible.some((skill) => skill.linkWarnings.length)}
        <label class="link-acknowledgement"><input type="checkbox" bind:checked={acknowledgeLinks} disabled={operationRunning} /> I reviewed the listed links and junctions. Copies materialize linked content; recycling removes link entries while preserving their targets.</label>
      {/if}
      {#if operationEvents.length}
        <div class="operation-log" role="log" aria-live="polite">
          {#each operationEvents as event, index (`${index}-${event.kind}`)}
            <p class:failed={event.success === false}>{event.folderName ? `${event.folderName}: ` : ''}{event.message}</p>
          {/each}
        </div>
      {/if}
      {#if operationError}<p class="dialog-error" role="alert">{operationError}</p>{/if}
      <div class="dialog-actions">
        {#if operationRunning}
          {@const current = [...operationEvents].reverse().find((event) => event.kind === 'progress')}
          <span class="operation-progress" aria-live="polite">{current?.folderName ? `${preparedInstall.action} ${current.folderName}` : preparedInstall.action} ({current?.completed ?? 0}/{current?.total ?? preparedInstall.eligible.length})</span>
        {:else if operationEvents.some((event) => event.kind === 'finished')}
          <button class="button button-primary" type="button" onclick={closeInstallDialog}>Done</button>
        {:else if operationError}
          <button class="button button-secondary" type="button" onclick={closeInstallDialog}>Close and refresh</button>
        {:else}
          <button class="button button-secondary" type="button" onclick={closeInstallDialog}>Cancel</button>
          <button class="button button-primary" type="button" onclick={runOperation} disabled={preparedInstall.eligible.some((skill) => skill.linkWarnings.length) && !acknowledgeLinks}>Confirm {preparedInstall.action}</button>
        {/if}
      </div>
    {/if}
  </dialog>
</div>
