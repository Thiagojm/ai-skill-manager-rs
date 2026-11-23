<script lang="ts">
  export let settingsError: string
  export let settingsFile: string
  export let stale: boolean
  export let loadError: string
</script>

{#if settingsError || stale || loadError}
<!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable notices need keyboard focus.) -->
<div class="notice-region" role="region" aria-label="Application notices" tabindex="0">
{#if settingsError}
  <div class="notice notice-error" role="alert">
    <span class="notice-icon" aria-hidden="true">!</span>
    <span>{settingsError}<small>Settings file: {settingsFile || 'unavailable'}</small></span>
  </div>
{/if}
{#if stale}
  <div class="notice notice-error" role="alert">This comparison is outdated because the post-operation refresh failed. Refresh successfully before selecting skills or running another operation.</div>
{/if}
{#if loadError}
  <div class="notice notice-error" role="alert">
    <span class="notice-icon" aria-hidden="true">!</span><span>{loadError}</span>
  </div>
{/if}

</div>
{/if}
