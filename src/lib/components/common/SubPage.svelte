<script>
  /** 子页面外壳：SubPageAppBar + 可滚动内容（对应 Scaffold+AppBar 结构） */
  import SubPageAppBar from './SubPageAppBar.svelte';

  let { title = '', actions = null, showBackButton = true, onBack = null, scrollable = true, children } = $props();
</script>

<div class="sub-page">
  <SubPageAppBar {title} {actions} {showBackButton} {onBack} />
  <div class="sub-content" class:no-scroll={!scrollable}>
    {@render children?.()}
  </div>
</div>

<style>
  .sub-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--md-surface);
  }
  .sub-content {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .sub-content.no-scroll {
    overflow: hidden;
  }
</style>
