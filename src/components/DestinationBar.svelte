<script lang="ts">
  import type { HarnessDescriptor } from '../api'
  import { formatPath } from '../format'
  import MoreMenu from './MoreMenu.svelte'

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
    <button class="button button-secondary" type="button" onclick={onChoose} {disabled}>Choose destination…</button>
    <button class="button button-secondary" type="button" onclick={onOpen} {disabled}>Open folder</button>
    <MoreMenu label="More destination actions" items={[
      { label: 'Enter path…', disabled, onSelect: onEnterPath },
      { label: 'Copy path', disabled: !path, onSelect: onCopy },
      ...(descriptor?.builtIn ? [] : [{ label: 'Manage harness…', disabled, onSelect: onManage }]),
    ]} />
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
