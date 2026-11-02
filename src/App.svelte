<script>
  /** 应用根组件：初始化 ServiceManager 后挂载主屏；主题响应式应用 */
  import MainScreen from './lib/components/MainScreen.svelte';
  import { sm } from './lib/services/service-manager.svelte.js';
  import { applyThemeNow } from './lib/services/theme-engine.svelte.js';
  import { initAppLinks } from './lib/services/app-links.svelte.js';

  let ready = $state(false);

  $effect(() => {
    let cancelled = false;
    (async () => {
      try {
        await sm.init();
      } catch (e) {
        console.error('ServiceManager 初始化失败:', e);
      }
      if (!cancelled) {
        ready = true;
        /* 深链接（astral://，Tauri 模式） */
        initAppLinks();
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  /* 主题响应：色值或模式变化时重新生成令牌 */
  $effect(() => {
    void sm.themeState.themeColor;
    void sm.themeState.themeMode;
    applyThemeNow();
  });
</script>

{#if ready}
  <MainScreen />
{:else}
  <div class="boot">Astral</div>
{/if}

<style>
  .boot {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    color: var(--md-on-surface-variant);
  }
</style>
