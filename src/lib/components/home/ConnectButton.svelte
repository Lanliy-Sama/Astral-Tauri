<script>
  /**
   * 连接按钮 —— 对照 lib/features/home/widgets/connect_button.dart + connect_button_style.dart。
   *
   * 三态（CoState 驱动）：
   *   idle      100×60 bg primary    icon power_settings_new  文本 '连接'
   *   connecting 180×60 bg error      icon close（800ms 往返旋转） '点击取消' + 进度条上滑
   *   connected  180×60 bg tertiary   icon link                '已连接'
   *
   * 进度条：180×6 r3，轨道 surfaceContainerHighest，填充 tertiary→primary 渐变，
   * 15s TweenAnimationBuilder（对应连接超时 15s）。
   */
  import Icon from '../common/Icon.svelte';
  import NpcapDialog from './NpcapDialog.svelte';
  import { sm } from '../../services/service-manager.svelte.js';
  import { CoState, MainTab } from '../../stores/enums.js';
  import { t } from '../../i18n/index.svelte.js';
  import { showSnackbar } from '../common/snackbars.svelte.js';
  import { openExternal, hasNpcapDriver } from '../../rust/api.js';
  import { isWindows, isTauri } from '../../utils/platform.js';

  let npcapOpen = $state(false);
  let progressKey = $state(0); // 每次进入 connecting 重置 15s 动画
  let lastState = $state(CoState.idle);

  const state = $derived(sm.connectionState.connectionState);
  const isIdle = $derived(state === CoState.idle);
  const isConnecting = $derived(state === CoState.connecting);
  const isConnected = $derived(state === CoState.connected);

  $effect(() => {
    if (state === CoState.connecting && lastState !== CoState.connecting) {
      progressKey += 1; // 重新播放 15s 进度动画
    }
    lastState = state;
  });

  /* ---------- 点击逻辑 ---------- */
  async function toggle() {
    if (isIdle) {
      await handleConnect();
    } else if (isConnecting) {
      await sm.cancelConnection();
    } else if (isConnected) {
      await sm.disconnect();
    }
  }

  async function handleConnect() {
    /* 1. 连接目标检查（ConnectionConnectGuard.connectTargetIssue） */
    const issue = sm.connectTargetIssue();
    if (issue) {
      const guide = {
        noRoom: {
          title: t('select_room_first'),
          action: t('go_select_room'),
          icon: 'meeting_room_outlined',
          tab: MainTab.room,
        },
        noneEnabled: {
          title: t('enable_server_first'),
          action: t('go_enable'),
          icon: 'toggle_on_outlined',
          tab: MainTab.servers,
        },
        noServers: {
          title: t('add_server_first'),
          action: t('go_add'),
          icon: 'dns_outlined',
          tab: MainTab.servers,
        },
      };
      const g = guide[issue];
      showSnackbar({
        title: g.title,
        type: 'info',
        icon: g.icon,
        duration: 3000,
        bg: 'var(--md-inverse-surface)',
        action: { label: g.action, onPressed: () => sm.uiState.goTo(g.tab) },
      });
      return;
    }

    /* 2. Npcap 检查（Windows + FakeTCP 场景） */
    const desktopLike = isWindows();
    if (desktopLike && containsFaketcp() && !(await hasNpcapDriver())) {
      npcapOpen = true;
      return;
    }

    /* 3. 发起连接 */
    const result = await sm.connect({ isManual: true });
    if (result === false) {
      showSnackbar({
        title: '连接失败',
        message: '请检查网络后重试，并确认房间与服务器配置无误',
        type: 'error',
        duration: 4000,
      });
    }
  }

  function containsFaketcp() {
    const room = sm.roomState.selectedRoom;
    if (!room) return false;
    const roomUrls = room.servers || [];
    if (roomUrls.some((u) => (u || '').toLowerCase().trim().startsWith('faketcp://'))) {
      return true;
    }
    return sm.serverState.servers
      .filter((s) => s.enable)
      .some((s) => s.faketcp === true || (s.url || '').toLowerCase().startsWith('faketcp://'));
  }

  async function onNpcapResult(view) {
    if (view) {
      const ok = await openExternal('https://astral.fan/quick-start/download-install/');
      if (!ok) {
        showSnackbar({
          title: '打开失败',
          message: '无法打开教程页面，请手动访问 astral.fan',
          type: 'error',
          duration: 4000,
        });
      }
    }
  }
</script>

<!-- Scaffold FAB：endFloat（外层 16px 边距由 HomePage 提供） -->
<div class="connect-button-zone">
  <div class="cb-column">
    <!-- 进度条区（connecting 时上滑淡入） -->
    <div class="progress-slot {isConnecting ? 'show' : ''}">
      {#key progressKey}
        <div class="progress-track">
          <div class="progress-fill" class:animate={isConnecting}></div>
        </div>
      {/key}
    </div>
    <!-- 按钮（200ms easeOutCubic 宽度动画） -->
    <button
      class="md-fab extended connect-fab {state}"
      style="width:{isIdle ? 100 : 180}px; height:60px; background:{isIdle
        ? 'var(--md-primary)'
        : isConnecting
          ? 'var(--md-error)'
          : 'var(--md-tertiary)'}; color:{isIdle
        ? 'var(--md-on-primary)'
        : isConnecting
          ? 'var(--md-on-error)'
          : 'var(--md-on-tertiary)'}"
      onclick={toggle}
    >
      <span class="cb-icon">
        {#key state}
          <span class="cb-icon-inner {isConnecting ? 'spin' : ''}">
            {#if isIdle}
              <Icon name="power_settings_new_rounded" size={24} color="inherit" />
            {:else if isConnecting}
              <Icon name="close_rounded" size={24} color="inherit" />
            {:else}
              <Icon name="link_rounded" size={24} color="inherit" />
            {/if}
          </span>
        {/key}
      </span>
      <span class="cb-label">
        {#key state}
          <span class="cb-label-inner">{isIdle ? '连接' : isConnecting ? '点击取消' : '已连接'}</span>
        {/key}
      </span>
    </button>
  </div>
</div>

<NpcapDialog bind:open={npcapOpen} onresult={onNpcapResult} />

<style>
  .connect-button-zone {
    padding: 16px; /* ConnectButton 内部 Padding(16) */
    display: flex;
    justify-content: flex-end;
    align-items: flex-end;
  }
  .cb-column {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: flex-end;
  }
  .progress-slot {
    height: 14px;
    width: 180px;
    display: flex;
    align-items: flex-end;
    justify-content: flex-end;
    overflow: hidden;
    opacity: 0;
    transform: translateY(100%);
    transition:
      opacity 300ms ease,
      transform 300ms ease;
    margin-bottom: 0;
  }
  .progress-slot.show {
    opacity: 1;
    transform: translateY(0);
  }
  .progress-track {
    width: 180px;
    height: 6px;
    border-radius: 3px;
    background: var(--md-surface-container-highest);
    margin-bottom: 8px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    width: 0;
    border-radius: 3px;
    background: linear-gradient(to right, var(--md-tertiary), var(--md-primary));
  }
  .progress-fill.animate {
    animation: connect-progress 15s ease-in-out forwards;
  }
  @keyframes connect-progress {
    from { width: 0%; }
    to { width: 100%; }
  }
  .connect-fab {
    transition: width 200ms cubic-bezier(0, 0, 0.2, 1);
    padding: 0 2px; /* extendedPadding: symmetric(h:2) */
    gap: 8px;
  }
  .cb-icon {
    display: inline-flex;
    width: 24px;
    height: 24px;
    align-items: center;
    justify-content: center;
  }
  .cb-icon-inner {
    display: inline-flex;
    animation: cb-fade-in 150ms ease;
  }
  .cb-icon-inner.spin {
    animation:
      cb-spin 800ms linear infinite alternate,
      cb-fade-in 150ms ease;
  }
  @keyframes cb-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
  @keyframes cb-fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  .cb-label {
    font-size: 16px;
    font-weight: 500;
    line-height: 20px;
    white-space: nowrap;
  }
  .cb-label-inner {
    display: inline-block;
    animation: cb-text-in 200ms ease;
  }
  @keyframes cb-text-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
