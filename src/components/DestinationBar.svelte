<script lang="ts">
  import type { HarnessDescriptor } from '../api'
  import { formatPath } from '../format'

  export let path: string | null | undefined
  export let descriptor: HarnessDescriptor | undefined
  export let warnings: string[]
  export let disabled: boolean
  export let onCopy: () => void
  export let onChoose: () => void
  export let onEnterPath: () => void
  export let onManage: () => void
  export let onOpen: () => void
</script>

<div class="destination-bar">
  <div class="destination-label"><span class="eyebrow">Managed destination</span><details class="path-disclosure"><summary title={path}>{formatPath(path)}</summary><code>{formatPath(path)}</code></details></div>
  <div class="destination-actions">
    <button class="button button-secondary" type="button" onclick={onCopy} disabled={!path}>Copy destination path</button>
    <button class="button button-secondary" type="button" onclick={onChoose} {disabled}>Choose destination…</button>
    <button class="button button-secondary" type="button" onclick={onEnterPath} {disabled}>Enter path…</button>
    {#if !descriptor?.builtIn}<button class="button button-secondary" type="button" onclick={onManage} {disabled}>Manage harness</button>{/if}
    <button class="button button-secondary" type="button" onclick={onOpen} {disabled}>Open folder</button>
  </div>
</div>

{#if (descriptor && !descriptor.destinationAvailable) || warnings.length}
<!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable notices need keyboard focus.) -->
<div class="notice-region" role="region" aria-label="Comparison notices" tabindex="0">
{#if descriptor && !descriptor.destinationAvailable}
  <p class="notice notice-error" role="alert"><span class="notice-icon" aria-hidden="true">!</span>{descriptor.builtIn ? 'This configured destination is currently unavailable. Choose another folder or install a selected skill to create it.' : 'This custom harness destination is unavailable. Choose an existing skills folder or remove this registration.'}</p>
{/if}
{#if warnings.length}
  <div class="scan-notices">
    {#each warnings.filter((warning) => descriptor?.destinationAvailable !== false || warning !== 'The configured destination folder does not exist yet.') as warning}
      <p class="notice notice-info"><span class="notice-icon" aria-hidden="true">i</span>{warning}</p>
    {/each}
  </div>
{/if}

</div>
{/if}
