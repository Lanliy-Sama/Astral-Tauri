<script>
  /**
   * 房间页（IndexedStack Tab）—— 翻译自 features/rooms/pages/room_page.dart。
   * 结构：Column[connected&&selectedRoom ? 当前房间卡 : 无, Expanded(UserPage|房间瀑布流)] + FAB 行（仅 idle）。
   * 当前房间卡点击 → copyShareLink(linkOnly: true)（仅复制链接，不弹分享对话框）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import MasonryGrid from '$lib/components/common/MasonryGrid.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import RoomCard from './RoomCard.svelte';
  import UserPage from './UserPage.svelte'; // 任务 7-a：UserPage 由另一同事实现（无 props）
  import RoomReorderSheet from './RoomReorderSheet.svelte';
  import AddRoomDialog from './dialogs/AddRoomDialog.svelte';
  import EditRoomDialog from './dialogs/EditRoomDialog.svelte';
  import PasteImportDialog from './dialogs/PasteImportDialog.svelte';
  import ShareExportDialog, { copyRoomShareLink } from './dialogs/ShareExportDialog.svelte';

  /* 断点（按 window.innerWidth）：≥1200→4 ≥900→3 ≥600→2 否则 1 */
  let windowWidth = $state(window.innerWidth);
  $effect(() => {
    const onResize = () => (windowWidth = window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });
  const columns = $derived(
    windowWidth >= 1200 ? 4 : windowWidth >= 900 ? 3 : windowWidth >= 600 ? 2 : 1
  );

  const connection = $derived(sm.connectionState.connectionState);
  const isConnected = $derived(connection === 'connected');
  const isIdle = $derived(connection === 'idle');
  const selectedRoom = $derived(sm.roomState.selectedRoom);

  const statusText = $derived(
    (isConnected ? '已连接' : connection === 'connecting' ? '连接中' : '未连接') +
      (isConnected ? ' (点击分享房间)' : '')
  );

  /* 弹窗开关（组件顶层 state，Tab 切换保活保留） */
  let addOpen = $state(false);
  let pasteOpen = $state(false);
  let reorderOpen = $state(false);
  let editOpen = $state(false);
  let editTarget = $state(null);
  let shareOpen = $state(false);
  let shareTarget = $state(null);
  let currentHovered = $state(false);

  function openEdit(room) {
    editTarget = room;
    editOpen = true;
  }

  function openShare(room) {
    shareTarget = room;
    shareOpen = true;
  }

  /* 瀑布流 items：RoomCard span=1 */
  const masonryItems = $derived(
    sm.roomState.rooms.map((room) => ({
      component: RoomCard,
      props: {
        room,
        isSelected: sm.roomState.selectedRoom?.id === room.id,
        onEdit: () => openEdit(room),
        onDelete: () => sm.deleteRoom(room.id),
        onShare: () => openShare(room),
      },
    }))
  );
</script>

<div class="room-page">
  {#if isConnected && selectedRoom}
    <!-- 顶部当前房间卡：Card margin(16,16,16,8) r8，hover 边框 primary 1px -->
    <div
      class="md-card current-card"
      class:hovered={currentHovered}
      onmouseenter={() => (currentHovered = true)}
      onmouseleave={() => (currentHovered = false)}
    >
      <div
        class="current-tile"
        role="button"
        tabindex="0"
        onclick={() => copyRoomShareLink(selectedRoom, { linkOnly: true })}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            copyRoomShareLink(selectedRoom, { linkOnly: true });
          }
        }}
      >
        <div class="tile-title">当前房间: {selectedRoom.name}</div>
        <div class="tile-subtitle">连接状态: {statusText}</div>
      </div>
    </div>
  {/if}

  <div class="page-content">
    {#if !isIdle}
      <!-- 已连接/连接中：用户页 -->
      <div class="user-page-host">
        <UserPage />
      </div>
    {:else}
      <!-- 未连接：房间瀑布流（SliverPadding 16 + SliverMasonryGrid 间距 8） -->
      <div class="rooms-scroll">
        <div class="rooms-padding">
          <MasonryGrid {columns} gap={8} items={masonryItems} />
          <div class="bottom-spacer"></div>
        </div>
      </div>
    {/if}
  </div>

  {#if isIdle}
    <!-- FAB 行：sort + 16 + paste + 16 + add（右下角固定） -->
    <div class="fab-row">
      <button class="md-fab" data-tip="排序" onclick={() => (reorderOpen = true)}>
        <Icon name="sort" />
      </button>
      <button class="md-fab" onclick={() => (pasteOpen = true)}>
        <Icon name="paste" />
      </button>
      <button class="md-fab" onclick={() => (addOpen = true)}>
        <Icon name="add" />
      </button>
    </div>
  {/if}

  <!-- 弹窗集 -->
  <AddRoomDialog bind:open={addOpen} />
  <EditRoomDialog bind:open={editOpen} room={editTarget} />
  <PasteImportDialog bind:open={pasteOpen} />
  <ShareExportDialog bind:open={shareOpen} room={shareTarget} />
  <RoomReorderSheet bind:open={reorderOpen} rooms={sm.roomState.rooms} />
</div>

<style>
  /* IndexedStack 页面根：height 100%，内容滚动 */
  .room-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
    overflow: hidden;
  }

  /* 当前房间卡 */
  .current-card {
    margin: 16px 16px 8px;
    border-radius: 8px;
    border: 1px solid transparent;
    flex-shrink: 0;
  }
  .current-card.hovered {
    border-color: var(--md-primary);
  }
  /* ListTile contentPadding h16 v8 */
  .current-tile {
    padding: 8px 16px;
    cursor: pointer;
  }
  .tile-title {
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
  }
  .tile-subtitle {
    margin-top: 2px;
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
  }

  .page-content {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .user-page-host {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .rooms-scroll {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .rooms-padding {
    padding: 16px;
  }
  /* 尾部 spacer：safeBottom + 20 */
  .bottom-spacer {
    height: calc(20px + env(safe-area-inset-bottom));
  }

  /* FAB：Scaffold floatingActionButton 右下角 */
  .fab-row {
    position: absolute;
    right: 16px;
    bottom: 16px;
    display: flex;
    gap: 16px;
    z-index: 10;
  }
</style>
