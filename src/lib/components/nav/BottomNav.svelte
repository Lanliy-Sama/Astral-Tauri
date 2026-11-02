<script>
  /**
   * 底部导航（移动端 / 窄窗）—— BottomNavigationBar(fixed)。
   * 小窗口模式：图标 20，选中字号 10，未选中不显示标签。
   */
  import Icon from '../common/Icon.svelte';
  import { sm } from '../../services/service-manager.svelte.js';

  let { items = [], isSmallWindow = false } = $props();

  const iconSize = $derived(isSmallWindow ? 20 : 24);
  const selectedFontSize = $derived(isSmallWindow ? 10 : 12);
  const unselectedFontSize = $derived(isSmallWindow ? 8 : 10);
</script>

<div class="bottom-nav">
  {#each items as item, index (index)}
    <button
      class="bottom-nav-item {sm.uiState.selectedIndex === index ? 'selected' : ''}"
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
