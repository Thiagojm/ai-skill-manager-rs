<script lang="ts">
  export let dialog: HTMLDialogElement
  export let name: string
  export let destination: string
  export let error: string
  export let busy: boolean
  export let disabled: boolean
  export let onChooseFolder: () => void
  export let onAdd: () => void
</script>

<dialog bind:this={dialog} class="operation-dialog management-dialog" aria-labelledby="add-harness-title" oncancel={(event) => { if (busy) event.preventDefault() }} onclose={() => { error = '' }}>
  <form onsubmit={(event) => { event.preventDefault(); onAdd() }}>
  <div class="dialog-heading"><span class="eyebrow">Custom harness</span><h2 id="add-harness-title">Add harness</h2><p>Register an existing skills folder. The folder will stay where it is.</p></div>
  <label class="management-field">Name<input bind:value={name} autocomplete="off" {disabled} /></label>
  <label class="management-field">Skills folder<input bind:value={destination} placeholder="Absolute path to an existing folder" autocomplete="off" {disabled} /></label>
  <button class="button button-secondary" type="button" onclick={onChooseFolder} {disabled}>Choose folder…</button>
  {#if error}<p class="dialog-error" role="alert">{error}</p>{/if}
  <div class="dialog-actions"><button class="button button-secondary" type="button" onclick={() => dialog.close()} {disabled}>Cancel</button><button class="button button-primary" type="submit" disabled={disabled || !name.trim() || !destination}>Add harness</button></div>
  </form>
</dialog>
