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

  /* v2.5：与 IconMenu 相同的 portal 保险 —— 菜单挂到 <body>，
   * 不受锚点祖先 transform/filter 影响，fixed 定位永远相对视口。
   * destroy 里 node.remove() 兜底：Svelte 按兄弟链卸载，移走的节点不在链上。 */
  function portalToBody(node) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }

  /* 滞后卸载：open→false 后播 160ms 退出动画再移除（与 IconMenu/Dialog 同法） */
  let visible = $state(false);
  let cycle = $state(0);
  const closing = $derived(!open && visible);

  $effect(() => {
    if (open) {
      if (!visible) cycle += 1;
      visible = true;
      return;
    }
    if (visible) {
      const t = setTimeout(() => {
        visible = false;
      }, 160);
      return () => clearTimeout(t);
    }
  });

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

{#if visible}
  {#key cycle}
  <div
    class="md-menu"
    class:closing
    use:portalToBody
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
  {/key}
{/if}
