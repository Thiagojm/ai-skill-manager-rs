<script lang="ts">
  import type { Harness, HarnessDescriptor, OverviewResponse } from '../api'

  export let harnesses: HarnessDescriptor[]
  export let activeHarness: Harness
  export let wide: boolean
  export let disabled: boolean
  export let element: HTMLElement
  export let onSwitch: (harness: Harness) => void
  export let onAdd: () => void
  export let overview: OverviewResponse | null
  export let outdated: boolean

  function summary(harness: Harness, overview: OverviewResponse | null, outdated: boolean) {
    const column = overview?.harnesses.find((item) => item.harness === harness)
    if (!column) return overview ? 'Not in overview' : ''
    const text = column.error ? 'Error' : !column.available ? 'Unavailable'
      : `${column.counts.missing ?? 0} missing · ${column.counts.different ?? 0} different`
    return outdated ? `${text} · Outdated` : text
  }
  import Icon from './Icon.svelte'
</script>

<nav class="harness-navigation" bind:this={element} aria-label="Harness destinations">
  {#if wide}
    <span class="eyebrow">Harnesses</span>
    <div class="harness-list">
      {#each harnesses as harness (harness.id)}
        <button type="button" aria-current={activeHarness === harness.id ? 'true' : undefined} onclick={() => onSwitch(harness.id)} {disabled}><span>{harness.label}</span>{#if overview}<span class="harness-summary">{summary(harness.id, overview, outdated)}</span>{/if}</button>
      {/each}
    </div>
  {:else}
    <label class="compact-harness">Harness<select value={activeHarness} onchange={(event) => onSwitch(event.currentTarget.value)} {disabled}>
      {#each harnesses as harness (harness.id)}<option value={harness.id}>{harness.label}{overview ? ` — ${summary(harness.id, overview, outdated)}` : ''}</option>{/each}
    </select></label>
  {/if}
  <button class="button button-secondary" type="button" onclick={onAdd} {disabled}><Icon name="plus" /> Add harness</button>
</nav>
