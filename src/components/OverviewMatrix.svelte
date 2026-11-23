<script lang="ts">
  import type { Harness, HarnessDescriptor, OverviewResponse, OverviewRow } from '../api'
  import { statusLabel } from '../format'
  import Icon from './Icon.svelte'

  export let overview: OverviewResponse | null
  export let harnesses: HarnessDescriptor[]
  export let statuses: string[]
  export let searchInput: HTMLInputElement
  export let outdated: boolean
  export let running: boolean
  export let disabled: boolean
  export let onRefresh: () => void
  export let onOpen: (harness: Harness, folderName: string) => void

  let search = ''
  let statusFilter = 'all'

  const label = (harness: Harness) => harnesses.find((item) => item.id === harness)?.label ?? harness
  $: columns = (overview?.harnesses ?? []).map((column) => ({ ...column, label: label(column.harness), usable: column.available && !column.error }))
  $: rows = groupRows(columns)
  $: counts = countCells(rows, columns)
  $: query = search.trim().toLocaleLowerCase()
  $: filtered = rows.filter((row) => {
    const cells = [...row.cells.values()]
    const matchesSearch = !query || cells.some((cell) => cell.folderName.toLocaleLowerCase().includes(query)
      || cell.name?.toLocaleLowerCase().includes(query) || cell.description?.toLocaleLowerCase().includes(query))
    return matchesSearch && (statusFilter === 'all' || columns.some((column) => column.usable && cellStatus(row, column.harness) === statusFilter))
  })

  function groupRows(list: typeof columns) {
    const grouped = new Map<string, { identity: string, title: string, folderName: string, cells: Map<Harness, OverviewRow> }>()
    for (const column of list) {
      for (const cell of column.rows) {
        const row = grouped.get(cell.identity) ?? { identity: cell.identity, title: cell.name || cell.folderName, folderName: cell.folderName, cells: new Map() }
        row.cells.set(column.harness, cell)
        grouped.set(cell.identity, row)
      }
    }
    return [...grouped.values()].sort((a, b) => a.folderName.localeCompare(b.folderName))
  }

  function cellStatus(row: { cells: Map<Harness, OverviewRow> }, harness: Harness) {
    return row.cells.get(harness)?.status ?? 'not_present'
  }

  function countCells(list: typeof rows, cols: typeof columns) {
    const result: Record<string, number> = { all: list.length }
    for (const row of list) {
      for (const column of cols) {
        if (!column.usable) continue
        const status = cellStatus(row, column.harness)
        result[status] = (result[status] ?? 0) + 1
      }
    }
    return result
  }

  const filterLabel = (status: string) => status === 'all' ? 'All' : status === 'not_present' ? 'Not present' : statusLabel(status)
</script>

<div class="overview-pane" aria-label="Overview of all harnesses">
  <div class="list-toolbar">
    <h2>Overview</h2>
    <div class="operation-actions">
      <span class="action-hint">{#if overview}Updated {new Date(overview.generatedAt).toLocaleTimeString()}{/if}</span>
      <button class="button button-quiet" type="button" onclick={onRefresh} {disabled} aria-label="Refresh overview"><Icon name="refresh" /> <span>Refresh</span></button>
    </div>
  </div>
  {#if outdated}<p class="notice notice-info overview-outdated"><span class="notice-icon"><Icon name="info" size={12} /></span>This overview is outdated. Refresh to update it.</p>{/if}
  <div class="filters">
    <label class="search-field"><Icon name="search" /><span class="sr-only">Search overview</span><input bind:this={searchInput} bind:value={search} placeholder="Search skills…" /></label><span class="result-count">{filtered.length} {filtered.length === 1 ? 'result' : 'results'}</span>
  </div>
  <div class="status-filters" aria-label="Filter by status">
    {#each [...statuses, 'not_present'].filter((status) => status === 'all' || status === statusFilter || counts[status]) as status}
      <button type="button" aria-pressed={statusFilter === status} onclick={() => statusFilter = status}>{filterLabel(status)} <span>{counts[status] ?? 0}</span></button>
    {/each}
  </div>
  {#if !overview}
    <div class="empty-state"><p>{running ? 'Building overview…' : 'Refresh to build the overview.'}</p></div>
  {:else if !filtered.length}
    <div class="empty-state"><span class="empty-icon"><Icon name="search" size={20} /></span><h3>No skills found</h3><p>Try another search or status.</p></div>
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (The scroll area needs keyboard focus.) -->
    <div class="matrix-scroll" tabindex="0" role="region" aria-label="Skill status by harness">
      <table class="matrix">
        <thead><tr>
          <th scope="col">Skill</th>
          {#each columns as column (column.harness)}
            <th scope="col" title={column.error ?? column.destinationPath ?? ''}>{column.label}{#if column.error}<span class="matrix-column-state">Error</span>{:else if !column.available}<span class="matrix-column-state">Unavailable</span>{/if}</th>
          {/each}
        </tr></thead>
        <tbody>
          {#each filtered as row (row.identity)}
            <tr>
              <th scope="row"><span class="skill-title">{row.title}</span>{#if row.title !== row.folderName}<span class="skill-folder">{row.folderName}</span>{/if}</th>
              {#each columns as column (column.harness)}
                {@const cell = row.cells.get(column.harness)}
                <td>
                  {#if column.error}<span class="matrix-state">Error</span>
                  {:else if !column.available}<span class="matrix-state">Unavailable</span>
                  {:else if cell}<button class="status status-{cell.status} matrix-cell" type="button" {disabled} aria-label="Skill {cell.folderName} in {column.label}: {statusLabel(cell.status)}" onclick={() => onOpen(column.harness, cell.folderName)}><i></i>{statusLabel(cell.status)}</button>
                  {:else}<span class="matrix-state">Not present</span>{/if}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
