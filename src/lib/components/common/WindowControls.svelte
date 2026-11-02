<script>
  /** 窗口控制按钮（最小化/最大化/关闭）—— 对应 WindowsControls */
  import Icon from './Icon.svelte';
  import { isTauri, isDesktopPlatform } from '../../../lib/utils/platform.js';
  import { sm } from '../../../lib/services/service-manager.svelte.js';

  let maximized = $state(false);

  async function tauriWindow() {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    return getCurrentWindow();
  }

  $effect(() => {
    if (!isTauri()) return;
    let unlisten = null;
    (async () => {
      try {
        const win = await tauriWindow();
        maximized = await win.isMaximized();
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen('tauri://resize', async () => {
          try {
            maximized = await win.isMaximized();
          } catch {}
        });
      } catch {}
    })();
    return () => unlisten?.();
  });

  async function minimize() {
    sm.uiState.setBackground(true);
    if (isTauri()) {
      try {
        (await tauriWindow()).minimize();
      } catch {}
    }
  }

  async function toggleMaximize() {
    if (isTauri()) {
      try {
        const win = await tauriWindow();
        await win.toggleMaximize();
        maximized = await win.isMaximized();
      } catch {}
    }
  }

  async function close() {
    if (isTauri()) {
      try {
        const win = await tauriWindow();
        await win.close();
      } catch {}
    } else {
      // 浏览器模式：模拟「最小化到托盘」隐藏页面内容
      window.__astralMockHidden = true;
    }
  }
</script>

{#if isDesktopPlatform() || !isTauri()}
  <div class="flex-row">
    <button class="md-icon-btn" data-tip="最小化" onclick={minimize}>
      <Icon name="remove" size={20} color="var(--md-on-primary-container)" />
    </button>
    <button class="md-icon-btn" data-tip={maximized ? '还原' : '最大化'} onclick={toggleMaximize}>
      <Icon
        name={maximized ? 'filter_none' : 'crop_square'}
        size={20}
        color="var(--md-on-primary-container)"
      />
    </button>
    <button class="md-icon-btn" data-tip="关闭" onclick={close}>
      <Icon name="close" size={20} color="var(--md-on-primary-container)" />
    </button>
  </div>
{/if}
