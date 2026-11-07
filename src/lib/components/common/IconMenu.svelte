<script>
  /**
   * 图标按钮 + 弹出菜单组合（状态栏主题/语言菜单等使用）。
   * items: [{ value, label, icon, iconColor, textColor, disabled, checkMark, trailing }]
   */
  import Icon from './Icon.svelte';

  let {
    icon,
    iconSize = 20,
    filled = false,
    buttonPadding = 8,
    tooltip = '',
    items = [],
    onSelect,
    menuWidth = 0, // 兼容旧参数：不再使用固定宽度，菜单宽度随内容自适应（同 PopupMenu）
    menuAlignEnd = true,
    iconColor = 'var(--md-on-primary-container)',
  } = $props();

  let open = $state(false);
  let anchor = $state(null);
  let menu = $state(null);
  let pos = $state({ right: 0, left: 0, top: 0 });

  /* 滞后卸载：open→false 后播 160ms 退出动画再移除菜单（重开时中断） */
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

  /* v2.5：菜单节点 portal 到 <body>，彻底脱离锚点祖先的布局树 ——
   * 无论祖先将来被加上 transform/filter/backdrop-filter（会改变 fixed 的
   * containing block）还是 flex 重排，菜单永远相对视口定位，不再可能被拉歪；
   * Svelte 5 事件委托同时挂在 document 上，portal 后点击照常生效。
   * 注：Svelte 卸载 if 块时按兄弟链移除节点，被移走的节点不在链上 ——
   * 必须在 action 的 destroy 里兑底 node.remove()，否则残留僵尸菜单。 */
  function portalToBody(node) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }

  function measure() {
    const rect = anchor.getBoundingClientRect();
    const estH = Math.min(items.length * 48 + 8, 480);
    /* 菜单上沿 = 按钮下沿（无间隙），右对齐锚定；宽度自适应内容 */
    let top = rect.bottom;
    if (top + estH > window.innerHeight) top = Math.max(8, rect.top - estH);
    pos = { right: Math.max(0, window.innerWidth - rect.right), left: rect.left, top };
  }

  function toggle() {
    if (open) {
      open = false;
      return;
    }
    measure();
    open = true;
  }

  /* 窗口尺寸变化时菜单跟随锚点（打开状态下才重算） */
  function onResize() {
    if (open && anchor) measure();
  }

  function select(v, item) {
    open = false;
    onSelect?.(v, item);
  }
</script>

<svelte:window
  onclick={(e) => {
    if (open && menu && !menu.contains(e.target) && anchor && !anchor.contains(e.target)) {
      open = false;
    }
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && open) open = false;
  }}
  onresize={onResize}
/>

<button
  class="md-icon-btn"
  bind:this={anchor}
  style="padding:{buttonPadding}px; color:{iconColor}"
  data-tip={open ? '' : tooltip}
  onclick={toggle}
>
  <Icon name={icon} size={iconSize} {filled} color={iconColor} />
</button>

{#if visible}
  {#key cycle}
    <div
      class="md-menu"
      class:closing
      use:portalToBody
      bind:this={menu}
      style="{menuAlignEnd ? `right:${pos.right}px` : `left:${pos.left}px`}; top:{pos.top}px; width:max-content; min-width:112px"
    >
      {#each items as item (item.value)}
        <div class="md-menu-item {item.disabled ? 'disabled' : ''}" onclick={() => select(item.value, item)}>
          {#if item.icon}
            <Icon name={item.icon} size={18} color={item.iconColor || 'var(--md-on-surface-variant)'} />
          {:else if item.flag}
            <span class="menu-flag">{item.flag}</span>
          {/if}
          <span class="ellipsis" style="white-space:nowrap; {item.textColor ? `color:${item.textColor}` : ''}">{item.label}</span>
          {#if item.checkMark}
            <Icon name="check" size={18} color="var(--flutter-green-600)" />
          {/if}
        </div>
      {/each}
    </div>
  {/key}
{/if}

<style>
  .menu-flag {
    font-size: 16px;
    line-height: 24px;
    flex-shrink: 0;
  }
</style>
