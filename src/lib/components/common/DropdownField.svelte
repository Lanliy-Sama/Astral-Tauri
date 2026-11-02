<script>
  /**
   * DropdownButtonFormField —— M3 样式下拉选择。
   * options: [{ value, label }]；label 为浮动标签（可选）。
   * 菜单在锚点下方弹出（宽度至少与锚点一致，对齐 Flutter 下拉行为）。
   */
  import Icon from './Icon.svelte';

  let {
    value = $bindable(null),
    options = [],
    label = '',
    disabled = false,
    compact = false,
    onChange = null,
    style = '',
    width = null,
  } = $props();

  let menuOpen = $state(false);
  let anchorEl = $state(null);
  let menuEl = $state(null);
  let menuPos = $state({ left: 0, top: 0, minWidth: 0 });

  const selectedLabel = $derived(
    options.find((o) => String(o.value) === String(value))?.label ??
    (value != null && value !== '' ? String(value) : '')
  );

  function openMenu() {
    if (disabled) return;
    const rect = anchorEl.getBoundingClientRect();
    const mWidth = Math.max(rect.width, 112);
    const left = Math.min(rect.left, window.innerWidth - mWidth - 8);
    let top = rect.bottom + 4;
    if (menuEl) {
      // 预估高度后翻转判断
      const estH = Math.min(options.length * 48 + 8, 480);
      if (top + estH > window.innerHeight) {
        top = Math.max(8, rect.top - estH - 4);
      }
    }
    menuPos = { left, top, minWidth: mWidth };
    menuOpen = !menuOpen;
  }

  function handleSelect(v) {
    value = v;
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
  class="md-dropdown {disabled ? 'disabled' : ''} {compact ? 'compact' : ''}"
  bind:this={anchorEl}
  style={style}
  onclick={openMenu}
>
  {#if label}
    <span class="dd-label">{label}</span>
  {/if}
  <span class="dd-value">{selectedLabel}</span>
  <span class="dd-arrow"><Icon name="arrow_drop_down" size={24} /></span>
</div>

{#if menuOpen}
  <div
    class="md-menu"
    bind:this={menuEl}
    style="left:{menuPos.left}px; top:{menuPos.top}px; min-width:{menuPos.minWidth}px; {width ? `width:${width}px` : ''}"
  >
    {#each options as option (String(option.value))}
      <div
        class="md-menu-item {String(option.value) === String(value) ? 'selected' : ''}"
        onclick={() => handleSelect(option.value)}
        style={String(option.value) === String(value) ? 'background: var(--md-secondary-container); color: var(--md-on-secondary-container)' : ''}
      >
        <span class="grow ellipsis">{option.label}</span>
      </div>
    {/each}
  </div>
{/if}
