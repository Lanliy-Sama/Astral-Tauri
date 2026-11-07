<script>
  /**
   * 裸 DropdownButton（无 FormField 边框）—— 用于 ListTile trailing 的紧凑下拉。
   * 对应 Flutter DropdownButton：值文本 + arrow_drop_down，菜单锚点下方弹出。
   * underline=true 时显示 Flutter 默认下划线（SoftwareSettingsPage 最大重试次数）；
   * false 时隐藏（NetworkSettingsPage 协议/压缩下拉，外层自带 r8 容器）。
   */
  import Icon from '$lib/components/common/Icon.svelte';

  let {
    value = null,
    options = [],
    onChange = null,
    width = null,
    underline = false,
    itemFontSize = 14,
    style = '',
  } = $props();

  let menuOpen = $state(false);
  let anchorEl = $state(null);
  let menuEl = $state(null);
  let menuPos = $state({ left: 0, top: 0, minWidth: 0 });

  /* v2.5：与 IconMenu/PopupMenu 相同的 portal 保险 —— 菜单挂到 <body>，
   * 不受锚点祖先 transform/filter 影响，fixed 定位永远相对视口；
   * destroy 里 node.remove() 兜底（Svelte 按兄弟链卸载，移走的节点不在链上）。 */
  function portalToBody(node) {
    document.body.appendChild(node);
    return {
      destroy() {
        node.remove();
      },
    };
  }

  const selectedLabel = $derived(
    options.find((o) => String(o.value) === String(value))?.label ??
      (value != null && value !== '' ? String(value) : '')
  );

  function openMenu() {
    const rect = anchorEl.getBoundingClientRect();
    const mWidth = Math.max(rect.width, 112);
    const left = Math.min(rect.left, window.innerWidth - mWidth - 8);
    let top = rect.bottom + 4;
    const estH = Math.min(options.length * 48 + 8, 480);
    if (top + estH > window.innerHeight) {
      top = Math.max(8, rect.top - estH - 4);
    }
    menuPos = { left, top, minWidth: mWidth };
    menuOpen = !menuOpen;
  }

  function handleSelect(v) {
    menuOpen = false;
    onChange?.(v);
  }
</script>

<svelte:window
  onclick={(e) => {
    if (menuOpen && menuEl && !menuEl.contains(e.target) && anchorEl && !anchorEl.contains(e.target)) {
      menuOpen = false;
    }
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && menuOpen) menuOpen = false;
  }}
/>

<div
  class="bare-dd {underline ? 'with-underline' : ''}"
  bind:this={anchorEl}
  style={(width ? `width:${width}px;` : '') + style}
  onclick={openMenu}
>
  <span class="bare-dd-value ellipsis">{selectedLabel}</span>
  <span class="bare-dd-arrow"><Icon name="arrow_drop_down" size={24} /></span>
</div>

{#if menuOpen}
  <div
    class="md-menu"
    use:portalToBody
    bind:this={menuEl}
    style="left:{menuPos.left}px; top:{menuPos.top}px; min-width:{menuPos.minWidth}px"
  >
    {#each options as option (String(option.value))}
      <div
        class="md-menu-item {String(option.value) === String(value) ? 'selected' : ''}"
        onclick={() => handleSelect(option.value)}
        style="font-size:{itemFontSize}px; line-height:20px; {String(option.value) === String(value) ? 'background: var(--md-secondary-container); color: var(--md-on-secondary-container)' : ''}"
      >
        <span class="grow ellipsis">{option.label}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .bare-dd {
    display: inline-flex;
    align-items: center;
    cursor: pointer;
    min-height: 24px;
    position: relative;
    box-sizing: border-box;
  }
  .bare-dd-value {
    flex: 1 1 0;
    min-width: 0;
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
  }
  .bare-dd-arrow {
    display: inline-flex;
    color: var(--md-on-surface-variant);
    flex-shrink: 0;
  }
  .bare-dd.with-underline {
    border-bottom: 1px solid var(--md-outline-variant);
    padding-bottom: 4px;
  }
</style>
