<script>
  /**
   * 主屏 —— Scaffold 结构：StatusBar(appBar) + LeftNav/BottomNav + IndexedStack。
   */
  import StatusBar from './common/StatusBar.svelte';
  import LeftNav from './nav/LeftNav.svelte';
  import BottomNav from './nav/BottomNav.svelte';
  import HomePage from './home/HomePage.svelte';
  import RoomPage from './rooms/RoomPage.svelte';
  import ToolsPage from './tools/ToolsPage.svelte';
  import ServersPage from './servers/ServersPage.svelte';
  import SettingsMainPage from './settings/SettingsMainPage.svelte';
  import SnackbarHost from './common/SnackbarHost.svelte';
  import UpdateDialogs from './common/update/UpdateDialogs.svelte';
  import Dialog from './common/Dialog.svelte';
  import BgmVolumeCapsule from './common/BgmVolumeCapsule.svelte';
  import { getStack, popPage } from '../router/PageRouter.svelte';
  import { sm } from '../services/ServiceManager.svelte';
  import { bgmAdjustVolume } from '../services/BgmPlayer.svelte';
  import { t } from '../i18n/I18n.svelte';
  import { isTauri, isAndroid } from '../utils/Platform.svelte';

  let windowWidth = $state(window.innerWidth);
  let windowHeight = $state(window.innerHeight);

  $effect(() => {
    const onResize = () => {
      windowWidth = window.innerWidth;
      windowHeight = window.innerHeight;
      sm.uiState.updateScreenSplitWidth(windowWidth);
    };
    window.addEventListener('resize', onResize);
    sm.uiState.updateScreenSplitWidth(windowWidth);
    return () => window.removeEventListener('resize', onResize);
  });

  /* 液态玻璃系统（html 根类切换 / 指针高光跟随 / 全局材质）已收拢至
   * lib/components/common/LiquidGlass.svelte，由 App.svelte 挂载一次 */

  /* Android 小窗口：宽<300 或 高<400（原版 SmallWindowAdapter 仅 Android 生效，桌面永不触发） */
  const isSmallWindow = $derived(
    isAndroid() && (windowWidth < 300 || windowHeight < 400)
  );

  const navigationItems = $derived([
    { icon: 'home_outlined', activeIcon: 'home', label: t('nav_home'), page: HomePage },
    { icon: 'room_preferences_outlined', activeIcon: 'room_preferences', label: t('nav_room'), page: RoomPage },
    { icon: 'build_outlined', activeIcon: 'build', label: '工具', page: ToolsPage },
    { icon: 'dns_outlined', activeIcon: 'dns', label: t('nav_server'), page: ServersPage },
    { icon: 'settings_outlined', activeIcon: 'settings', label: t('nav_settings'), page: SettingsMainPage },
  ]);

  /* 越界索引重置 */
  $effect(() => {
    if (sm.uiState.selectedIndex >= navigationItems.length) {
      sm.uiState.selectedIndex = 0;
    }
  });

  /* 自动检查更新（启动 1 秒后，静默） */
  $effect(() => {
    const timer = setTimeout(async () => {
      if (sm.updateState.autoCheckUpdate || sm.updateState.beta) {
        const result = await sm.checkForUpdates();
        window.__astralUpdateResult = result;
        window.dispatchEvent(new CustomEvent('astral-update-result', { detail: result }));
      }
    }, 1000);
    return () => clearTimeout(timer);
  });

  const pages = $derived(navigationItems.map((item) => item.page));
  const safeIndex = $derived(
    sm.uiState.selectedIndex >= 0 && sm.uiState.selectedIndex < navigationItems.length
      ? sm.uiState.selectedIndex
      : 0
  );

  const routeStack = $derived(getStack());

  /* 切页入场动画：IndexedStack 瞬切生硬（顿挫感），新可见页轻位移+淡入；
   * WAAPI 驱动（不动 DOM 结构/页面状态），首帧与 prefers-reduced-motion 跳过 */
  let pageEls = [];
  let firstPagePaint = true;
  $effect(() => {
    const el = pageEls[safeIndex];
    if (!el) return;
    if (firstPagePaint) {
      firstPagePaint = false;
      return;
    }
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    el.animate(
      [{ opacity: 0, transform: 'translateY(10px)' }, { opacity: 1, transform: 'translateY(0)' }],
      { duration: 220, easing: 'cubic-bezier(0.05, 0.7, 0.1, 1)' }
    );
  });

  /* 最小化/失焦暂停拓扑同步（isInBackground），重新聚焦恢复 —— 此前只有
   * minimize 单向置 true、无恢复路径：最小化过一次后拓扑同步与连线动画
   * 永久停止（真 bug）。托盘/窗口恢复均会触发 focus。 */
  $effect(() => {
    const onFocus = () => sm.uiState.setBackground(false);
    const onBlur = () => sm.uiState.setBackground(true);
    window.addEventListener('focus', onFocus);
    window.addEventListener('blur', onBlur);
    return () => {
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('blur', onBlur);
    };
  });

  /* Escape 键返回（对应 Android 返回键）；
   * Ctrl+↑/↓ 背景音乐音量（灵动岛胶囊反馈，文本输入时不劫持） */
  function onKeydown(e) {
    if (e.key === 'Escape' && routeStack.length > 0) {
      popPage();
      return;
    }
    if (e.ctrlKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      if (!sm.bgmState.enabled) return;
      const t = e.target;
      if (
        t &&
        typeof t.closest === 'function' &&
        t.closest('input, textarea, select, [contenteditable="true"]')
      ) {
        return;
      }
      bgmAdjustVolume(e.key === 'ArrowUp' ? 5 : -5);
      e.preventDefault();
    }
  }

  /* 移动端/窄窗左右滑动切页（对应原版 PageView 手势） */
  const bottomNavMode = $derived(!(sm.uiState.isDesktop && !isSmallWindow));
  let touchStartX = 0;
  let touchStartY = 0;
  let touchTarget = null;

  function onTouchStart(e) {
    if (!bottomNavMode) return;
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
    touchTarget = e.target;
  }
  function onTouchEnd(e) {
    if (!bottomNavMode || touchTarget === null) return;
    const dx = e.changedTouches[0].clientX - touchStartX;
    const dy = e.changedTouches[0].clientY - touchStartY;
    touchTarget = null;
    if (Math.abs(dx) < 80 || Math.abs(dy) > 60) return;
    /* 起点/终点落在交互控件（输入框/开关/滑块）上时不切页 */
    for (const el of [touchTarget, e.target]) {
      if (el && typeof el.closest === 'function' && el.closest('input, textarea, button, [role="slider"], [role="button"]')) return;
    }
    const dir = dx < 0 ? 1 : -1;
    const next = safeIndex + dir;
    if (next >= 0 && next < navigationItems.length) {
      sm.uiState.selectedIndex = next;
    }
  }

  /* 托盘「隐藏托盘」确认流程（对应原版 trayHide，Tauri 模式） */
  let trayHideOpen = $state(false);

  $effect(() => {
    if (!isTauri()) return;
    let unlisten = null;
    import('@tauri-apps/api/event')
      .then(({ listen }) =>
        listen('tray-hide-request', () => {
          trayHideOpen = true;
        })
      )
      .then((fn) => (unlisten = fn))
      .catch(() => {});
    return () => unlisten?.();
  });

  async function confirmTrayHide() {
    trayHideOpen = false;
    sm.uiState.trayHidden = true;
    sm.uiState.setBackground(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('destroy_tray');
    } catch {
      /* 忽略 */
    }
  }
</script>

<div class="main-screen">
  {#if !isSmallWindow}
    <StatusBar />
  {/if}
  <div class="body">
    {#if sm.uiState.isDesktop && !isSmallWindow}
      <LeftNav items={navigationItems} />
    {/if}
    <div class="content-col">
      {#if isSmallWindow}
        <div class="small-window-title">
          {navigationItems[safeIndex]?.label || ''}
        </div>
      {/if}
      <div class="content" ontouchstart={onTouchStart} ontouchend={onTouchEnd}>
        {#each pages as Page, index (index)}
          <div class="stack-page {index === safeIndex ? '' : 'hidden'}" bind:this={pageEls[index]}>
            <Page />
          </div>
        {/each}
      </div>
    </div>
  </div>
  {#if !(sm.uiState.isDesktop && !isSmallWindow)}
    <BottomNav items={navigationItems} {isSmallWindow} />
  {/if}
  <SnackbarHost />
  <UpdateDialogs />
  <!-- 灵动岛音量胶囊：Ctrl+↑/↓ 调背景音乐音量时弹出 -->
  <BgmVolumeCapsule />

  <!-- 托盘隐藏确认（Tauri 模式） -->
  <Dialog bind:open={trayHideOpen} title={t('tray_hide_title')} dismissible={false}>
    <div class="md-body-medium" style="white-space:pre-line; max-width:420px">{t('tray_hide_message')}</div>
    {#snippet actions()}
      <button class="md-btn md-text-btn" onclick={() => (trayHideOpen = false)}>{t('cancel')}</button>
      <button class="md-btn md-text-btn" onclick={confirmTrayHide}>{t('tray_hide_confirm')}</button>
    {/snippet}
  </Dialog>

  <!-- 子页面路由栈（覆盖整个壳，同 Flutter 根 Navigator push） -->
  {#each routeStack as entry (entry.id)}
    <div class="route-overlay" class:route-out={entry.closing}>
      <svelte:component this={entry.component} {...entry.props} />
    </div>
  {/each}
</div>

<svelte:window on:keydown={onKeydown} />

<style>
  .main-screen {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--md-surface);
    position: relative;
  }
  .body {
    flex: 1 1 0;
    display: flex;
    min-height: 0;
  }
  .content-col {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .content {
    flex: 1 1 0;
    position: relative;
    min-height: 0;
    overflow: hidden;
  }
  .small-window-title {
    height: 36px;
    background: var(--md-primary-container-04);
    padding: 0 8px;
    display: flex;
    align-items: center;
    font-size: 16px;
    font-weight: 700;
    color: var(--md-on-primary-container);
    flex-shrink: 0;
  }
  .route-overlay {
    position: absolute;
    inset: 0;
    z-index: 500;
    background: var(--md-surface);
    /* MaterialPageRoute：入场缩放再收敛（0.94→1）+ emphasized-decelerate，
     * 幅度更小更优雅；pop 时反向轻缩放淡出（PageRouter 标 closing 延迟卸载） */
    animation: route-in 300ms cubic-bezier(0.05, 0.7, 0.1, 1);
  }
  .route-overlay.route-out {
    animation: route-out 190ms cubic-bezier(0.3, 0, 0.8, 0.15) forwards;
    pointer-events: none;
  }
  @keyframes route-in {
    from { opacity: 0; transform: scale(0.94); }
    to { opacity: 1; transform: scale(1); }
  }
  @keyframes route-out {
    from { opacity: 1; transform: scale(1); }
    to { opacity: 0; transform: scale(0.96); }
  }
</style>
