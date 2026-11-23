<script lang="ts">
  import type { Harness, HarnessDescriptor } from '../api'

  export let harnesses: HarnessDescriptor[]
  export let activeHarness: Harness
  export let wide: boolean
  export let disabled: boolean
  export let element: HTMLElement
  export let onSwitch: (harness: Harness) => void
  export let onAdd: () => void
  import Icon from './Icon.svelte'
</script>

<nav class="harness-navigation" bind:this={element} aria-label="Harness destinations">
  {#if wide}
    <span class="eyebrow">Harnesses</span>
    <div class="harness-list">
      {#each harnesses as harness (harness.id)}
        <button type="button" aria-current={activeHarness === harness.id ? 'true' : undefined} onclick={() => onSwitch(harness.id)} {disabled}>{harness.label}</button>
      {/each}
    </div>
  {:else}
    <label class="compact-harness">Harness<select value={activeHarness} onchange={(event) => onSwitch(event.currentTarget.value)} {disabled}>
      {#each harnesses as harness (harness.id)}<option value={harness.id}>{harness.label}</option>{/each}
    </select></label>
  {/if}
  <button class="button button-secondary" type="button" onclick={onAdd} {disabled}><Icon name="plus" /> Add harness</button>
</nav>
