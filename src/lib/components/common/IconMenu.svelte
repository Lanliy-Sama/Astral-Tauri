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

  function toggle() {
    if (open) {
      open = false;
      return;
    }
    const rect = anchor.getBoundingClientRect();
    const estH = Math.min(items.length * 48 + 8, 480);
    /* 菜单上沿 = 按钮下沿（无间隙），右对齐锚定；宽度自适应内容 */
    let top = rect.bottom;
    if (top + estH > window.innerHeight) top = Math.max(8, rect.top - estH);
    pos = { right: Math.max(0, window.innerWidth - rect.right), left: rect.left, top };
    open = true;
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
/>

<button
  class="md-icon-btn"
  bind:this={anchor}
  style="padding:{buttonPadding}px; color:{iconColor}"
  data-tip={tooltip}
  onclick={toggle}
>
  <Icon name={icon} size={iconSize} {filled} color={iconColor} />
</button>

{#if open}
  <div
    class="md-menu"
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
{/if}

<style>
  .menu-flag {
    font-size: 16px;
    line-height: 24px;
    flex-shrink: 0;
  }
</style>
