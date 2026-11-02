<script>
  /**
   * 房间选择对话框 —— 对照 lib/features/home/widgets/canvas_jump.dart（CanvasJump.show）。
   * 原版通过 showDialog 弹出；此处改为 bind:open + onselect(room) 的组件式调用。
   */
  import Dialog from '../common/Dialog.svelte';
  import TextField from '../common/TextField.svelte';
  import Icon from '../common/Icon.svelte';

  let { open = $bindable(false), rooms = [], onselect = () => {} } = $props();

  let searchQuery = $state('');
  let hoveredName = $state('');
  let isScrolling = $state(false);
  let winW = $state(window.innerWidth);
  let winH = $state(window.innerHeight);
  let listEl = $state(null);
  let thumbTop = $state(0);
  let thumbHeight = $state(0);
  let scrollTimer = null;

  const filteredRooms = $derived(
    (rooms || []).filter((room) =>
      (room.name || '').toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
  /* 内容宽 = 屏宽/1.2（外加左右 contentPadding 16×2），高 = 屏高/2 + 12 */
  const dialogW = $derived(Math.min(winW / 1.2 + 32, winW * 0.9));
  const bodyW = $derived(dialogW - 32);
  const bodyH = $derived(winH / 2 + 12);

  $effect(() => {
    const onResize = () => {
      winW = window.innerWidth;
      winH = window.innerHeight;
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  /* 打开时重置（对应原版每次 showDialog 重建 State） */
  $effect(() => {
    if (open) {
      searchQuery = '';
      hoveredName = '';
      isScrolling = false;
      requestAnimationFrame(() => {
        if (listEl) listEl.scrollTop = 0;
        updateThumb();
      });
    }
  });

  /* 列表内容变化后重算滚动条 */
  $effect(() => {
    void filteredRooms.length;
    requestAnimationFrame(updateThumb);
  });

  function updateThumb() {
    if (!listEl) {
      thumbHeight = 0;
      return;
    }
    const el = listEl;
    const total = el.scrollHeight;
    const view = el.clientHeight;
    if (total <= view || view <= 0) {
      thumbHeight = 0;
      return;
    }
    const h = Math.max((view / total) * view, 36);
    const maxTop = view - h;
    const t = (el.scrollTop / (total - view)) * maxTop;
    thumbHeight = h;
    thumbTop = Math.max(0, Math.min(maxTop, t));
  }

  function onScroll() {
    if (!isScrolling) isScrolling = true;
    if (scrollTimer) clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => {
      isScrolling = false;
    }, 300);
    updateThumb();
  }

  function selectRoom(room) {
    open = false;
    onselect(room);
  }
</script>

<div class="cj-wrap">
  <Dialog bind:open width={dialogW}>
    <div class="cj-root">
      <div class="cj-title">
        <Icon name="meeting_room" size={24} color="var(--md-primary)" />
        <span class="cj-title-text">选择房间</span>
      </div>
      <div class="cj-body" style="width:{bodyW}px; height:{bodyH}px">
        <div class="cj-search">
          <TextField bind:value={searchQuery} placeholder="搜索房间" prefixIcon="search" />
        </div>
        <div class="cj-list-outer">
          <div class="cj-scroll-area">
            <div class="cj-list" bind:this={listEl} onscroll={onScroll}>
              {#each filteredRooms as room, index (room.id ?? index)}
                <div class="cj-item-pad">
                  <div
                    class="cj-item"
                    class:hover={hoveredName === room.name}
                    onmouseenter={() => (hoveredName = room.name)}
                    onmouseleave={() => {
                      if (hoveredName === room.name) hoveredName = '';
                    }}
                    onclick={() => selectRoom(room)}
                    onkeydown={(e) => {
                      if (e.key === 'Enter') selectRoom(room);
                    }}
                    role="button"
                    tabindex="0"
                  >
                    <div class="cj-tile">
                      <div class="cj-tile-content">
                        <div class="cj-tile-title">{room.name}</div>
                        <div class="cj-tile-sub">{room.encrypted ? '加密房间' : '开放房间'}</div>
                      </div>
                      <span class="cj-chevron"><Icon name="chevron_right" size={24} /></span>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
            <div class="cj-scrollbar" class:visible={isScrolling && thumbHeight > 0}>
              <div class="cj-thumb" style="top:{thumbTop}px; height:{thumbHeight}px"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
    {#snippet actions()}
      <button class="md-btn md-text-btn cj-cancel" onclick={() => (open = false)}>取消</button>
    {/snippet}
  </Dialog>
</div>

<style>
  /* AlertDialog r16 + title/content/actionsPadding 覆盖 */
  .cj-wrap :global(.md-dialog) {
    border-radius: 16px;
  }
  .cj-wrap :global(.md-dialog .dialog-content) {
    padding: 0;
  }
  .cj-wrap :global(.md-dialog .dialog-actions) {
    padding: 16px;
  }

  .cj-title {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 16px 0;
  }
  .cj-title-text {
    font-size: 18px;
    color: var(--md-primary);
  }
  .cj-body {
    margin: 8px 16px;
    display: flex;
    flex-direction: column;
  }

  /* 搜索框：contentPadding v12 */
  .cj-search {
    padding: 0 8px;
  }
  .cj-search :global(.md-text-field) {
    min-height: 50px;
  }
  .cj-search :global(.md-text-field input) {
    padding: 12px 0;
  }
  .cj-search :global(.md-text-field input::placeholder) {
    color: var(--md-on-surface-variant);
    opacity: 1;
  }

  .cj-list-outer {
    flex: 1 1 0;
    min-height: 0;
    padding: 8px;
  }
  .cj-scroll-area {
    position: relative;
    height: 100%;
    border-radius: 16px;
    overflow: hidden;
  }
  .cj-list {
    height: 100%;
    overflow-y: auto;
  }

  .cj-item-pad {
    padding: 4px 0;
  }
  .cj-item {
    border-radius: 16px;
    border: 1.5px solid transparent;
    background: var(--md-surface-container-highest-095);
    cursor: pointer;
  }
  :global([data-theme-mode='dark']) .cj-item {
    background: var(--md-surface-container-highest-015);
  }
  .cj-item.hover {
    background: var(--md-primary-container-012);
    border-color: var(--md-primary);
  }
  .cj-tile {
    display: flex;
    align-items: center;
    padding: 0 16px;
    min-height: 72px;
  }
  .cj-tile-content {
    flex: 1 1 0;
    min-width: 0;
  }
  .cj-tile-title {
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cj-tile-sub {
    margin-top: 2px;
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
  }
  .cj-chevron {
    display: inline-flex;
    color: var(--md-primary);
    opacity: 0.59; /* primary.withAlpha(150) */
  }

  /* 滚动条：滚动时淡入，300ms 无滚动后淡出（thickness 11, radius 16） */
  .cj-scrollbar {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 11px;
    opacity: 0;
    transition: opacity 200ms ease;
    pointer-events: none;
  }
  .cj-scrollbar.visible {
    opacity: 1;
  }
  .cj-thumb {
    position: absolute;
    left: 0;
    right: 0;
    border-radius: 5.5px;
    background: color-mix(in srgb, var(--md-on-surface) 40%, transparent);
  }

  .cj-cancel {
    font-size: 16px;
  }
</style>
