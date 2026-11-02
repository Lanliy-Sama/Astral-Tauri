<script>
  /**
   * 首页 —— 对照 lib/features/home/pages/home_page.dart。
   * StaggeredGrid（MasonryGrid）4 卡片 span=2 + 尾部 100px 占位 + FAB(ConnectButton)。
   */
  import MasonryGrid from '../common/MasonryGrid.svelte';
  import ConnectButton from './ConnectButton.svelte';
  import UserIpBox from './UserIpBox.svelte';
  import QuickNetworkConfig from './QuickNetworkConfig.svelte';
  import AboutHome from './AboutHome.svelte';
  import HitokotoCard from './HitokotoCard.svelte';

  let windowWidth = $state(window.innerWidth);

  $effect(() => {
    const onResize = () => (windowWidth = window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  /* 列数：≥1200→5 ≥900→4 ≥600→3 否则 2 */
  const columns = $derived(
    windowWidth >= 1200 ? 5 : windowWidth >= 900 ? 4 : windowWidth >= 600 ? 3 : 2
  );

  const items = $derived([
    { component: UserIpBox },
    { component: QuickNetworkConfig },
    { component: AboutHome },
    { component: HitokotoCard },
  ]);

  /* 尾部全宽 100px 占位（为 FAB 预留滚动空间） */
  const spacerStyle = 'height:100px;';
</script>

<div class="home-page">
  <div class="home-scroll">
    <div class="home-padding">
      <MasonryGrid {columns} gap={8} {items} spanOf={() => 2}>
      </MasonryGrid>
      <div style={spacerStyle}></div>
    </div>
  </div>
  <!-- Scaffold FAB endFloat：16px 边距 -->
  <div class="fab-area">
    <ConnectButton />
  </div>
</div>

<style>
  .home-page {
    height: 100%;
    position: relative;
    background: var(--md-surface);
  }
  .home-scroll {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .home-padding {
    padding: 14px 14px calc(14px + env(safe-area-inset-bottom, 0px));
  }
  .fab-area {
    position: absolute;
    right: 16px;
    bottom: 16px;
    pointer-events: none;
  }
  .fab-area :global(.connect-button-zone) {
    pointer-events: auto;
  }
</style>
