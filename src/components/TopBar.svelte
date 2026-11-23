<script lang="ts">
  import appIcon from '../../src-tauri/icons/128x128.png'
  import type { Theme } from '../api'

  export let theme: Theme
  export let disabled: boolean
  export let onToggleTheme: () => void
  import Icon from './Icon.svelte'
</script>

<header class="topbar">
  <a class="brand" href="#main" aria-label="AI Skill Manager home">
    <img class="brand-mark" src={appIcon} alt="" width="34" height="34" />
    <span><strong>Skill Manager</strong><small>Local library comparison</small></span>
  </a>
  <div class="topbar-actions">
    <span class="local-indicator"><i></i> Local workspace</span>
    <button class="icon-button" type="button" onclick={onToggleTheme} {disabled} aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'} title="Change theme">
      <Icon name={theme === 'dark' ? 'sun' : 'moon'} size={18} />
    </button>
  </div>
</header>
