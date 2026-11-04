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

<!-- 液态玻璃折射滤镜：feTurbulence 流动噪声 + feDisplacementMap 对真实背景做位移折射 -->
<svg class="lg-defs" aria-hidden="true" focusable="false">
  <defs>
    <filter id="lg-refract" x="-30%" y="-30%" width="160%" height="160%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.008 0.012" numOctaves="2" seed="11" result="lg-noise">
        <animate
          attributeName="baseFrequency"
          dur="16s"
          values="0.008 0.012;0.013 0.007;0.008 0.012"
          repeatCount="indefinite"
        />
      </feTurbulence>
      <feDisplacementMap in="SourceGraphic" in2="lg-noise" scale="12" xChannelSelector="R" yChannelSelector="G" />
    </filter>
  </defs>
</svg>

{#if ready}
  <MainScreen />
{:else}
  <div class="boot">Astral</div>
{/if}

<style>
  .lg-defs {
    position: absolute;
    width: 0;
    height: 0;
    pointer-events: none;
  }
  .boot {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    color: var(--md-on-surface-variant);
  }
</style>
