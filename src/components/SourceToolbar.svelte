<script lang="ts">
  import { formatPath } from '../format'

  export let source: string | null
  export let disabled: boolean
  export let onCopy: () => void
  export let onChoose: () => void
  export let onEnterPath: () => void
</script>

<section class="source-card" aria-labelledby="source-title">
  <div class="source-copy">
    <h1 id="source-title">Skill library</h1>
    {#if !source}<p>Choose a parent folder containing skill folders.</p>{/if}
    <details class="path-disclosure"><summary title={source ?? ''}>{source || 'Choose a folder containing skills'}</summary><code>{formatPath(source)}</code></details>
  </div>
  <div class="destination-actions"><button class="button button-secondary" type="button" onclick={onCopy} disabled={!source}>Copy source path</button><button class="button button-primary" type="button" onclick={onChoose} {disabled}>
    <span aria-hidden="true">＋</span> Choose folder…
  </button><button class="button button-secondary" type="button" onclick={onEnterPath} {disabled}>Enter path…</button></div>
</section>
