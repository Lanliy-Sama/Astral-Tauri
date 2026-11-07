<script>
  /**
   * 房间卡片 —— 翻译自 lib/features/rooms/widgets/room_card.dart。
   * 选中态背景 = HSLColor.fromColor(primary).withLightness(0.95亮 / 0.10暗)。
   * 边框：(selected||hovered) ? primary : transparent，宽 (hover&&!selected) ? 1 : 2。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { buildSchemes } from '$lib/theme/Material3.svelte';

  let {
    room,
    isSelected = false,
    onEdit = null,
    onDelete = null,
    onShare = null,
  } = $props();

  let hovered = $state(false);
  let confirmOpen = $state(false);

  /* 当前主题明暗（选中背景亮度分档用，对应 Theme.brightness） */
  let isDark = $state(false);
  $effect(() => {
    const mode = sm.themeState.themeMode;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => {
      isDark = mode === 'dark' || (mode !== 'light' && mq.matches);
    };
    update();
    mq.addEventListener('change', update);
    return () => mq.removeEventListener('change', update);
  });

  function hexToRgb(hex) {
    const c = hex.replace('#', '');
    return [
      parseInt(c.substring(0, 2), 16),
      parseInt(c.substring(2, 4), 16),
      parseInt(c.substring(4, 6), 16),
    ];
  }

  function rgbToHsl(r, g, b) {
    const rr = r / 255;
    const gg = g / 255;
    const bb = b / 255;
    const max = Math.max(rr, gg, bb);
    const min = Math.min(rr, gg, bb);
    const l = (max + min) / 2;
    let h = 0;
    let s = 0;
    if (max !== min) {
      const d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      if (max === rr) h = (gg - bb) / d + (gg < bb ? 6 : 0);
      else if (max === gg) h = (bb - rr) / d + 2;
      else h = (rr - gg) / d + 4;
      h /= 6;
    }
    return [h * 360, s, l];
  }

  /* 与运行时 --md-primary 完全一致（buildSchemes(seed)[mode].primary） */
  const selectedBackground = $derived.by(() => {
    /* 液态玻璃：中心完全透明（选中态由主色边框表达）——
     * 内联 style 的 background 优先级高于玻璃基座的 transparent，必须在这里让位 */
    if (sm.appSettingsState.liquidGlass) return 'transparent';
    if (!isSelected) return 'var(--md-surface-container-low)';
    void isDark;
    void sm.themeState.themeColor;
    try {
      const schemes = buildSchemes(sm.themeState.themeColor);
      const primary = (isDark ? schemes.dark : schemes.light).primary;
      const [r, g, b] = hexToRgb(primary);
      const [h, s] = rgbToHsl(r, g, b);
      return `hsl(${h.toFixed(2)} ${(s * 100).toFixed(2)}% ${isDark ? 10 : 95}%)`;
    } catch {
      return 'var(--md-surface-container-low)';
    }
  });

  const borderColor = $derived((isSelected || hovered) ? 'var(--md-primary)' : 'transparent');
  const borderWidth = $derived(hovered && !isSelected ? 1 : 2);
  const hasServers = $derived(Array.isArray(room.servers) && room.servers.length > 0);

  /* InkWell onTap → setRoom */
  function selectRoom() {
    sm.setRoom(room);
  }
</script>

<div
  class="md-card room-card"
  style="background:{selectedBackground}; border:{borderWidth}px solid {borderColor};"
  onmouseenter={() => (hovered = true)}
  onmouseleave={() => (hovered = false)}
>
  <div
    class="card-body"
    role="button"
    tabindex="0"
    onclick={selectRoom}
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        selectRoom();
      }
    }}
  >
    <div class="row-1">
      <div class="name-wrap">
        {#if isSelected}
          <span class="check-icon">
            <Icon name="check_circle" size={20} color="var(--md-primary)" />
          </span>
        {/if}
        <span class="room-name">{room.name}</span>
      </div>
      <div class="icon-row">
        {#if onShare}
          <button
            class="md-icon-btn icon-20"
            data-tip="分享房间"
            onclick={() => onShare?.()}
          >
            <Icon name="share" size={20} />
          </button>
        {/if}
        {#if onDelete}
          {#if isSelected}
            <span class="disabled-wrap" data-tip="不能删除正在连接的房间">
              <button class="md-icon-btn icon-20" disabled style="color: var(--md-outline)">
                <Icon name="delete" size={20} />
              </button>
            </span>
          {:else}
            <button
              class="md-icon-btn icon-20"
              data-tip="删除房间"
              onclick={() => (confirmOpen = true)}
            >
              <Icon name="delete" size={20} />
            </button>
          {/if}
        {/if}
        {#if onEdit}
          <button class="md-icon-btn icon-20" data-tip="编辑房间" onclick={() => onEdit?.()}>
            <Icon name="edit" size={20} />
          </button>
        {/if}
        <Icon
          name={room.encrypted ? 'lock' : 'lock_open'}
          size={20}
          color={room.encrypted ? 'var(--flutter-red)' : 'var(--flutter-green)'}
        />
      </div>
    </div>
    <div class="row-2">
      <span class="type-text">类型: {room.encrypted ? '保护' : '不保护'}</span>
      {#if hasServers}
        <span class="server-badge">携带服务器</span>
      {/if}
    </div>
  </div>
</div>

<!-- 删除确认（AlertDialog '确认删除'） -->
<Dialog bind:open={confirmOpen} title="确认删除">
  <div>确定要删除这个房间吗？</div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (confirmOpen = false)}>取消</button>
    <button
      class="md-btn md-text-btn delete-btn"
      onclick={() => {
        confirmOpen = false;
        onDelete?.();
      }}
    >删除</button>
  {/snippet}
</Dialog>

<style>
  .room-card {
    overflow: hidden;
  }
  .card-body {
    padding: 16px;
    cursor: pointer;
  }
  .row-1 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .name-wrap {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    align-items: center;
  }
  .check-icon {
    margin-right: 8px;
    display: inline-flex;
    flex-shrink: 0;
  }
  .room-name {
    min-width: 0;
    font-size: 18px;
    line-height: 24px;
    font-weight: 700;
    color: var(--md-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  /* IconButton(padding: 0, constraints: BoxConstraints()) → 20×20 */
  .icon-20 {
    width: 20px;
    height: 20px;
    min-width: 20px;
  }
  .disabled-wrap {
    display: inline-flex;
    position: relative;
  }
  .row-2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 4px;
  }
  .type-text {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
  }
  .server-badge {
    padding: 2px 8px;
    background: var(--md-primary-02);
    border-radius: 4px;
    color: var(--md-primary);
    font-size: 12px;
    line-height: 16px;
    flex-shrink: 0;
  }
  .delete-btn {
    color: var(--flutter-red);
  }
</style>
