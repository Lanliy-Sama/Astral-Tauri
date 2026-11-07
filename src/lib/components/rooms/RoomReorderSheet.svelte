<script>
  /**
   * 房间排序弹窗 —— 翻译自 features/rooms/widgets/room_reorder_sheet.dart。
   * >600px：Dialog 400×600 居中；否则底部抽屉（高 70%，顶圆角 28）。
   * 排序：HTML5 drag & drop，整卡可拖（对应 ReorderableDragStartListener 包裹整卡）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';

  let { open = $bindable(false), rooms = [] } = $props();

  let localRooms = $state([]);
  let dragIndex = $state(null);
  let windowWidth = $state(window.innerWidth);

  $effect(() => {
    const onResize = () => (windowWidth = window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  const isWide = $derived(windowWidth > 600);

  /* 打开时快照（对应 List.from(rooms)），仅从关闭 → 打开的沿触发 */
  let wasOpen = false;
  $effect(() => {
    const nowOpen = open;
    if (nowOpen && !wasOpen) {
      localRooms = [...rooms];
      dragIndex = null;
    }
    wasOpen = nowOpen;
  });

  function onDragStart(e, i) {
    dragIndex = i;
    e.dataTransfer.effectAllowed = 'move';
    try {
      e.dataTransfer.setData('text/plain', String(i));
    } catch {
      /* 某些环境不允许 setData */
    }
  }

  function onDragOver(e, i) {
    if (dragIndex === null) return;
    e.preventDefault();
    if (dragIndex === i) return;
    const list = [...localRooms];
    const [item] = list.splice(dragIndex, 1);
    list.splice(i, 0, item);
    localRooms = list;
    dragIndex = i;
  }

  function onDrop(e) {
    e.preventDefault();
    dragIndex = null;
  }

  /* onReorder：确认后写入 sm.reorderRooms 并关闭 */
  function confirmReorder() {
    sm.reorderRooms(localRooms);
    open = false;
  }

  function onKeydown(e) {
    if (e.key === 'Escape' && open) open = false;
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#snippet sheetBody()}
  <div class="sheet-inner">
    <!-- 标题区 Padding(24,14,8,16) -->
    <div class="sheet-header">
      <div class="title-row">
        <Icon name="sort" size={24} color="var(--md-primary)" />
        <span class="title">房间排序</span>
      </div>
      <div class="subtitle">拖拽房间卡片来调整显示顺序</div>
    </div>

    <!-- 列表 Padding(0,16,16,16) -->
    <div
      class="list-area"
      ondragover={(e) => {
        if (dragIndex !== null) e.preventDefault();
      }}
      ondrop={onDrop}
    >
      {#each localRooms as room, i (room.id ?? `idx-${i}`)}
        <div
          class="reorder-item"
          class:dragging={dragIndex === i}
          draggable="true"
          ondragstart={(e) => onDragStart(e, i)}
          ondragover={(e) => onDragOver(e, i)}
          ondrop={onDrop}
          ondragend={() => (dragIndex = null)}
        >
          <div class="item-tile">
            <div class="tile-content">
              <div class="tile-title">{room.name}</div>
              <div class="tile-subtitle">{room.encrypted ? '加密房间' : '开放房间'}</div>
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- 底部按钮 Padding(24) -->
    <div class="sheet-actions">
      <button class="md-btn md-outlined-btn sheet-btn" onclick={() => (open = false)}>
        取消
      </button>
      <button class="md-btn md-filled-btn sheet-btn" onclick={confirmReorder}>确认</button>
    </div>
  </div>
{/snippet}

{#if open}
  {#if isWide}
    <!-- PC 端：Dialog 400×600 -->
    <div
      class="md-dialog-scrim reorder-scrim"
      onclick={(e) => {
        if (e.target === e.currentTarget) open = false;
      }}
    >
      <div class="reorder-dialog" role="dialog">
        {@render sheetBody()}
      </div>
    </div>
  {:else}
    <!-- 移动端：底部抽屉（初始 70% 高，顶圆角 28） -->
    <div
      class="sheet-scrim"
      onclick={(e) => {
        if (e.target === e.currentTarget) open = false;
      }}
    ></div>
    <div class="reorder-sheet" role="dialog">
      {@render sheetBody()}
    </div>
  {/if}
{/if}

<style>
  /* ---- PC：Dialog 400×600 ---- */
  .reorder-dialog {
    width: 400px;
    height: 600px;
    max-width: 90vw;
    max-height: 90vh;
    background: var(--md-surface-container-high);
    border-radius: 28px;
    box-shadow:
      0 3px 3px rgba(0, 0, 0, 0.3),
      0 12px 24px 6px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* ---- 移动端：底部抽屉 ---- */
  .sheet-scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.32);
    z-index: 1000;
  }
  .reorder-sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    height: 70vh;
    background: var(--md-surface);
    border-radius: 28px 28px 0 0;
    z-index: 1001;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .sheet-inner {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .sheet-header {
    padding: 14px 8px 16px 24px;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title {
    font-size: 18px;
    line-height: 24px;
    font-weight: 500;
    color: var(--md-primary);
  }
  .subtitle {
    margin-top: 4px;
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
  }

  .list-area {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 16px 16px;
  }

  /* 列表项：Padding(v4) + r16 surfaceContainerHighest + hover 边框 primary 1.5 */
  .reorder-item {
    padding: 4px 0;
  }
  .reorder-item.dragging {
    opacity: 0.6;
  }
  .item-tile {
    background: var(--md-surface-container-highest);
    border: 1.5px solid transparent;
    border-radius: 16px;
    transition: border-color 100ms ease;
    cursor: grab;
  }
  .reorder-item:hover .item-tile {
    border-color: var(--md-primary);
  }
  .reorder-item.dragging .item-tile {
    cursor: grabbing;
  }
  /* ListTile contentPadding h12（两行 72 高） */
  .tile-content {
    min-height: 72px;
    padding: 0 12px;
    display: flex;
    flex-direction: column;
    justify-content: center;
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

  /* 底部按钮：Expanded + v16 padding（高 52）+ r12 */
  .sheet-actions {
    display: flex;
    gap: 16px;
    padding: 24px;
  }
  .sheet-btn {
    flex: 1 1 0;
    height: 52px;
    border-radius: 12px;
    padding: 0;
  }
</style>
