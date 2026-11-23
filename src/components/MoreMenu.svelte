<script lang="ts">
  import { tick } from 'svelte'

  export let label: string
  export let items: { label: string; disabled?: boolean; onSelect: () => void }[]

  const id = `menu-${Math.random().toString(36).slice(2)}`
  let open = false
  let root: HTMLElement
  let trigger: HTMLButtonElement
  let menu: HTMLElement

  const enabledItems = () => [...menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')]

  async function show() {
    open = true
    await tick()
    enabledItems()[0]?.focus()
  }

  function close(returnFocus: boolean) {
    open = false
    if (returnFocus) trigger.focus()
  }

  function keydown(event: KeyboardEvent) {
    const items = enabledItems()
    const index = items.indexOf(document.activeElement as HTMLButtonElement)
    const next = { ArrowDown: index + 1, ArrowUp: index - 1, Home: 0, End: items.length - 1 }[event.key]
    if (next !== undefined) {
      event.preventDefault()
      items[(next + items.length) % items.length]?.focus()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      close(true)
    } else if (event.key === 'Tab') close(false)
  }

  function outside(event: PointerEvent) {
    if (open && !root.contains(event.target as Node)) close(true)
  }

  function select(item: (typeof items)[number]) {
    close(true)
    item.onSelect()
  }
  import Icon from './Icon.svelte'
</script>

<svelte:window onpointerdown={outside} />

<div class="more-menu" bind:this={root}>
  <button bind:this={trigger} class="button button-secondary" type="button" aria-label={label} aria-haspopup="menu" aria-expanded={open} aria-controls={id} onclick={() => open ? close(true) : show()}><Icon name="more" /> More</button>
  {#if open}
    <div bind:this={menu} id={id} class="menu-popup" role="menu" aria-label={label} tabindex="-1" onkeydown={keydown}>
      {#each items as item}
        <button type="button" role="menuitem" tabindex="-1" disabled={item.disabled} onclick={() => select(item)}>{item.label}</button>
      {/each}
    </div>
  {/if}
</div>
