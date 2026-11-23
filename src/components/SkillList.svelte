<script lang="ts">
  import type { OperationAction, SkillRow } from '../api'
  import { statusLabel } from '../format'

  export let filteredSkills: SkillRow[]
  export let statuses: string[]
  export let statusCounts: Record<string, number>
  export let search: string
  export let statusFilter: string
  export let focusedFolder: string
  export let searchInput: HTMLInputElement
  export let selectedNames: Set<string>
  export let selectedCount: number
  export let hiddenSelectedCount: number
  export let allFilteredSelected: boolean
  export let installSelectedCount: number
  export let updateSelectedCount: number
  export let uninstallSelectedCount: number
  export let stale: boolean
  export let loading: boolean
  export let controlsDisabled: boolean
  export let hasScan: boolean
  export let needsSource: boolean
  export let harnessLabel: string
  export let onPrepare: (action: OperationAction) => void
  export let onRefresh: () => void
  export let onChangeSelection: (mode: 'filtered' | 'all' | 'hidden' | 'visible') => void
  export let onToggle: (skill: SkillRow) => void
  $: visibleSelected = filteredSkills.filter((skill) => selectedNames.has(skill.folderName)).length
  $: headerChecked = filteredSkills.length > 0 && allFilteredSelected

  function indeterminate(node: HTMLInputElement, value: boolean) {
    node.indeterminate = value
    return { update(next: boolean) { node.indeterminate = next } }
  }

  function rowKey(event: KeyboardEvent, skill: SkillRow, index: number) {
    if (event.key === ' ') {
      event.preventDefault()
      if (event.type === 'keydown' && !event.repeat && !controlsDisabled && !stale) onToggle(skill)
      return
    }
    if (event.type !== 'keydown' || (event.key !== 'ArrowDown' && event.key !== 'ArrowUp')) return
    const target = index + (event.key === 'ArrowDown' ? 1 : -1)
    const next = filteredSkills[target]
    if (!next) return
    event.preventDefault()
    focusedFolder = next.folderName
    const rows = (event.currentTarget as HTMLElement).closest('.skill-list')?.querySelectorAll<HTMLElement>('.skill-summary')
    rows?.[target]?.focus()
  }
  import Icon from './Icon.svelte'
</script>

<div class="list-pane">
  <div class="list-toolbar">
    <h2 class="sr-only">Skills</h2>
    <div class="operation-actions">
      <span class="action-hint" role="status" title={selectedCount ? undefined : 'Select skills to install, update or uninstall.'}>{selectedCount ? '' : 'Select skills to install, update or uninstall.'}</span>
      <button class="button {installSelectedCount ? 'button-primary' : 'button-secondary'} install-button" type="button" onclick={() => onPrepare('install')} disabled={!installSelectedCount || stale || loading || controlsDisabled} aria-label="Review selected skills for installation">Install{installSelectedCount ? ` (${installSelectedCount})` : ''}</button>
      <button class="button button-secondary install-button" type="button" onclick={() => onPrepare('update')} disabled={!updateSelectedCount || stale || loading || controlsDisabled} aria-label="Review selected skills for update">Update{updateSelectedCount ? ` (${updateSelectedCount})` : ''}</button>
      <button class="button button-quiet install-button" type="button" onclick={() => onPrepare('uninstall')} disabled={!uninstallSelectedCount || stale || loading || controlsDisabled} aria-label="Review selected skills for uninstallation">Uninstall{uninstallSelectedCount ? ` (${uninstallSelectedCount})` : ''}</button>
      <button class="button button-quiet" type="button" onclick={onRefresh} disabled={loading || controlsDisabled} aria-label="Refresh comparison"><Icon name="refresh" /> <span>Refresh</span></button>
    </div>
  </div>
  <div class="filters">
    <label class="search-field"><Icon name="search" /><span class="sr-only">Search skills</span><input bind:this={searchInput} bind:value={search} placeholder="Search skills…" /></label><span class="result-count">{filteredSkills.length} {filteredSkills.length === 1 ? 'result' : 'results'}</span>
  </div>
  <div class="status-filters" aria-label="Filter by status">
    {#each statuses.filter((status) => status === 'all' || status === statusFilter || statusCounts[status]) as status}
      <button type="button" aria-pressed={statusFilter === status} onclick={() => statusFilter = status}>{status === 'all' ? 'All' : statusLabel(status)} <span>{statusCounts[status] ?? 0}</span></button>
    {/each}
  </div>
  <div class="selection-controls">
    <button class="button button-quiet" type="button" onclick={() => onChangeSelection('filtered')} disabled={controlsDisabled || stale || !filteredSkills.length || allFilteredSelected}>Select filtered</button>
    <button class="button button-quiet" type="button" onclick={() => onChangeSelection('all')} disabled={controlsDisabled || stale || !selectedCount}>Clear selection</button>
    <button class="button button-quiet" type="button" onclick={() => onChangeSelection('hidden')} disabled={controlsDisabled || stale || !hiddenSelectedCount}>Clear hidden selections</button>
  </div>
  {#if selectedCount > 0}<p class="selection-note">{selectedCount} selected for review · {hiddenSelectedCount} hidden by filters</p>{/if}

  {#if loading && !hasScan}
    <div class="empty-state"><p>Scanning folders…</p></div>
  {:else if needsSource}
    <div class="empty-state"><span class="empty-icon"><Icon name="folder" size={20} /></span><h3>Choose a skill library</h3><p>Select a folder to compare its skills with {harnessLabel}.</p></div>
  {:else if filteredSkills.length === 0}
    <div class="empty-state"><span class="empty-icon"><Icon name="search" size={20} /></span><h3>No skills found</h3><p>Try another search or check the selected folders.</p></div>
  {:else}
    <label class="select-visible"><input type="checkbox" checked={headerChecked} use:indeterminate={visibleSelected > 0 && !headerChecked} onclick={() => onChangeSelection(headerChecked ? 'visible' : 'filtered')} disabled={controlsDisabled || stale} /> Select all visible skills</label>
    <div class="skill-list" aria-label="Comparison results">
      {#each filteredSkills as skill, index (skill.folderName)}
        <div class="skill-row" class:focused={focusedFolder === skill.folderName}>
          <input type="checkbox" checked={selectedNames.has(skill.folderName)} onchange={() => onToggle(skill)} aria-label="Select {skill.folderName}" disabled={controlsDisabled || stale} />
          <button class="skill-summary" type="button" data-folder={skill.folderName} onclick={() => focusedFolder = skill.folderName} onkeydown={(event) => rowKey(event, skill, index)} onkeyup={(event) => rowKey(event, skill, index)} aria-keyshortcuts="ArrowUp ArrowDown Space" aria-label="Show details for {skill.folderName}">
            <span class="skill-title">{skill.name || skill.folderName}</span>
            <span class="skill-folder">{skill.folderName}</span>
          </button>
          <span class="status status-{skill.status}"><i></i>{statusLabel(skill.status)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
