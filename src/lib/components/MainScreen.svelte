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
  import { getStack, popPage } from '../router/page-router.svelte.js';
  import { sm } from '../services/service-manager.svelte.js';
  import { t } from '../i18n/index.svelte.js';
  import { isTauri, isAndroid } from '../utils/platform.js';

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

  /* Escape 键返回（对应 Android 返回键） */
  function onKeydown(e) {
    if (e.key === 'Escape' && routeStack.length > 0) {
      popPage();
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
      <div class="content">
        {#each pages as Page, index (index)}
          <div class="stack-page {index === safeIndex ? '' : 'hidden'}">
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
    <div class="route-overlay">
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
    /* MaterialPageRoute + Windows ZoomPageTransitionsBuilder：300ms 淡入+缩放 0.80→1 */
    animation: route-in 300ms cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes route-in {
    from { opacity: 0; transform: scale(0.8); }
    to { opacity: 1; transform: scale(1); }
  }
</style>
