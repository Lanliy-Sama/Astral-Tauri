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

<!-- 液态玻璃折射滤镜（对齐 glassium 光学模型）：
  feTurbulence + feDisplacementMap 只作用于边缘透镜层（liquid-glass.css 的 ::before，
  被 mask 限制在边缘带内），中心磨砂层不带位移 —— 与 glassium「边缘折射、中心纯净」一致。
  - #lg-edge：静止态，轻微几何位移（scale 9）
  - #lg-edge-hot：按住/拖拽态，强位移（scale 18）+ baseFrequency 持续漂移 = 液态流动 -->
<svg class="lg-defs" aria-hidden="true" focusable="false">
  <defs>
    <filter id="lg-edge" x="-20%" y="-20%" width="140%" height="140%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.012 0.016" numOctaves="2" seed="7" result="lg-noise" />
      <feDisplacementMap in="SourceGraphic" in2="lg-noise" scale="9" xChannelSelector="R" yChannelSelector="G" />
    </filter>
    <filter id="lg-edge-hot" x="-30%" y="-30%" width="160%" height="160%" color-interpolation-filters="sRGB">
      <feTurbulence type="fractalNoise" baseFrequency="0.02 0.024" numOctaves="2" seed="7" result="lg-noise">
        <animate
          attributeName="baseFrequency"
          dur="2.8s"
          values="0.02 0.024;0.028 0.017;0.02 0.024"
          repeatCount="indefinite"
        />
      </feTurbulence>
      <feDisplacementMap in="SourceGraphic" in2="lg-noise" scale="18" xChannelSelector="R" yChannelSelector="G" />
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
