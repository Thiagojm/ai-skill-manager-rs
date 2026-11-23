<script lang="ts">
  export let dialog: HTMLDialogElement
  export let label: string | undefined
  export let name: string
  export let confirmRemoving: boolean
  export let error: string
  export let saving: boolean
  export let disabled: boolean
  export let onRename: () => void
  export let onRemove: () => void
</script>

<dialog bind:this={dialog} class="operation-dialog management-dialog" aria-labelledby="manage-harness-title" oncancel={(event) => { if (saving) event.preventDefault() }} onclose={() => { confirmRemoving = false; error = '' }}>
  <div class="dialog-heading"><span class="eyebrow">Custom harness</span><h2 id="manage-harness-title">Manage {label}</h2></div>
  {#if confirmRemoving}
    <form onsubmit={(event) => { event.preventDefault(); onRemove() }}>
    <div class="dialog-warning"><strong>Remove this registration?</strong><p>Only the saved registration and destination mapping will be removed. The skills folder and every file in it will remain untouched.</p></div>
    {#if error}<p class="dialog-error" role="alert">{error}</p>{/if}
    <div class="dialog-actions"><button class="button button-secondary" type="button" onclick={() => confirmRemoving = false} {disabled}>Keep harness</button><button class="button button-danger" type="submit" {disabled}>Remove registration</button></div>
    </form>
  {:else}
    <form onsubmit={(event) => { event.preventDefault(); onRename() }}>
    <label class="management-field">Name<input bind:value={name} autocomplete="off" {disabled} /></label>
    {#if error}<p class="dialog-error" role="alert">{error}</p>{/if}
    <div class="dialog-actions"><button class="button button-quiet" type="button" onclick={() => confirmRemoving = true} {disabled}>Remove harness…</button><button class="button button-secondary" type="button" onclick={() => dialog.close()} {disabled}>Close</button><button class="button button-primary" type="submit" disabled={disabled || !name.trim()}>Save name</button></div>
    </form>
  {/if}
</dialog>
