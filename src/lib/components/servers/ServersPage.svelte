<script>
  /**
   * 服务器管理页（底栏独立入口）—— 对应 lib/features/servers/pages/server_page.dart。
   * BaseStatefulSettingsPage(showAppBar: false)：无 AppBar + FAB 添加；
   * ReorderableListView（整卡拖拽、实时预览排序）；30s 服务器状态轮询。
   */
  import { onMount } from 'svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import ServerDialog from './dialogs/ServerDialog.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { ServerStatus } from '$lib/stores/enums.js';
  import { isBlockedServer } from '$lib/utils/network.js';
  import { snackInfo } from '$lib/components/common/snackbars.svelte.js';

  /* ---- 添加 / 编辑 / 删除对话框 ---- */
  let dialogOpen = $state(false);
  let dialogServer = $state(null); // null = 添加模式
  let deleteOpen = $state(false);
  let deleteTarget = $state(null);

  /* ---- PopupMenuButton（编辑 / 删除） ---- */
  let menuOpen = $state(false);
  let menuServer = $state(null);
  let menuPos = $state({ x: 0, y: 0 });
  let menuEl = $state(null);

  const servers = $derived(sm.serverState.servers);
  const statuses = $derived(sm.serverStatusState.serverStatuses);

  /* initState postFrame：启动 30s 周期检测；dispose：停止 */
  onMount(() => {
    sm.startServerStatusCheck();
    return () => sm.stopServerStatusCheck();
  });

  /* ServerStatusStyle.color：online green / offline red / inUse blue / unknown outline */
  function statusColor(status) {
    switch (status) {
      case ServerStatus.online:
        return 'var(--flutter-green)';
      case ServerStatus.offline:
        return 'var(--flutter-red)';
      case ServerStatus.inUse:
        return 'var(--flutter-blue)';
      default:
        return 'var(--md-outline)';
    }
  }

  function openAdd() {
    dialogServer = null;
    dialogOpen = true;
  }

  function openEdit(server) {
    dialogServer = server;
    dialogOpen = true;
  }

  /* ---- 弹出菜单（右对齐锚点按钮下方） ---- */
  function toggleMenu(e, server) {
    if (menuOpen && menuServer === server) {
      menuOpen = false;
      return;
    }
    const rect = e.currentTarget.getBoundingClientRect();
    const estH = 2 * 48 + 8;
    let top = rect.bottom + 4;
    if (top + estH > window.innerHeight) top = Math.max(8, rect.top - estH - 4);
    menuPos = { x: rect.right, y: top };
    menuServer = server;
    menuOpen = true;
  }

  function onMenuSelect(value) {
    menuOpen = false;
    const server = menuServer;
    if (value === 'edit') {
      if (isBlockedServer(server.url)) {
        snackInfo('不可编辑', '此服务器不可编辑', { duration: 2000 });
      } else {
        openEdit(server);
      }
    } else if (value === 'delete') {
      deleteTarget = server;
      deleteOpen = true;
    }
  }

  function confirmDelete() {
    sm.deleteServer(deleteTarget);
    deleteOpen = false;
  }

  /* ---- 拖拽排序（ReorderableDragStartListener 整卡拖拽 + 实时预览） ---- */
  let dragIndex = $state(null);
  let previewOrder = $state(null);
  let dropped = false;

  const displayServers = $derived(previewOrder ?? servers);

  function onDragStart(e, index) {
    dragIndex = index;
    dropped = false;
    previewOrder = [...displayServers];
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', String(index));
  }

  /* insertPos：拖拽点之前要插入的卡片序号（按当前预览顺序），转换为 removeAt/insert 语义 */
  function applyPreview(insertPos) {
    if (dragIndex == null || insertPos == null) return;
    const cur = [...displayServers];
    let target = insertPos > dragIndex ? insertPos - 1 : insertPos;
    if (target < 0) target = 0;
    if (target === dragIndex) return;
    const [item] = cur.splice(dragIndex, 1);
    cur.splice(target, 0, item);
    previewOrder = cur;
    dragIndex = target;
  }

  function onListDragOver(e) {
    if (dragIndex == null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    const cards = Array.from(e.currentTarget.querySelectorAll('.server-card'));
    let insertPos = cards.length;
    for (let i = 0; i < cards.length; i++) {
      const r = cards[i].getBoundingClientRect();
      if (e.clientY < r.top + r.height / 2) {
        insertPos = i;
        break;
      }
    }
    applyPreview(insertPos);
  }

  function onListDrop(e) {
    if (dragIndex == null) return;
    e.preventDefault();
    dropped = true;
  }

  function onDragEnd() {
    if (dragIndex == null) return;
    if (dropped && previewOrder) sm.reorderServers(previewOrder);
    dragIndex = null;
    previewOrder = null;
    dropped = false;
  }
</script>

<svelte:window
  onclick={(e) => {
    if (
      menuOpen &&
      menuEl &&
      !menuEl.contains(e.target) &&
      !(e.target.closest && e.target.closest('.menu-trigger'))
    ) {
      menuOpen = false;
    }
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape' && menuOpen) menuOpen = false;
  }}
/>

<div class="servers-page">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="servers-scroll" ondragover={onListDragOver} ondrop={onListDrop}>
    {#if servers.length === 0}
      <!-- buildEmptyState -->
      <div class="empty-wrap">
        <div class="empty-col">
          <Icon name="dns_outlined" size={64} color="var(--md-outline)" />
          <div class="gap16"></div>
          <div class="md-title-medium" style="color:var(--md-outline)">暂无服务器</div>
          <div class="gap16"></div>
          <button class="md-btn md-elevated-btn" onclick={openAdd}>
            <Icon name="add" size={18} />添加服务器
          </button>
        </div>
      </div>
    {:else}
      <div class="server-list">
        {#each displayServers as server, index (server.id)}
          {@const blocked = isBlockedServer(server.url)}
          {@const status = statuses[server.id] ?? ServerStatus.unknown}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="md-card server-card {index === dragIndex ? 'dragging' : ''}"
            draggable="true"
            ondragstart={(e) => onDragStart(e, index)}
            ondragend={onDragEnd}
          >
            <div class="md-list-tile two-line no-cursor server-tile">
              <span class="tile-leading">
                <span class="status-bar" style="background:{statusColor(status)}"></span>
              </span>
              <span class="tile-content">
                <span class="tile-title" style="font-weight:600">{server.name}</span>
                <span class="tile-subtitle server-url">{blocked ? '***' : server.url}</span>
              </span>
              <span class="tile-trailing" style="gap:4px">
                <Switch
                  scale
                  value={server.enable}
                  onChange={(v) => sm.setServerEnable(server, v)}
                />
                <button class="md-icon-btn menu-trigger" onclick={(e) => toggleMenu(e, server)}>
                  <Icon name="more_vert" size={24} color="var(--md-on-surface-variant)" />
                </button>
              </span>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- FloatingActionButton：右下角固定 -->
  <button class="md-fab servers-fab" onclick={openAdd} aria-label="添加服务器">
    <Icon name="add" size={24} />
  </button>

  <!-- PopupMenuButton：编辑 / 删除 -->
  {#if menuOpen}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="pop-anchor" style="left:{menuPos.x}px; top:{menuPos.y}px">
      <div class="md-menu pop-menu" bind:this={menuEl}>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="md-menu-item" onclick={() => onMenuSelect('edit')}>
          <!-- Icons.edit_outlined / Icons.delete_outline：无 Material Symbols 连字名，用基名字形 + FILL 0 -->
          <span
            class="ms"
            style="font-size:18px; color:{menuServer && isBlockedServer(menuServer.url) ? 'var(--md-outline)' : 'var(--md-primary)'}">edit</span>
          <span
            style="font-size:16px; line-height:24px; {menuServer && isBlockedServer(menuServer.url)
              ? 'color:var(--md-outline)'
              : ''}">编辑</span>
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="md-menu-item" onclick={() => onMenuSelect('delete')}>
          <span class="ms" style="font-size:18px; color:var(--md-error)">delete</span>
          <span style="font-size:16px; line-height:24px; color:var(--md-error)">删除</span>
        </div>
      </div>
    </div>
  {/if}

  <!-- 添加 / 编辑服务器 -->
  <ServerDialog bind:open={dialogOpen} server={dialogServer} />

  <!-- 删除确认 -->
  <Dialog bind:open={deleteOpen} title="删除服务器">
    <div>
      {deleteTarget ? `确定要删除服务器 "${deleteTarget.name}" 吗？此操作不可撤销。` : ''}
    </div>
    {#snippet actions()}
      <button class="md-btn md-text-btn" onclick={() => (deleteOpen = false)}>取消</button>
      <button class="md-btn md-text-btn" style="color:var(--flutter-red)" onclick={confirmDelete}>
        删除
      </button>
    {/snippet}
  </Dialog>
</div>

<style>
  .servers-page {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .servers-scroll {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  /* ReorderableListView padding 12 */
  .server-list {
    padding: 12px;
  }
  .server-card {
    margin-bottom: 8px;
  }
  .server-card.dragging {
    opacity: 0.35;
  }
  /* ListTile 无 onTap → 无水波/hover */
  .server-card .md-list-tile:hover,
  .server-card .md-list-tile:active {
    background: transparent;
  }
  /* leading 为 4px 状态色条（贴左缘，非 24px 图标槽位） */
  .server-card .md-list-tile .tile-leading {
    width: auto;
    justify-content: flex-start;
  }
  .status-bar {
    width: 4px;
    height: 40px;
    border-radius: 2px;
    display: block;
  }
  .server-url {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* 空态：Center 居中 */
  .empty-wrap {
    min-height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .empty-col {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .gap16 {
    height: 16px;
  }
  /* Scaffold FAB：右下角固定（内容滚动，FAB 不动） */
  .servers-fab {
    position: absolute;
    right: 16px;
    bottom: 16px;
    z-index: 20;
  }
  /* PopupMenu：fixed 锚点 + translateX(-100%) 右对齐 */
  .pop-anchor {
    position: fixed;
    transform: translateX(-100%);
    z-index: 1200;
  }
  .pop-anchor .pop-menu {
    position: static;
  }
</style>
