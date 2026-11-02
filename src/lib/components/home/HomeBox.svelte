<script>
  /**
   * Home 卡片外壳 —— 对照 lib/features/home/widgets/home_box.dart。
   * 原版的 widthSpan / fixedCellHeight 由外层 MasonryGrid（spanOf）承担，
   * 此处只渲染 Card：r8 + hover 时 1px primary 边框 + 内层 padding 12。
   */
  let { isBorder = true, children } = $props();
  let hovered = $state(false);
</script>

<div
  class="home-box"
  class:borderless={!isBorder}
  class:hovered
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
>
  <div class="home-box-inner">
    {@render children?.()}
  </div>
</div>

<style>
  .home-box {
    background: var(--md-surface-container-low);
    border-radius: 8px;
    border: 1px solid transparent;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15);
    width: 100%;
  }
  .home-box.hovered {
    border-color: var(--md-primary);
  }
  .home-box.borderless {
    border-radius: 1px;
  }
  .home-box-inner {
    padding: 12px;
  }
  .home-box.borderless .home-box-inner {
    padding: 1px;
  }
</style>
