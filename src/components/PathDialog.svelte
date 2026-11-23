<script lang="ts">
  export let dialog: HTMLDialogElement
  export let kind: 'source' | 'destination'
  export let value: string
  export let error: string
  export let saving: boolean
  export let disabled: boolean
  export let onSave: (event: SubmitEvent) => void
</script>

<dialog bind:this={dialog} class="operation-dialog management-dialog" aria-labelledby="path-title" oncancel={(event) => { if (saving) event.preventDefault() }}>
  <form onsubmit={onSave}>
    <div class="dialog-heading"><h2 id="path-title">Enter {kind} folder path</h2><p>Use an absolute path. Paths are literal; ~ and environment variables are not expanded.</p></div>
    <label class="management-field">Folder path<input bind:value autocomplete="off" required {disabled} /></label>
    {#if error}<p class="dialog-error" role="alert">{error}</p>{/if}
    <div class="dialog-actions"><button class="button button-secondary" type="button" onclick={() => dialog.close()} {disabled}>Cancel</button><button class="button button-primary" type="submit" disabled={disabled || !value.trim()}>Use folder</button></div>
  </form>
</dialog>
