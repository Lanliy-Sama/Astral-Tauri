<script>
  /**
   * 弹出菜单（PopupMenuButton / DropdownButton 共用）。
   * items: [{ value, label, icon, iconColor, textColor, disabled, subtitle }]
   */
  import Icon from './Icon.svelte';

  let {
    open = $bindable(false),
    x = 0,
    y = 0,
    items = [],
    onSelect,
    alignEnd = false,
    itemIconSize = 24,
  } = $props();

  let menuEl = $state(null);
  let menuWidth = $state(0);

  $effect(() => {
    if (open && menuEl) {
      menuWidth = menuEl.offsetWidth;
    }
  });

  function close() {
    open = false;
  }

  function handleSelect(item) {
    if (item.disabled) return;
    open = false;
    onSelect?.(item.value, item);
  }
</script>

<svelte:window
  onclick={(e) => {
    if (open && menuEl && !menuEl.contains(e.target)) close();
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && open) close();
  }}
  onblur={close}
/>

{#if open}
  <div
    class="md-menu"
    bind:this={menuEl}
    style={alignEnd ? `right:${x}px; top:${y}px` : `left:${x}px; top:${y}px`}
  >
    {#each items as item (item.value)}
      <div
        class="md-menu-item {item.disabled ? 'disabled' : ''}"
        onclick={() => handleSelect(item)}
      >
        {#if item.icon}
          <Icon name={item.icon} size={itemIconSize} color={item.iconColor || ''} />
        {/if}
        <span class="grow" style={item.textColor ? `color:${item.textColor}` : ''}>{item.label}</span>
        {#if item.checkMark}
          <Icon name="check" size={18} color="var(--flutter-green-600)" />
        {/if}
      </div>
    {/each}
  </div>
{/if}
