<script lang="ts">
  import { formatPath } from '../format'
  import MoreMenu from './MoreMenu.svelte'

  export let source: string | null
  export let disabled: boolean
  export let onCopy: () => void
  export let onChoose: () => void
  export let onEnterPath: () => void
  export let onOpen: () => void
</script>

<section class="source-card" aria-labelledby="source-title">
  <div class="source-copy">
    <h1 id="source-title">Skill library</h1>
    {#if !source}<p>Choose a parent folder containing skill folders.</p>{/if}
    <details class="path-disclosure"><summary title={source ?? ''}>{source || 'Choose a folder containing skills'}</summary><code>{formatPath(source)}</code></details>
  </div>
  <div class="destination-actions"><button class="button button-primary" type="button" onclick={onChoose} {disabled}>
    <span aria-hidden="true">＋</span> Choose folder…
  </button><MoreMenu label="More skill library actions" items={[
    { label: 'Enter path…', disabled, onSelect: onEnterPath },
    { label: 'Copy path', disabled: !source, onSelect: onCopy },
    { label: 'Open folder', disabled: disabled || !source, onSelect: onOpen },
  ]} /></div>
</section>
