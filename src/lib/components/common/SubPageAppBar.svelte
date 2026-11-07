<script>
  /**
   * 子页面 AppBar（Flutter M3 AppBar：56px、surface 背景、居中标题、返回箭头）。
   */
  import Icon from './Icon.svelte';
  import { popPage } from '../../router/PageRouter.svelte';

  let { title = '', actions = null, showBackButton = true, onBack = null } = $props();

  function goBack() {
    if (onBack) onBack();
    else popPage();
  }
</script>

<div class="sub-app-bar">
  {#if showBackButton}
    <button class="md-icon-btn" onclick={goBack} data-tip="返回">
      <Icon name="arrow_back" size={24} color="var(--md-on-surface)" />
    </button>
  {:else}
    <div style="width:48px"></div>
  {/if}
  <div class="bar-title">{title}</div>
  <div class="bar-actions">
    {#if actions}
      {@render actions()}
    {/if}
  </div>
</div>

<style>
  .sub-app-bar {
    height: 56px;
    background: var(--md-surface);
    display: flex;
    align-items: center;
    padding: 0 8px 0 4px;
    flex-shrink: 0;
    position: relative;
    z-index: 10;
  }
  .bar-title {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    max-width: 60%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 22px;
    line-height: 28px;
    color: var(--md-on-surface);
  }
  .bar-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
