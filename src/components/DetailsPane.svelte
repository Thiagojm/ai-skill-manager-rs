<script lang="ts">
  import type { OperationAction, SkillRow } from '../api'
  import { differenceLabels, formatPath, statusLabel } from '../format'

  export let skill: SkillRow | null
  export let disabled: boolean
  export let onAction: (action: OperationAction) => void
  export let onCopy: (path: string) => void
</script>

<aside class="details-pane" aria-label="Skill details">
  {#if skill}
    <div class="details-header">
      <div><span class="eyebrow">Skill details</span><h2>{skill.name || skill.folderName}</h2></div>
      <span class="status status-{skill.status}"><i></i>{statusLabel(skill.status)}</span>
    </div>
    {#if skill.eligibleActions.length}
      <div class="detail-actions">
        {#each skill.eligibleActions as action}<button class="button {action === 'uninstall' ? 'button-quiet' : 'button-secondary'}" type="button" onclick={() => onAction(action)} {disabled}>{action === 'install' ? 'Install' : action === 'update' ? 'Update' : 'Uninstall'} this skill</button>{/each}
      </div>
    {/if}
    {#if skill.description}<p class="description">{skill.description}</p>{/if}
    <dl class="path-list">
      <div><dt>Folder identity</dt><dd><code>{skill.folderName}</code></dd></div>
      <div><dt>Source path</dt><dd title={skill.sourcePath ?? ''}><code>{formatPath(skill.sourcePath)}</code></dd></div>
      <div><dt>Destination path</dt><dd title={skill.destinationPath ?? ''}><code>{formatPath(skill.destinationPath)}</code></dd></div>
    </dl>
    {#if skill.error}<div class="detail-warning error-text"><strong>Scan issue</strong><p>{skill.error}</p></div>{/if}
    {#if skill.warnings.length}
      <div class="detail-warning"><strong>Metadata and folder warnings</strong>
        {#each skill.warnings as warning}<p>{warning}</p>{/each}
      </div>
    {/if}
    {#if skill.linkWarnings.length}
      <div class="detail-warning link-list"><strong>Links and junctions</strong>
        <p>Copies replace links with ordinary files and folders. A linked source can remain Different after Install or Update because entry types differ, even when content matches.</p>
        {#each skill.linkWarnings as link}<p><code>{link.path}</code><span>→</span><code>{link.target}</code></p>{/each}
      </div>
    {/if}
    {#if skill.differences.length}
      <div class="differences"><div class="section-heading"><strong>Update destination differences</strong><span>{skill.differences.length}</span></div>
        <p>Update replaces the complete destination folder, including removal of destination-only entries. This does not preview Uninstall. Copies materialize links, so linked sources can remain Different.</p>
        <ul>{#each skill.differences as difference}<li><span class="diff-kind diff-{difference.kind}">{differenceLabels[difference.kind]}</span><code>{difference.path}</code><button class="button button-secondary copy-difference" type="button" aria-label={`Copy difference path ${difference.path}`} onclick={() => onCopy(difference.path)}>Copy</button></li>{/each}</ul>
      </div>
    {:else if skill.status === 'identical'}
      <div class="identical-note"><span aria-hidden="true">✓</span> Complete folder trees match</div>
    {/if}
  {:else}
    <div class="details-empty"><span class="empty-icon" aria-hidden="true">⌑</span><h3>Skill details</h3><p>Select a skill row to see its paths, warnings, and file-level differences.</p></div>
  {/if}
</aside>
