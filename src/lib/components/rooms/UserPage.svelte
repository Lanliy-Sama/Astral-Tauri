<script>
  /**
   * 用户页 —— user_page.dart 1:1 移植（连接时显示）。
   * 两个 FAB（右下角）：拓扑切换（list/hub）+ 房间设置（bar_chart）；
   * body：非 connected → '无数据'；无节点 → '房间内暂无成员'；
   * _showTopology → NetworkTopologyView；否则按 displayState 排序/过滤后以
   * userListStyle 渲染：list → ListUserCard 列表；simple/detailed → 瀑布流网格
   * （列数 ≥1200→3 / ≥900→2 / 其余 1，间距 8）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import MasonryGrid from '$lib/components/common/MasonryGrid.svelte';
  import ListUserCard from './cards/ListUserCard.svelte';
  import MiniUserCard from './cards/MiniUserCard.svelte';
  import AllUserCard from './cards/AllUserCard.svelte';
  import RoomSettingsSheet from './RoomSettingsSheet.svelte';
  import NetworkTopologyView from './topology/NetworkTopologyView.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { isServerNode } from '$lib/utils/network.js';
  import {
    UserSortOption,
    UserSortOrder,
    UserDisplayMode,
    UserListStyle,
  } from '$lib/stores/enums.js';

  /* 是否显示拓扑图（局部 state，IndexedStack 切 Tab 后保留） */
  let showTopology = $state(false);
  let settingsOpen = $state(false);

  /* 断点宽度（MediaQuery.of(context).size.width → window.innerWidth） */
  let windowWidth = $state(window.innerWidth);
  $effect(() => {
    const onResize = () => (windowWidth = window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  const connectionState = $derived(sm.connectionState.connectionState);
  const netStatus = $derived(sm.connectionState.netStatus);
  const nodes = $derived(netStatus?.nodes || []);
  const localIpv4 = $derived(sm.networkConfigState.ipv4 || '');
  const hasNodes = $derived(nodes.length > 0);

  /* 排序 + 过滤（拷贝数组排序；过滤 displayMode） */
  const filteredNodes = $derived.by(() => {
    const list = [...nodes];
    const ascending = sm.displayState.sortOrder === UserSortOrder.ascending;
    if (sm.displayState.sortOption === UserSortOption.latency) {
      list.sort((a, b) => {
        const c = (a.latencyMs ?? 0) - (b.latencyMs ?? 0);
        return ascending ? c : -c;
      });
    } else if (sm.displayState.sortOption === UserSortOption.nameLength) {
      list.sort((a, b) => {
        const c = (a.hostname || '').length - (b.hostname || '').length;
        return ascending ? c : -c;
      });
    }
    if (sm.displayState.displayMode === UserDisplayMode.users) {
      return list.filter((n) => !isServerNode(n));
    }
    if (sm.displayState.displayMode === UserDisplayMode.servers) {
      return list.filter((n) => isServerNode(n));
    }
    return list;
  });

  /* 用户网格列数 */
  const columns = $derived(
    windowWidth >= 1200 ? 3 : windowWidth >= 900 ? 2 : 1
  );

  const gridItems = $derived(
    filteredNodes.map((player) => ({
      component:
        sm.displayState.userListStyle === UserListStyle.simple ? MiniUserCard : AllUserCard,
      props: { player, localIpv4 },
    }))
  );
</script>

<div class="user-page">
  {#if connectionState !== 'connected'}
    <!-- 无数据 -->
    <div class="empty">
      <Icon name="cloud_off_rounded" size={48} color="var(--md-primary)" />
      <div class="empty-text">无数据</div>
    </div>
  {:else if !hasNodes}
    <!-- 房间内暂无成员 -->
    <div class="empty">
      <Icon name="people_outline" size={64} color="var(--md-primary-06b)" />
      <div class="empty-title">房间内暂无成员</div>
      <div class="empty-sub">当前没有其他玩家连接到房间</div>
    </div>
  {:else if showTopology}
    <!-- 拓扑图 -->
    <NetworkTopologyView
      {nodes}
      reduceUpdates={sm.appSettingsState.reduceAnimationUpdates}
      isInBackground={sm.uiState.isInBackground}
    />
  {:else if sm.displayState.userListStyle === UserListStyle.list}
    <!-- 列表样式：SliverPadding(8,0,8,8) + SliverList -->
    <div class="scroll list-scroll">
      {#each filteredNodes as player, index (player.peerId ?? index)}
        <ListUserCard {player} {localIpv4} showDivider={index < filteredNodes.length - 1} />
      {/each}
    </div>
  {:else}
    <!-- 简约/详细：SliverPadding(12,0,12,0) + 瀑布流网格（间距 8） -->
    <div class="scroll grid-scroll">
      <MasonryGrid {columns} gap={8} items={gridItems} />
    </div>
  {/if}

  <!-- FAB 列（右下角，16px 间距） -->
  <div class="fabs">
    <button
      class="md-fab"
      data-tip={showTopology ? '列表视图' : '拓扑图'}
      onclick={() => (showTopology = !showTopology)}
    >
      <Icon name={showTopology ? 'list' : 'hub'} size={24} />
    </button>
    <div class="fab-gap"></div>
    <button class="md-fab" data-tip="房间设置" onclick={() => (settingsOpen = true)}>
      <Icon name="bar_chart" size={24} />
    </button>
  </div>

  <RoomSettingsSheet bind:open={settingsOpen} />
</div>

<style>
  .user-page {
    position: relative;
    height: 100%;
    background: var(--md-surface);
    overflow: hidden;
  }
  .scroll {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .list-scroll {
    padding: 0 8px 8px;
  }
  .grid-scroll {
    padding: 0 12px;
  }

  /* 空态（居中列） */
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .empty-text {
    margin-top: 16px;
    font-weight: 700;
    color: var(--md-primary);
  }
  .empty-title {
    margin-top: 16px;
    font-size: 18px;
    font-weight: 700;
    color: var(--md-primary);
  }
  .empty-sub {
    margin-top: 8px;
    color: var(--md-on-surface-07);
  }

  /* FAB 列：右下角 endFloat */
  .fabs {
    position: absolute;
    right: 16px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    z-index: 10;
  }
  .fab-gap {
    height: 16px;
  }
  /* FAB 贴底，tooltip 显示在上方（Flutter Tooltip 会避开屏幕边缘） */
  .fabs .md-fab[data-tip]:hover::after {
    top: auto;
    bottom: calc(100% + 4px);
  }
</style>
