<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import TopBar from './components/TopBar.svelte'
  import SourceToolbar from './components/SourceToolbar.svelte'
  import NoticeRegion from './components/NoticeRegion.svelte'
  import HarnessNavigation from './components/HarnessNavigation.svelte'
  import DestinationBar from './components/DestinationBar.svelte'
  import SkillList from './components/SkillList.svelte'
  import DetailsPane from './components/DetailsPane.svelte'
  import PathDialog from './components/PathDialog.svelte'
  import AddHarnessDialog from './components/AddHarnessDialog.svelte'
  import ManageHarnessDialog from './components/ManageHarnessDialog.svelte'
  import OperationDialog from './components/OperationDialog.svelte'
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
  $: destinationPath = activeHarness ? scan?.destinationPath ?? settings.destinations[activeHarness] : undefined
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
  <TopBar theme={settings.theme} disabled={controlsDisabled} onToggleTheme={toggleTheme} />

  <main id="main">
    <SourceToolbar source={settings.source} disabled={controlsDisabled} onCopy={() => copyPath(settings.source, 'Source path')} onChoose={chooseSource} onEnterPath={() => enterPath('source')} />

    <NoticeRegion {settingsError} {settingsFile} {stale} {loadError} />
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
      <HarnessNavigation {harnesses} {activeHarness} wide={wideNavigation} disabled={controlsDisabled} bind:element={harnessNavigation} onSwitch={switchHarness} onAdd={openAddDialog} />
      <div class="workspace-content">
      <DestinationBar path={destinationPath} descriptor={activeDescriptor} warnings={scan?.warnings ?? []} disabled={controlsDisabled} onCopy={() => copyPath(destinationPath, 'Destination path')} onChoose={chooseDestination} onEnterPath={() => enterPath('destination')} onManage={openManageDialog} onOpen={openDestination} />
      <div id="comparison-panel" class="comparison-panel" aria-label="Skills comparison">
        <SkillList {filteredSkills} {statuses} {statusCounts} bind:search bind:statusFilter bind:focusedFolder bind:searchInput {selectedNames} {selectedCount} {hiddenSelectedCount} {allFilteredSelected} {installSelectedCount} {updateSelectedCount} {uninstallSelectedCount} {stale} {loading} {controlsDisabled} hasScan={!!scan} needsSource={!settings.source && !(scan?.skills.length)} harnessLabel={labelFor(activeHarness)} onPrepare={prepareAction} onRefresh={() => refresh(false, true)} onChangeSelection={changeSelection} onToggle={toggleSelection} />
        <DetailsPane skill={focusedSkill} onCopy={(path) => copyPath(path, 'Difference path')} />
      </div>
      </div>
      {/if}
    </section>
    <footer><span>Local skill folder manager</span><span>Files and scripts stay on this device</span></footer>
  </main>

  <PathDialog bind:dialog={pathDialog} kind={pathKind} bind:value={pathValue} error={loadError} {saving} disabled={controlsDisabled} onSave={savePath} />
  <AddHarnessDialog bind:dialog={addDialog} bind:name={addName} bind:destination={addDestination} error={loadError} busy={saving || choosing} disabled={controlsDisabled} onChooseFolder={chooseCustomFolder} onAdd={addHarness} />
  <ManageHarnessDialog bind:dialog={manageDialog} label={activeDescriptor?.label} bind:name={renameValue} bind:confirmRemoving error={loadError} {saving} disabled={controlsDisabled} onRename={renameHarness} onRemove={removeHarness} />
  <OperationDialog bind:dialog={installDialog} bind:prepared={preparedInstall} bind:acknowledgeLinks events={operationEvents} latestProgress={latestOperationProgress} started={operationStarted} running={operationRunning} error={operationError} onRun={runOperation} onClose={closeInstallDialog} />
</div>
