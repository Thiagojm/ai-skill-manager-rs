<script lang="ts">
  import type { OperationAction, SkillRow } from '../api'
  import { differenceLabels, formatPath, statusLabel } from '../format'

  const differenceKinds = ['added', 'changed', 'type_changed', 'removed'] as const

  export let skill: SkillRow | null
  export let disabled: boolean
  export let onAction: (action: OperationAction) => void
  export let onCopy: (path: string) => void
  import Icon from './Icon.svelte'
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
        <p>Links are copied as normal files and folders. Because the entry type changes, this skill may still show as Different after Install or Update, even when the content matches.</p>
        {#each skill.linkWarnings as link}<p><code>{link.path}</code><span aria-hidden="true">→</span><span class="sr-only">links to</span><code>{link.target}</code></p>{/each}
      </div>
    {/if}
    {#if skill.differences.length}
      <div class="differences"><div class="section-heading"><strong>Update destination differences</strong><span>{skill.differences.length}</span></div>
        <p>Update replaces the whole folder in the destination. Files that exist only in the destination will be removed. This list does not preview Uninstall. Linked files are copied as normal files, so this skill may still show as Different after updating.</p>
        <div class="diff-list">
          {#each differenceKinds as kind}{@const items = skill.differences.filter((difference) => difference.kind === kind)}{#if items.length}
            <h3 class="diff-kind diff-{kind}">{differenceLabels[kind]} <span>{items.length}</span></h3>
            <ul>{#each items as difference}<li><code>{difference.path}</code><button class="icon-button icon-button-small" type="button" aria-label={`Copy path ${difference.path}`} title="Copy path" onclick={() => onCopy(difference.path)}><Icon name="copy" size={14} /></button></li>{/each}</ul>
          {/if}{/each}
        </div>
      </div>
    {:else if skill.status === 'identical'}
      <div class="identical-note"><Icon name="check" /> Complete folder trees match</div>
    {/if}
  {:else}
    <div class="details-empty"><span class="empty-icon"><Icon name="info" size={20} /></span><h3>Skill details</h3><p>Select a skill row to see its paths, warnings, and file-level differences.</p></div>
  {/if}
</aside>
