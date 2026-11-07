<script>
  /**
   * 底部导航（移动端 / 窄窗）—— BottomNavigationBar(fixed)。
   * 小窗口模式：图标 20，选中字号 10，未选中不显示标签。
   * 液态玻璃：选中项下方为滑动玻璃胶囊（LiquidGlass.svelte 材质，带过冲回弹）。
   */
  import Icon from '../common/Icon.svelte';
  import { sm } from '../../services/ServiceManager.svelte';

  let { items = [], isSmallWindow = false } = $props();

  const iconSize = $derived(isSmallWindow ? 20 : 24);
  const selectedFontSize = $derived(isSmallWindow ? 10 : 12);
  const unselectedFontSize = $derived(isSmallWindow ? 8 : 10);

  let navEl = null;
  let itemEls = [];
  let pillX = $state(0);
  let pillW = $state(0);

  /* 选中项变化 → 胶囊滑到该项（测量 offsetLeft/offsetWidth） */
  $effect(() => {
    const idx = sm.uiState.selectedIndex;
    const el = itemEls[idx];
    if (el) {
      pillX = el.offsetLeft + 6;
      pillW = el.offsetWidth - 12;
    }
  });

  $effect(() => {
    const onResize = () => {
      const el = itemEls[sm.uiState.selectedIndex];
      if (el) {
        pillX = el.offsetLeft + 6;
        pillW = el.offsetWidth - 12;
      }
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });
</script>

<div class="bottom-nav" bind:this={navEl}>
  <!-- 液态玻璃选中胶囊（仅 liquid-glass 模式显示） -->
  <div
    class="selection-pill lg-glass lg-spec"
    style="width:{pillW}px; transform: translateX({pillX}px)"
  ></div>
  {#each items as item, index (index)}
    <button
      class="bottom-nav-item {sm.uiState.selectedIndex === index ? 'selected' : ''}"
      bind:this={itemEls[index]}
      onclick={() => (sm.uiState.selectedIndex = index)}
    >
      <Icon
        name={sm.uiState.selectedIndex === index ? item.activeIcon : item.icon}
        size={iconSize}
        color={sm.uiState.selectedIndex === index ? 'var(--md-primary)' : 'var(--md-on-surface-variant)'}
      />
      {#if sm.uiState.selectedIndex === index || !isSmallWindow}
        <span
          class="bn-label {sm.uiState.selectedIndex === index ? 'sel' : 'unsel'}"
          style="font-size:{sm.uiState.selectedIndex === index ? selectedFontSize : unselectedFontSize}px"
        >{item.label}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .bottom-nav {
    background: var(--md-surface-container-low);
    display: flex;
    border-top: 1px solid var(--md-outline-variant);
    flex-shrink: 0;
    position: relative;
  }
  .bottom-nav-item {
    flex: 1;
    min-height: 56px;
    border: none;
    background: transparent;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 6px 0;
    cursor: pointer;
    position: relative;
    z-index: 1;
  }
  /* 液态玻璃选中胶囊：默认隐藏（非玻璃模式保持原样），玻璃模式由 LiquidGlass.svelte 激活 */
  .selection-pill {
    display: none;
    position: absolute;
    top: 6px;
    height: 44px;
    left: 0;
    border-radius: 16px;
    pointer-events: none;
    will-change: transform;
  }
  :global(html.liquid-glass) .selection-pill {
    display: block;
    transition: transform 420ms cubic-bezier(0.22, 1.2, 0.36, 1), width 420ms cubic-bezier(0.22, 1.2, 0.36, 1);
  }
  .bn-label {
    line-height: 16px;
    transition: color 100ms ease;
  }
  .bn-label.sel {
    color: var(--md-primary);
    font-weight: 500;
  }
  .bn-label.unsel {
    color: var(--md-on-surface-variant);
  }
</style>
