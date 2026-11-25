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
  import MusicPlatformPage from '$lib/components/settings/MusicPlatformPage.svelte';
  import { pushPage } from '$lib/router/PageRouter.svelte';
  import { roomSocial } from '$lib/services/RoomSocial.svelte';
  import { roomConnections } from '$lib/services/RoomConnections.svelte';
  import { t } from '$lib/i18n/I18n.svelte';

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

  $effect(() => {
    const room = isConnected ? selectedRoom : null;
    if (room) roomSocial.start(room);
    else if (roomSocial.connected) roomSocial.stop();
  });

  $effect(() => {
    const nodes = sm.connectionState.netStatus?.nodes || [];
    if (roomSocial.connected) roomSocial.setNodes(nodes);
  });

  const statusText = $derived(
    isConnected ? t('connected_share_hint') : connection === 'connecting' ? t('connecting') : t('not_connected')
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

  async function joinRoom(room) {
    await roomConnections.connectRoom(room);
  }

  async function deleteRoom(room) {
    await roomConnections.disconnectRoom(room.id);
    await sm.deleteRoom(room.id);
  }

  /* 瀑布流 items：RoomCard span=1 */
  const masonryItems = $derived(
    sm.roomState.rooms.map((room) => ({
      component: RoomCard,
      props: {
        room,
        isSelected: sm.roomState.selectedRoom?.id === room.id,
        onEdit: () => openEdit(room),
        onDelete: () => deleteRoom(room),
        onShare: () => openShare(room),
        onJoin: () => joinRoom(room),
      },
    }))
  );
</script>

<div class="room-page">
  {#if roomConnections.tabs.length}
    <div class="room-tabs" role="tablist" aria-label={t('joined_rooms')}>
      {#each roomConnections.tabs as session (session.room.id)}
        <div class="room-tab" class:active={roomConnections.activeRoomId === session.room.id}>
          <button
            class="tab-select"
            role="tab"
            aria-selected={roomConnections.activeRoomId === session.room.id}
            onclick={() => roomConnections.setActiveRoom(session.room.id)}
          >
            <span class="status-dot {session.status}"></span>
            <span class="tab-name">{session.room.name}</span>
            <span class="tab-state">{session.status === 'connected' ? t('connected') : session.status === 'connecting' ? t('connecting') : t('connection_failed')}</span>
          </button>
          <button class="tab-close" aria-label={t('leave_room', { name: session.room.name })} onclick={() => roomConnections.disconnectRoom(session.room.id)}>
            <Icon name="close" size={16} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

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
        <div class="tile-title">{t('current_room', { name: selectedRoom.name })}</div>
        <div class="tile-subtitle">{t('connection_status', { status: statusText })}</div>
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

  <button
    class="md-fab music-fab"
    class:connected={!isIdle}
    data-tip={t('room_music')}
    onclick={() => pushPage(MusicPlatformPage)}
  >
    <Icon name="album" />
  </button>

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

  .room-tabs {
    display: flex;
    flex-shrink: 0;
    gap: 8px;
    margin: 12px 16px 0;
    padding: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
  }
  .room-tab {
    display: flex;
    align-items: center;
    flex: 0 0 auto;
    max-width: 250px;
    border: 1px solid var(--md-outline-variant);
    border-radius: 16px;
    background: var(--md-surface-container-low);
    transition: border-color 160ms ease, background 160ms ease;
  }
  .room-tab.active {
    border-color: var(--md-primary);
    background: var(--md-primary-06);
  }
  .tab-select {
    min-width: 0;
    height: 42px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 4px 0 12px;
    border: 0;
    background: transparent;
    color: var(--md-on-surface);
    cursor: pointer;
  }
  .status-dot { width: 8px; height: 8px; flex-shrink: 0; border-radius: 50%; background: var(--md-outline); }
  .status-dot.connected { background: var(--flutter-green); }
  .status-dot.connecting { background: var(--flutter-orange); animation: tab-pulse 1s ease-in-out infinite; }
  .status-dot.error { background: var(--md-error); }
  .tab-name { max-width: 112px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; font-weight: 600; }
  .tab-state { color: var(--md-on-surface-variant); font-size: 10px; white-space: nowrap; }
  .tab-close { width: 30px; height: 30px; display: grid; place-items: center; margin-right: 4px; border: 0; border-radius: 50%; background: transparent; color: var(--md-on-surface-variant); cursor: pointer; }
  .tab-close:hover { background: var(--md-on-surface-08); color: var(--md-error); }
  @keyframes tab-pulse { 50% { opacity: 0.35; } }
  @media (prefers-reduced-motion: reduce) {
    .status-dot.connecting { animation: none; }
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

  .music-fab {
    position: absolute;
    right: 224px;
    bottom: 16px;
    z-index: 11;
  }
  .music-fab.connected {
    right: 88px;
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
