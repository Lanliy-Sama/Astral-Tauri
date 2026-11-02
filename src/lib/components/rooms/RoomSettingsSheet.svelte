<script>
  /**
   * 房间设置弹窗 —— room_settings_sheet.dart 1:1 移植。
   * >600px：Dialog 400×600（surfaceContainerHigh, r28）；否则底部抽屉（surface, 顶圆角 28，
   * 初始 50% 高，可拖拽头部在 40%-80% 间调整 —— DraggableScrollableSheet 0.5/0.4/0.8 的简化）。
   * 内容：标题区 + 4 区选项按钮组（显示模式/用户显示/用户排序/排序方式），点击实时生效。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import {
    UserListStyle,
    UserDisplayMode,
    UserSortOption,
    UserSortOrder,
  } from '$lib/stores/enums.js';

  let { open = $bindable(false) } = $props();

  let windowWidth = $state(window.innerWidth);
  $effect(() => {
    const onResize = () => (windowWidth = window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });
  const isDesktop = $derived(windowWidth > 600);

  /* 底部抽屉拖拽（头部区域作为拖拽手柄） */
  let sheetPercent = $state(0.5);
  let sheetDrag = null;
  function onSheetPointerDown(e) {
    sheetDrag = { startY: e.clientY, startPercent: sheetPercent };
    e.currentTarget.setPointerCapture(e.pointerId);
  }
  function onSheetPointerMove(e) {
    if (!sheetDrag) return;
    const next =
      sheetDrag.startPercent - (e.clientY - sheetDrag.startY) / window.innerHeight;
    sheetPercent = Math.min(0.8, Math.max(0.4, next));
  }
  function onSheetPointerUp() {
    sheetDrag = null;
  }

  function close() {
    open = false;
  }
  function onKeydown(e) {
    if (e.key === 'Escape' && open) close();
  }

  /* 四区设置（Watch 内监听 displayState，实时高亮） */
  const sections = $derived([
    {
      title: '显示模式',
      options: [
        {
          label: '简约',
          selected: sm.displayState.userListStyle === UserListStyle.simple,
          onSelect: () => sm.setUserListStyle(UserListStyle.simple),
        },
        {
          label: '详细',
          selected: sm.displayState.userListStyle === UserListStyle.detailed,
          onSelect: () => sm.setUserListStyle(UserListStyle.detailed),
        },
        {
          label: '列表',
          selected: sm.displayState.userListStyle === UserListStyle.list,
          onSelect: () => sm.setUserListStyle(UserListStyle.list),
        },
      ],
    },
    {
      title: '用户显示',
      options: [
        {
          label: '默认',
          selected: sm.displayState.displayMode === UserDisplayMode.all,
          onSelect: () => sm.setDisplayMode(UserDisplayMode.all),
        },
        {
          label: '用户',
          selected: sm.displayState.displayMode === UserDisplayMode.users,
          onSelect: () => sm.setDisplayMode(UserDisplayMode.users),
        },
        {
          label: '服务器',
          selected: sm.displayState.displayMode === UserDisplayMode.servers,
          onSelect: () => sm.setDisplayMode(UserDisplayMode.servers),
        },
      ],
    },
    {
      title: '用户排序',
      options: [
        {
          label: '默认',
          selected: sm.displayState.sortOption === UserSortOption.none,
          onSelect: () => sm.setSortOption(UserSortOption.none),
        },
        {
          label: '延迟',
          selected: sm.displayState.sortOption === UserSortOption.latency,
          onSelect: () => sm.setSortOption(UserSortOption.latency),
        },
        {
          label: '用户名',
          selected: sm.displayState.sortOption === UserSortOption.nameLength,
          onSelect: () => sm.setSortOption(UserSortOption.nameLength),
        },
      ],
    },
    {
      title: '排序方式',
      options: [
        {
          label: '升序',
          selected: sm.displayState.sortOrder === UserSortOrder.ascending,
          onSelect: () => sm.setSortOrder(UserSortOrder.ascending),
        },
        {
          label: '降序',
          selected: sm.displayState.sortOrder === UserSortOrder.descending,
          onSelect: () => sm.setSortOrder(UserSortOrder.descending),
        },
      ],
    },
  ]);
</script>

<svelte:window onkeydown={onKeydown} />

{#snippet sheetHeader()}
  <!-- 标题区 Padding(24,14,8,4)；Column 默认 crossAxisAlignment.center + 贪婪 ListView
       ⇒ 标题块整体居中（宽度随内容） -->
  <div class="sheet-header">
    <div class="title-row">
      <Icon name="bar_chart_outlined" size={20} color="var(--md-primary)" />
      <span class="title">房间设置</span>
    </div>
    <div class="subtitle">点击下方设置项进行配置，所有更改将实时生效</div>
  </div>
{/snippet}

{#snippet sheetBody()}
  <!-- ListView(padding: h16) -->
  <div class="sheet-body">
    {#each sections as section (section.title)}
      <div class="section">
        <div class="section-title">{section.title}</div>
        <div class="opts">
          {#each section.options as opt (opt.label)}
            <button class="opt" class:selected={opt.selected} onclick={opt.onSelect}>
              {opt.label}
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>
{/snippet}

{#if open}
  {#if isDesktop}
    <!-- PC 端：Dialog maxW400 maxH600（Column 主轴撑满 → 固定 600 高） -->
    <div
      class="md-dialog-scrim settings-scrim"
      onclick={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div class="settings-dialog" role="dialog">
        {@render sheetHeader()}
        {@render sheetBody()}
      </div>
    </div>
  {:else}
    <!-- 移动端：底部抽屉 r28，初始 50% 高 -->
    <div
      class="sheet-scrim"
      onclick={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    ></div>
    <div class="bottom-sheet" role="dialog" style="height:{sheetPercent * 100}%">
      <div
        class="drag-area"
        onpointerdown={onSheetPointerDown}
        onpointermove={onSheetPointerMove}
        onpointerup={onSheetPointerUp}
        onpointercancel={onSheetPointerUp}
      >
        {@render sheetHeader()}
      </div>
      {@render sheetBody()}
    </div>
  {/if}
{/if}

<style>
  /* ---- PC 端 Dialog ---- */
  .settings-dialog {
    width: 400px;
    height: 600px;
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 48px);
    background: var(--md-surface-container-high);
    border-radius: 28px;
    box-shadow:
      0 3px 3px rgba(0, 0, 0, 0.3),
      0 12px 24px 6px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  /* ---- 移动端底部抽屉 ---- */
  .sheet-scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.32);
    z-index: 1000;
  }
  .bottom-sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--md-surface);
    border-radius: 28px 28px 0 0;
    z-index: 1001;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: sheet-up 250ms cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes sheet-up {
    from {
      transform: translateY(100%);
    }
    to {
      transform: translateY(0);
    }
  }
  .drag-area {
    cursor: ns-resize;
    flex-shrink: 0;
    touch-action: none;
  }

  /* ---- 共用内容 ---- */
  .sheet-header {
    align-self: center;
    width: fit-content;
    padding: 14px 8px 4px 24px;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title {
    font-size: 16px;
    line-height: 24px;
    font-weight: 500;
    color: var(--md-primary);
  }
  .subtitle {
    margin-top: 4px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
  }

  .sheet-body {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 0 16px;
  }
  .section-title {
    padding: 16px 8px 8px 8px;
    font-size: 14px;
    font-weight: 700;
    color: var(--md-primary);
    text-align: left;
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  /* IntrinsicWidth + SizedBox(h32) + TextButton(padding h30, r5, 文本 13) */
  .opt {
    height: 32px;
    padding: 0 30px;
    border-radius: 5px;
    border: 1px solid var(--md-outline);
    background: var(--md-surface-container-highest);
    color: var(--md-on-surface-variant);
    font-size: 13px;
    letter-spacing: 0.1px;
    line-height: 20px;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 120ms ease;
  }
  .opt:hover {
    background: color-mix(in srgb, var(--md-surface-container-highest) 92%, var(--md-on-surface-variant));
  }
  .opt.selected {
    background: var(--md-primary);
    color: var(--md-on-primary);
    border-color: var(--md-primary);
  }
  .opt.selected:hover {
    background: color-mix(in srgb, var(--md-primary) 92%, var(--md-on-primary));
  }
</style>
