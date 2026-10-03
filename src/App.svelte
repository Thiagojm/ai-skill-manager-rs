<script lang="ts">
  import { onMount, tick } from 'svelte'
  import appIcon from '../src-tauri/icons/128x128.png'
  import { open } from '@tauri-apps/plugin-dialog'
  import {
    loadSettings,
    saveSettings,
    openHarnessFolder,
    scanSkills,
    prepareOperation,
    executeOperation,
    type OperationAction,
    type Harness,
    type HarnessDescriptor,
    type OperationEvent,
    type PrepareResponse,
    type ScanResponse,
    type ScanProgress,
    type Settings,
    type SkillRow,
  } from './api'

  let harnesses: HarnessDescriptor[] = []
  let settings: Settings = { source: null, destinations: {}, custom_harnesses: {}, theme: 'dark' }
  let settingsFile = ''
  let settingsError = ''
  let loadError = ''
  let stale = false
  let scan: ScanResponse | null = null
  let activeHarness: Harness | null = null
  let search = ''
  let statusFilter = 'all'
  let focusedFolder = ''
  let loading = false
  let saving = false
  let operationPreparing = false
  let operationRunning = false
  let operationError = ''
  let operationEvents: OperationEvent[] = []
  let latestOperationProgress: OperationEvent | null = null
  let operationStarted = false
  let operationSequence = 0
  let scanProgress: ScanProgress | null = null
  let copyStatus = ''
  let copySequence = 0
  const stageLabels = { discover_source: 'Discovering source skills', source: 'Reading source skills', discover_destination: 'Discovering destination skills', destination: 'Reading destination skills', compare: 'Comparing skills' }
  const differenceLabels = { added: 'Will be added to destination', removed: 'Will be removed from destination', changed: 'Content or permissions changed', type_changed: 'Entry type changed' }
  $: operationSucceeded = operationEvents.filter((event) => event.kind === 'result' && event.success === true).length
  $: operationFailed = operationEvents.filter((event) => event.kind === 'result' && event.success === false).length

  async function copyPath(path: string | null | undefined, label: string) {
    if (!path) return
    const sequence = ++copySequence
    copyStatus = ''
    try {
      await navigator.clipboard.writeText(path)
      if (sequence === copySequence) copyStatus = `${label} copied.`
    } catch {
      if (sequence === copySequence) copyStatus = `Could not copy ${label.toLowerCase()}. Clipboard access is unavailable.`
    }
  }

  function acceptScanProgress(sequence: number, event: ScanProgress) {
    if (sequence === scanSequence && loading) scanProgress = event
  }
  let preparedInstall: PrepareResponse | null = null
  let acknowledgeLinks = false
  let installDialog: HTMLDialogElement
  let addDialog: HTMLDialogElement
  let manageDialog: HTMLDialogElement
  let pathDialog: HTMLDialogElement
  let pathKind: 'source' | 'destination' = 'source'
  let pathValue = ''
  let addName = ''
  let addDestination = ''
  let renameValue = ''
  let confirmRemoving = false
  let scanSequence = 0
  let wideNavigation = window.matchMedia('(min-width: 1101px)').matches
  let harnessNavigation: HTMLElement
  let searchInput: HTMLInputElement
  const statuses = ['all', 'missing', 'different', 'identical', 'installed_only', 'invalid_source', 'ambiguous', 'error']
  $: statusCounts = countStatuses(scan?.skills ?? [])
  $: visibleNames = new Set(filteredSkills.map((skill) => skill.folderName))
  $: hiddenSelectedCount = activeSelections.filter((name) => !visibleNames.has(name)).length
  $: allFilteredSelected = filteredSkills.every((skill) => selectedNames.has(skill.folderName))

  function countStatuses(rows: SkillRow[]) {
    const counts: Record<string, number> = { all: rows.length }
    for (const row of rows) counts[row.status] = (counts[row.status] ?? 0) + 1
    return counts
  }

  function changeSelection(mode: 'filtered' | 'all' | 'hidden') {
    if (!activeHarness || controlsDisabled || stale) return
    selections[activeHarness] = mode === 'all' ? [] : mode === 'hidden'
      ? activeSelections.filter((name) => visibleNames.has(name))
      : [...new Set([...activeSelections, ...visibleNames])]
  }

  onMount(() => {
    const media = window.matchMedia('(min-width: 1101px)')
    const change = async () => {
      const transferFocus = harnessNavigation?.contains(document.activeElement)
      wideNavigation = media.matches
      await tick()
      if (transferFocus) harnessNavigation?.querySelector<HTMLElement>('[aria-current="true"], select')?.focus()
    }
    const shortcut = (event: KeyboardEvent) => {
      if (!event.ctrlKey || event.key.toLowerCase() !== 'f' || !searchInput?.isConnected || document.querySelector('dialog[open]')) return
      const target = event.target
      if (target instanceof HTMLElement && target !== searchInput && target.closest('input, textarea, select, [contenteditable="true"]')) return
      event.preventDefault()
      searchInput?.focus()
      searchInput?.select()
    }
    media.addEventListener('change', change)
    window.addEventListener('keydown', shortcut)
    return () => {
      media.removeEventListener('change', change)
      window.removeEventListener('keydown', shortcut)
    }
  })

  let choosing = false
  let selections: Record<Harness, string[]> = {}

  $: rowsByFolder = new Map((scan?.skills ?? []).map((skill) => [skill.folderName, skill]))
  $: query = search.trim().toLocaleLowerCase()
  $: filteredSkills = (scan?.skills ?? []).filter((skill) => {
    const matchesSearch = !query || skill.folderName.toLocaleLowerCase().includes(query)
      || skill.name?.toLocaleLowerCase().includes(query)
    return matchesSearch && (statusFilter === 'all' || skill.status === statusFilter)
  })
  $: focusedSkill = rowsByFolder.get(focusedFolder) ?? null
  $: activeSelections = activeHarness ? selections[activeHarness] ?? [] : []
  $: activeDescriptor = harnesses.find((item) => item.id === activeHarness)
  $: selectedCount = activeSelections.length
  $: selectedNames = new Set(activeSelections)
  $: actionCounts = countActions(activeSelections, rowsByFolder)
  $: installSelectedCount = actionCounts.install
  $: updateSelectedCount = actionCounts.update
  $: uninstallSelectedCount = actionCounts.uninstall

  function countActions(names: string[], rows: Map<string, SkillRow>) {
    const counts = { install: 0, update: 0, uninstall: 0 }
    for (const name of names) {
      for (const action of rows.get(name)?.eligibleActions ?? []) counts[action]++
    }
    return counts
  }
  $: controlsDisabled = loading || choosing || saving || operationPreparing || operationRunning
  $: document.documentElement.dataset.theme = settings.theme

  onMount(async () => {
    try {
      await refresh(false, true)
    } catch (error) {
      loadError = String(error)
    }
  })

  async function refresh(reuseSource = false, reloadRegistry = false) {
    if (loading || operationRunning || operationPreparing) return
    const sequence = ++scanSequence
    loading = true
    scanProgress = null
    loadError = ''
    try {
      const previousHarness = activeHarness
      if (reloadRegistry) applySettingsResponse(await loadSettings())
      const harness = activeHarness
      if (!harness) {
        scan = null
        stale = false
        focusedFolder = ''
        selections = {}
        return
      }
      const response = await scanSkills(harness, previousHarness === harness && reuseSource, (event) => acceptScanProgress(sequence, event))
      if (sequence !== scanSequence) return
      scan = response
      stale = false
      selections[harness] = selectionsFor(harness).filter((name) => response.skills.some((skill) => skill.folderName === name))
      if (!response.skills.some((skill) => skill.folderName === focusedFolder)) {
        focusedFolder = response.skills[0]?.folderName ?? ''
      }
    } catch (error) {
      if (sequence === scanSequence) {
        scan = null
        stale = false
        loadError = String(error)
      }
    } finally {
      if (sequence === scanSequence) { loading = false; scanProgress = null }
    }
  }

  function applySettingsResponse(response: Awaited<ReturnType<typeof loadSettings>>, preferred: Harness | null = activeHarness) {
    settings = response.settings
    settingsFile = response.settingsFile
    settingsError = response.error ?? ''
    harnesses = response.harnesses.filter((harness) => harness.visible)
    for (const descriptor of response.harnesses) {
      if (!descriptor.visible) selections[descriptor.id] = []
    }
    const previous = activeHarness
    activeHarness = preferred && harnesses.some((harness) => harness.id === preferred)
      ? preferred
      : harnesses.some((harness) => harness.id === previous) ? previous : harnesses[0]?.id ?? null
    if (!activeHarness) {
      scan = null
      stale = false
      focusedFolder = ''
      selections = {}
    } else if (previous !== activeHarness) {
      scan = null
      stale = false
      focusedFolder = ''
      if (previous && !harnesses.some((harness) => harness.id === previous)) selections[previous] = []
    }
  }

  function selectionsFor(harness: Harness) { return selections[harness] ?? [] }

  function enterPath(kind: 'source' | 'destination') {
    if (controlsDisabled || (kind === 'destination' && !activeHarness)) return
    pathKind = kind
    pathValue = (kind === 'source' ? settings.source : settings.destinations[activeHarness!]) ?? ''
    loadError = ''
    pathDialog.showModal()
  }

  async function savePath(event: SubmitEvent) {
    event.preventDefault()
    if (controlsDisabled || !pathValue.trim()) return
    const path = pathValue.trim()
    const next = pathKind === 'source'
      ? { ...settings, source: path }
      : { ...settings, destinations: { ...settings.destinations, [activeHarness!]: path } }
    if (await persist(next, pathKind)) pathDialog.close()
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
    if (!activeHarness || controlsDisabled) return
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

  async function openDestination() {
    if (!activeHarness || controlsDisabled) return
    loadError = ''
    try {
      await openHarnessFolder(activeHarness)
    } catch (error) {
      loadError = String(error)
    }
  }

  async function persist(next: Settings, changed: 'source' | 'destination') {
    if (saving || operationPreparing || operationRunning) return
    saving = true
    loadError = ''
    try {
      await saveSettings(next)
      const previousHarness = activeHarness
      applySettingsResponse(await loadSettings())
      settingsError = ''
      if (changed === 'source') selections = {}
      else if (changed === 'destination' && previousHarness) selections[previousHarness] = []
      focusedFolder = ''
      await refresh()
      return true
    } catch (error) {
      loadError = String(error)
      return false
    } finally {
      saving = false
    }
  }

  function openAddDialog() {
    if (controlsDisabled) return
    loadError = ''
    addName = ''
    addDestination = ''
    addDialog.showModal()
  }

  async function chooseCustomFolder() {
    if (controlsDisabled) return
    choosing = true
    try {
      const selected = await open({ directory: true, multiple: false, title: 'Choose custom harness skills folder' })
      if (typeof selected === 'string') addDestination = selected
    } catch (error) { loadError = String(error) }
    finally { choosing = false }
  }

  async function addHarness() {
    const name = addName.trim()
    if (!name || !addDestination || controlsDisabled) return
    const id = `custom-${crypto.randomUUID()}`
    await saveRegistry({
      ...settings,
      destinations: { ...settings.destinations, [id]: addDestination },
      custom_harnesses: { ...settings.custom_harnesses, [id]: name },
    }, id, addDialog)
  }

  function openManageDialog() {
    if (!activeDescriptor || activeDescriptor.builtIn || controlsDisabled) return
    loadError = ''
    renameValue = activeDescriptor.label
    confirmRemoving = false
    manageDialog.showModal()
  }

  async function renameHarness() {
    if (!activeHarness || !renameValue.trim() || controlsDisabled) return
    await saveRegistry({ ...settings, custom_harnesses: { ...settings.custom_harnesses, [activeHarness]: renameValue.trim() } }, activeHarness, manageDialog)
  }

  async function removeHarness() {
    if (!activeHarness || !confirmRemoving || controlsDisabled) return
    const removed = activeHarness
    const destinations = { ...settings.destinations }
    const custom_harnesses = { ...settings.custom_harnesses }
    delete destinations[removed]
    delete custom_harnesses[removed]
    await saveRegistry({ ...settings, destinations, custom_harnesses }, null, manageDialog, removed)
  }

  async function saveRegistry(next: Settings, preferred: Harness | null, dialog: HTMLDialogElement, removed: Harness | null = null) {
    if (saving || operationPreparing || operationRunning) return
    saving = true
    loadError = ''
    try {
      await saveSettings(next)
      const response = await loadSettings()
      if (response.error) throw new Error(response.error)
      applySettingsResponse(response, preferred)
      if (removed) selections[removed] = []
      dialog.close()
      confirmRemoving = false
      focusedFolder = ''
      search = ''
      statusFilter = 'all'
      await refresh()
    } catch (error) { loadError = String(error) }
    finally { saving = false }
  }

  async function switchHarness(harness: Harness) {
    if (harness === activeHarness || controlsDisabled || !harnesses.some((item) => item.id === harness)) return
    activeHarness = harness
    search = ''
    statusFilter = 'all'
    focusedFolder = ''
    scan = null
    stale = false
    await refresh(true)
  }

  function toggleSelection(skill: SkillRow) {
    if (!activeHarness || controlsDisabled || stale) return
    const selected = new Set(selectionsFor(activeHarness))
    if (selected.has(skill.folderName)) selected.delete(skill.folderName)
    else selected.add(skill.folderName)
    selections[activeHarness] = [...selected]
  }

  async function toggleTheme() {
    if (controlsDisabled) return
    saving = true
    loadError = ''
    const next: Settings = { ...settings, theme: settings.theme === 'dark' ? 'light' : 'dark' }
    try {
      await saveSettings(next)
      settings = next
    } catch (error) {
      loadError = String(error)
    } finally {
      saving = false
    }
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
    if (!activeHarness || !scan || !eligibleCount || stale || loading || controlsDisabled) return
    operationPreparing = true
    operationError = ''
    try {
      preparedInstall = await prepareOperation(action, activeHarness, scan.revision, selectionsFor(activeHarness))
      acknowledgeLinks = false
      operationSequence++
      operationEvents = []
      latestOperationProgress = null
      operationStarted = false
      installDialog.showModal()
    } catch (error) {
      loadError = String(error)
    } finally {
      operationPreparing = false
    }
  }

  async function runOperation() {
    if (!activeHarness || !preparedInstall || operationRunning) return
    if (preparedInstall.eligible.some((skill) => skill.linkWarnings.length) && !acknowledgeLinks) return
    operationRunning = true
    operationError = ''
    operationEvents = []
    latestOperationProgress = null
    operationStarted = true
    const sequence = ++operationSequence
    try {
      await executeOperation(preparedInstall.token, acknowledgeLinks, (event) => {
        if (sequence !== operationSequence) return
        operationEvents = [...operationEvents, event]
        if (event.kind === 'progress') latestOperationProgress = event
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
    if (!activeHarness) {
      scan = null
      stale = false
      return
    }
    const sequence = ++scanSequence
    loading = true
    scanProgress = null
    try {
      const response = await scanSkills(activeHarness, false, (event) => acceptScanProgress(sequence, event))
      if (sequence === scanSequence) {
        scan = response
        stale = false
        focusedFolder = response.skills.some((skill) => skill.folderName === focusedFolder)
          ? focusedFolder
          : response.skills[0]?.folderName ?? ''
      }
    } catch (error) {
      if (sequence === scanSequence) {
        stale = scan !== null
        loadError = String(error)
      }
    } finally {
      if (sequence === scanSequence) { loading = false; scanProgress = null }
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
      <img class="brand-mark" src={appIcon} alt="" width="34" height="34" />
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
      <div class="source-copy">
        <h1 id="source-title">Skill library</h1>
        {#if !settings.source}<p>Choose a parent folder containing skill folders.</p>{/if}
        <details class="path-disclosure"><summary title={settings.source ?? ''}>{settings.source || 'Choose a folder containing skills'}</summary><code>{formatPath(settings.source)}</code></details>
      </div>
      <div class="destination-actions"><button class="button button-secondary" type="button" onclick={() => copyPath(settings.source, 'Source path')} disabled={!settings.source}>Copy source path</button><button class="button button-primary" type="button" onclick={chooseSource} disabled={controlsDisabled}>
        <span aria-hidden="true">＋</span> Choose folder…
      </button><button class="button button-secondary" type="button" onclick={() => enterPath('source')} disabled={controlsDisabled}>Enter path…</button></div>
    </section>

    {#if settingsError || stale || loadError}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable notices need keyboard focus.) -->
    <div class="notice-region" role="region" aria-label="Application notices" tabindex="0">
    {#if settingsError}
      <div class="notice notice-error" role="alert">
        <span class="notice-icon" aria-hidden="true">!</span>
        <span>{settingsError}<small>Settings file: {settingsFile || 'unavailable'}</small></span>
      </div>
    {/if}
    {#if stale}
      <div class="notice notice-error" role="alert">This comparison is outdated because the post-operation refresh failed. Refresh successfully before selecting skills or running another operation.</div>
    {/if}
    {#if loadError}
      <div class="notice notice-error" role="alert">
        <span class="notice-icon" aria-hidden="true">!</span><span>{loadError}</span>
      </div>
    {/if}

    </div>
    {/if}
    <p class="copy-status" role="status">{copyStatus}</p>
    {#if loading}
      <div class="scan-progress" role="status" aria-live="polite">
        {#if !scanProgress || scanProgress.total === null}<span class="spinner" aria-hidden="true"></span>{/if}
        <span>{scanProgress ? stageLabels[scanProgress.stage] : 'Starting scan'}{scanProgress?.total !== null && scanProgress ? ` (${scanProgress.completed}/${scanProgress.total})` : '…'}{scanProgress?.reusedSource ? ' — cached source reused' : ''}</span>
      </div>
    {/if}
    <section class="workspace" aria-label="Harness comparison">
      {#if !activeHarness}
        <div class="no-harness-state">
          <span class="empty-icon" aria-hidden="true">▧</span>
          <h2>No configured harnesses found</h2>
          <p>No built-in harness configuration was detected. Add a custom skills folder or refresh.</p>
          <div class="destination-actions"><button class="button button-primary" type="button" onclick={openAddDialog} disabled={controlsDisabled}>＋ Add harness</button><button class="button button-secondary" type="button" onclick={() => refresh(false, true)} disabled={controlsDisabled}>Refresh</button></div>
        </div>
      {:else}
      <nav class="harness-navigation" bind:this={harnessNavigation} aria-label="Harness destinations">
        {#if wideNavigation}
          <span class="eyebrow">Harnesses</span>
          <div class="harness-list">
            {#each harnesses as harness (harness.id)}
              <button type="button" aria-current={activeHarness === harness.id ? 'true' : undefined} onclick={() => switchHarness(harness.id)} disabled={controlsDisabled}>{harness.label}</button>
            {/each}
          </div>
        {:else}
          <label class="compact-harness">Harness<select value={activeHarness} onchange={(event) => switchHarness(event.currentTarget.value)} disabled={controlsDisabled}>
            {#each harnesses as harness (harness.id)}<option value={harness.id}>{harness.label}</option>{/each}
          </select></label>
        {/if}
        <button class="button button-secondary" type="button" onclick={openAddDialog} disabled={controlsDisabled}>＋ Add harness</button>
      </nav>
      <div class="workspace-content">
      <div class="destination-bar">
        <div class="destination-label"><span class="eyebrow">Managed destination</span><details class="path-disclosure"><summary title={scan?.destinationPath ?? settings.destinations[activeHarness]}>{formatPath(scan?.destinationPath ?? settings.destinations[activeHarness])}</summary><code>{formatPath(scan?.destinationPath ?? settings.destinations[activeHarness])}</code></details></div>
        <div class="destination-actions">
          <button class="button button-secondary" type="button" onclick={() => copyPath(scan?.destinationPath ?? settings.destinations[activeHarness!], 'Destination path')} disabled={!(scan?.destinationPath ?? settings.destinations[activeHarness!])}>Copy destination path</button>
          <button class="button button-secondary" type="button" onclick={chooseDestination} disabled={controlsDisabled}>Choose destination…</button>
          <button class="button button-secondary" type="button" onclick={() => enterPath('destination')} disabled={controlsDisabled}>Enter path…</button>
          {#if !activeDescriptor?.builtIn}<button class="button button-secondary" type="button" onclick={openManageDialog} disabled={controlsDisabled}>Manage harness</button>{/if}
          <button class="button button-secondary" type="button" onclick={openDestination} disabled={controlsDisabled}>Open folder</button>
        </div>
      </div>

      {#if (activeDescriptor && !activeDescriptor.destinationAvailable) || scan?.warnings.length}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable notices need keyboard focus.) -->
      <div class="notice-region" role="region" aria-label="Comparison notices" tabindex="0">
      {#if activeDescriptor && !activeDescriptor.destinationAvailable}
        <p class="notice notice-error" role="alert"><span class="notice-icon" aria-hidden="true">!</span>{activeDescriptor.builtIn ? 'This configured destination is currently unavailable. Choose another folder or install a selected skill to create it.' : 'This custom harness destination is unavailable. Choose an existing skills folder or remove this registration.'}</p>
      {/if}
      {#if scan?.warnings.length}
        <div class="scan-notices">
          {#each scan.warnings.filter((warning) => activeDescriptor?.destinationAvailable !== false || warning !== 'The configured destination folder does not exist yet.') as warning}
            <p class="notice notice-info"><span class="notice-icon" aria-hidden="true">i</span>{warning}</p>
          {/each}
        </div>
      {/if}

      </div>
      {/if}
      <div id="comparison-panel" class="comparison-panel" aria-label="Skills comparison">
        <div class="list-pane">
          <div class="list-toolbar">
            <h2 class="sr-only">Skills</h2>
            <div class="operation-actions">
              <button class="button button-primary install-button" type="button" onclick={() => prepareAction('install')} disabled={!installSelectedCount || stale || loading || controlsDisabled} aria-label="Review selected skills for installation">Install{installSelectedCount ? ` (${installSelectedCount})` : ''}</button>
              <button class="button button-secondary install-button" type="button" onclick={() => prepareAction('update')} disabled={!updateSelectedCount || stale || loading || controlsDisabled} aria-label="Review selected skills for update">Update{updateSelectedCount ? ` (${updateSelectedCount})` : ''}</button>
              <button class="button button-quiet install-button" type="button" onclick={() => prepareAction('uninstall')} disabled={!uninstallSelectedCount || stale || loading || controlsDisabled} aria-label="Review selected skills for uninstallation">Uninstall{uninstallSelectedCount ? ` (${uninstallSelectedCount})` : ''}</button>
              <button class="button button-quiet" type="button" onclick={() => refresh(false, true)} disabled={loading || controlsDisabled} aria-label="Refresh comparison">↻ <span>Refresh</span></button>
            </div>
          </div>
          <div class="filters">
            <label class="search-field"><span aria-hidden="true">⌕</span><span class="sr-only">Search skills</span><input bind:this={searchInput} bind:value={search} placeholder="Search skills…" /></label><span class="result-count">{filteredSkills.length} {filteredSkills.length === 1 ? 'result' : 'results'}</span>
          </div>
          <div class="status-filters" aria-label="Filter by status">
            {#each statuses as status}
              <button type="button" aria-pressed={statusFilter === status} onclick={() => statusFilter = status}>{status === 'all' ? 'All' : statusLabel(status)} <span>{statusCounts[status] ?? 0}</span></button>
            {/each}
          </div>
          <div class="selection-controls">
            <button class="button button-quiet" type="button" onclick={() => changeSelection('filtered')} disabled={controlsDisabled || stale || !filteredSkills.length || allFilteredSelected}>Select filtered</button>
            <button class="button button-quiet" type="button" onclick={() => changeSelection('all')} disabled={controlsDisabled || stale || !selectedCount}>Clear selection</button>
            <button class="button button-quiet" type="button" onclick={() => changeSelection('hidden')} disabled={controlsDisabled || stale || !hiddenSelectedCount}>Clear hidden selections</button>
          </div>
          {#if selectedCount > 0}<p class="selection-note">{selectedCount} selected for review · {hiddenSelectedCount} hidden by filters</p>{/if}

          {#if loading && !scan}
            <div class="empty-state"><p>Scanning folders…</p></div>
          {:else if !settings.source && !(scan?.skills.length)}
            <div class="empty-state"><span class="empty-icon" aria-hidden="true">▧</span><h3>Choose a skill library</h3><p>Select a folder to compare its skills with {labelFor(activeHarness)}.</p></div>
          {:else if filteredSkills.length === 0}
            <div class="empty-state"><span class="empty-icon" aria-hidden="true">⌕</span><h3>No skills found</h3><p>Try another search or check the selected folders.</p></div>
          {:else}
            <div class="skill-list" aria-label="Comparison results">
              {#each filteredSkills as skill (skill.folderName)}
                <div class="skill-row" class:focused={focusedFolder === skill.folderName}>
                  <input type="checkbox" checked={selectedNames.has(skill.folderName)} onchange={() => toggleSelection(skill)} aria-label="Select {skill.folderName}" disabled={controlsDisabled || stale} />
                  <button class="skill-summary" type="button" onclick={() => focusedFolder = skill.folderName} aria-label="Show details for {skill.folderName}">
                    <span class="skill-title">{skill.name || skill.folderName}</span>
                    <span class="skill-folder">{skill.folderName}</span>
                  </button>
                  <span class="status status-{skill.status}"><i></i>{statusLabel(skill.status)}</span>
                </div>
              {/each}
            </div>
          {/if}
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
                <p>Copies replace links with ordinary files and folders. A linked source can remain Different after Install or Update because entry types differ, even when content matches.</p>
                {#each focusedSkill.linkWarnings as link}<p><code>{link.path}</code><span>→</span><code>{link.target}</code></p>{/each}
              </div>
            {/if}
            {#if focusedSkill.differences.length}
              <div class="differences"><div class="section-heading"><strong>Update destination differences</strong><span>{focusedSkill.differences.length}</span></div>
                <p>Update replaces the complete destination folder, including removal of destination-only entries. This does not preview Uninstall. Copies materialize links, so linked sources can remain Different.</p>
                <ul>{#each focusedSkill.differences as difference}<li><span class="diff-kind diff-{difference.kind}">{differenceLabels[difference.kind]}</span><code>{difference.path}</code><button class="button button-secondary copy-difference" type="button" aria-label={`Copy difference path ${difference.path}`} onclick={() => copyPath(difference.path, 'Difference path')}>Copy</button></li>{/each}</ul>
              </div>
            {:else if focusedSkill.status === 'identical'}
              <div class="identical-note"><span aria-hidden="true">✓</span> Complete folder trees match</div>
            {/if}
          {:else}
            <div class="details-empty"><span class="empty-icon" aria-hidden="true">⌑</span><h3>Skill details</h3><p>Select a skill row to see its paths, warnings, and file-level differences.</p></div>
          {/if}
        </aside>
      </div>
      </div>
      {/if}
    </section>
    <footer><span>Local skill folder manager</span><span>Files and scripts stay on this device</span></footer>
  </main>

  <dialog bind:this={pathDialog} class="operation-dialog management-dialog" aria-labelledby="path-title" oncancel={(event) => { if (saving) event.preventDefault() }}>
    <form onsubmit={savePath}>
      <div class="dialog-heading"><h2 id="path-title">Enter {pathKind} folder path</h2><p>Use an absolute path. Paths are literal; ~ and environment variables are not expanded.</p></div>
      <label class="management-field">Folder path<input bind:value={pathValue} autocomplete="off" required disabled={controlsDisabled} /></label>
      {#if loadError}<p class="dialog-error" role="alert">{loadError}</p>{/if}
      <div class="dialog-actions"><button class="button button-secondary" type="button" onclick={() => pathDialog.close()} disabled={controlsDisabled}>Cancel</button><button class="button button-primary" type="submit" disabled={controlsDisabled || !pathValue.trim()}>Use folder</button></div>
    </form>
  </dialog>

  <dialog bind:this={addDialog} class="operation-dialog management-dialog" aria-labelledby="add-harness-title" oncancel={(event) => { if (saving || choosing) event.preventDefault() }}>
    <div class="dialog-heading"><span class="eyebrow">Custom harness</span><h2 id="add-harness-title">Add harness</h2><p>Register an existing skills folder. The folder will stay where it is.</p></div>
    <label class="management-field">Name<input bind:value={addName} autocomplete="off" disabled={controlsDisabled} /></label>
    <label class="management-field">Skills folder<input bind:value={addDestination} placeholder="Absolute path to an existing folder" autocomplete="off" disabled={controlsDisabled} /></label>
    <button class="button button-secondary" type="button" onclick={chooseCustomFolder} disabled={controlsDisabled}>Choose folder…</button>
    {#if loadError}<p class="dialog-error" role="alert">{loadError}</p>{/if}
    <div class="dialog-actions"><button class="button button-secondary" type="button" onclick={() => addDialog.close()} disabled={controlsDisabled}>Cancel</button><button class="button button-primary" type="button" onclick={addHarness} disabled={controlsDisabled || !addName.trim() || !addDestination}>Add harness</button></div>
  </dialog>

  <dialog bind:this={manageDialog} class="operation-dialog management-dialog" aria-labelledby="manage-harness-title" oncancel={(event) => { if (saving) event.preventDefault() }} onclose={() => { confirmRemoving = false }}>
    <div class="dialog-heading"><span class="eyebrow">Custom harness</span><h2 id="manage-harness-title">Manage {activeDescriptor?.label}</h2></div>
    {#if confirmRemoving}
      <div class="dialog-warning"><strong>Remove this registration?</strong><p>Only the saved registration and destination mapping will be removed. The skills folder and every file in it will remain untouched.</p></div>
      {#if loadError}<p class="dialog-error" role="alert">{loadError}</p>{/if}
      <div class="dialog-actions"><button class="button button-secondary" type="button" onclick={() => confirmRemoving = false} disabled={controlsDisabled}>Keep harness</button><button class="button button-danger" type="button" onclick={removeHarness} disabled={controlsDisabled}>Remove registration</button></div>
    {:else}
      <label class="management-field">Name<input bind:value={renameValue} autocomplete="off" disabled={controlsDisabled} /></label>
      {#if loadError}<p class="dialog-error" role="alert">{loadError}</p>{/if}
      <div class="dialog-actions"><button class="button button-quiet" type="button" onclick={() => confirmRemoving = true} disabled={controlsDisabled}>Remove harness…</button><button class="button button-secondary" type="button" onclick={() => manageDialog.close()} disabled={controlsDisabled}>Close</button><button class="button button-primary" type="button" onclick={renameHarness} disabled={controlsDisabled || !renameValue.trim()}>Save name</button></div>
    {/if}
  </dialog>

  <dialog bind:this={installDialog} class="operation-dialog" aria-labelledby="install-title" oncancel={(event) => { if (operationRunning) event.preventDefault() }} onclose={() => { if (!operationRunning) preparedInstall = null }}>
    {#if preparedInstall}
      <div class="dialog-heading"><span class="eyebrow">Confirm {preparedInstall.action}</span><h2 id="install-title">{preparedInstall.action === 'install' ? 'Install complete skill folders?' : preparedInstall.action === 'update' ? 'Replace complete skill folders?' : 'Move skill folders to the system trash?'}</h2>
        <p>{preparedInstall.action === 'install' ? 'Selected folders will be copied to this destination:' : preparedInstall.action === 'update' ? 'The existing complete folders will move to the system trash before replacements are placed:' : 'Selected complete folders will move to the system trash from:'}</p><code>{preparedInstall.destinationPath}</code>
      </div>
      {#if preparedInstall.action !== 'install'}
        <div class="dialog-warning"><strong>Recovery and refresh</strong><p>The operating system controls trash retention. Restore items manually from the system trash; external harnesses may need a refresh or restart.</p></div>
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
      {#if operationStarted}
        <p class="operation-summary" role="status">{operationSucceeded} successful · {operationFailed} failed · {preparedInstall.skipped.length} skipped during preparation{operationRunning ? ' · Running' : ''}</p>
      {/if}
      {#if operationError}<p class="dialog-error" role="alert">{operationError}</p>{/if}
      <div class="dialog-actions">
        {#if operationRunning}
          {@const current = latestOperationProgress}
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
