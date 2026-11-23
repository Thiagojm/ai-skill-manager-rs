<script lang="ts">
  import type { OperationEvent, PrepareResponse } from '../api'

  export let dialog: HTMLDialogElement
  export let prepared: PrepareResponse | null
  export let acknowledgeLinks: boolean
  export let events: OperationEvent[]
  export let latestProgress: OperationEvent | null
  export let started: boolean
  export let running: boolean
  export let error: string
  export let onRun: () => void
  export let onClose: () => void
  $: succeeded = events.filter((event) => event.kind === 'result' && event.success === true).length
  $: failed = events.filter((event) => event.kind === 'result' && event.success === false).length
</script>

<dialog bind:this={dialog} class="operation-dialog" aria-labelledby="install-title" oncancel={(event) => { if (running) event.preventDefault() }} onclose={() => { if (!running) prepared = null }}>
  {#if prepared}
    <div class="dialog-heading"><span class="eyebrow">Confirm {prepared.action}</span><h2 id="install-title">{prepared.action === 'install' ? 'Install complete skill folders?' : prepared.action === 'update' ? 'Replace complete skill folders?' : 'Move skill folders to the system trash?'}</h2>
      <p>{prepared.action === 'install' ? 'Selected folders will be copied to this destination:' : prepared.action === 'update' ? 'The existing complete folders will move to the system trash before replacements are placed:' : 'Selected complete folders will move to the system trash from:'}</p><code>{prepared.destinationPath}</code>
    </div>
    {#if prepared.action !== 'install'}
      <div class="dialog-warning"><strong>Recovery and refresh</strong><p>The operating system controls trash retention. Restore items manually from the system trash; external harnesses may need a refresh or restart.</p></div>
    {/if}
    {#if prepared.warnings.length}
      <div class="dialog-warning"><strong>Review these warnings</strong>
        {#each prepared.warnings as warning}<p>{warning}</p>{/each}
      </div>
    {/if}
    <ul class="install-list">
      {#each prepared.eligible as skill (skill.folderName)}
        <li><strong>{skill.folderName}</strong>
          {#each skill.warnings as warning}<small>{warning}</small>{/each}
          {#each skill.linkWarnings as link}<small><code>{link.path}</code> points to <code>{link.target}</code></small>{/each}
        </li>
      {/each}
    </ul>
    {#if prepared.skipped.length}
      <div class="dialog-skipped"><strong>Skipped selections</strong>{#each prepared.skipped as item}<p>{item}</p>{/each}</div>
    {/if}
    {#if prepared.eligible.some((skill) => skill.linkWarnings.length)}
      <label class="link-acknowledgement"><input type="checkbox" bind:checked={acknowledgeLinks} disabled={running} /> I reviewed the listed links and junctions. Copies materialize linked content; recycling removes link entries while preserving their targets.</label>
    {/if}
    {#if events.length}
      <div class="operation-log" role="log" aria-live="polite">
        {#each events as event, index (`${index}-${event.kind}`)}
          <p class:failed={event.success === false}>{event.folderName ? `${event.folderName}: ` : ''}{event.message}</p>
        {/each}
      </div>
    {/if}
    {#if started}
      <p class="operation-summary" role="status">{succeeded} successful · {failed} failed · {prepared.skipped.length} skipped during preparation{running ? ' · Running' : ''}</p>
    {/if}
    {#if error}<p class="dialog-error" role="alert">{error}</p>{/if}
    <div class="dialog-actions">
      {#if running}
        {@const current = latestProgress}
        <span class="operation-progress" aria-live="polite">{current?.folderName ? `${prepared.action} ${current.folderName}` : prepared.action} ({current?.completed ?? 0}/{current?.total ?? prepared.eligible.length})</span>
      {:else if events.some((event) => event.kind === 'finished')}
        <button class="button button-primary" type="button" onclick={onClose}>Done</button>
      {:else if error}
        <button class="button button-secondary" type="button" onclick={onClose}>Close and refresh</button>
      {:else}
        <button class="button button-secondary" type="button" onclick={onClose}>Cancel</button>
        <button class="button button-primary" type="button" onclick={onRun} disabled={prepared.eligible.some((skill) => skill.linkWarnings.length) && !acknowledgeLinks}>Confirm {prepared.action}</button>
      {/if}
    </div>
  {/if}
</dialog>
