<script>
  /**
   * 应用根组件 —— 启动装配 + Material 3 设计系统（全局样式）。
   * 原 main.js / theme-engine.svelte.js / app-links.svelte.js 的 JS 已全部内联于此：
   *   启动序列：initPlatform → initI18n → 主题调试覆盖 → initThemeEngine → sm.init
   *   （挂载前显示 boot 启动屏，就绪后渲染 MainScreen）
   * M3 设计系统（原独立 app.css）以本组件 <style> 的 :global 块承载；
   * 液态玻璃系统见 <LiquidGlass />（CSS/SVG/JS 自包含）。
   */
  import MainScreen from './lib/components/MainScreen.svelte';
  import LiquidGlass from './lib/components/common/LiquidGlass.svelte';
  import { sm } from './lib/services/ServiceManager.svelte';
  import { buildSchemes, applyTheme } from './lib/theme/Material3.svelte';
  import { initPlatform, isTauri } from './lib/utils/Platform.svelte';
  import { initI18n } from './lib/i18n/I18n.svelte';
  import { decryptRoom, validateRoom, cleanRoom } from './lib/utils/RoomShareCodec.svelte';
  import { snackError, snackSuccess, snackInfo } from './lib/components/common/SnackbarStore.svelte';

  let ready = $state(false);

  /* ============ 主题引擎（原 theme-engine.svelte.js） ============ */
  let mediaQuery = null;

  function initThemeEngine() {
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', () => {
      if (sm.themeState.themeMode === 'system') applyThemeNow();
    });
  }

  function resolveMode() {
    const mode = sm.themeState.themeMode;
    if (mode === 'light') return 'light';
    if (mode === 'dark') return 'dark';
    return mediaQuery && mediaQuery.matches ? 'dark' : 'light';
  }

  /** 立即应用当前主题（幂等） */
  function applyThemeNow() {
    const schemes = buildSchemes(sm.themeState.themeColor);
    const mode = resolveMode();
    applyTheme(schemes[mode], mode);
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) {
      meta.setAttribute(
        'content',
        mode === 'dark' ? schemes.dark.surfaceContainerLow : schemes.light.primaryContainer
      );
    }
    window.__astralThemeMode = mode;
  }

  /** 强制切换（截图调试用）：?theme=dark / ?theme=light */
  function applyDebugThemeOverride() {
    const params = new URLSearchParams(location.search);
    const forced = params.get('theme');
    if (forced === 'dark' || forced === 'light') {
      sm.themeState.themeMode = forced;
    }
  }

  /* ============ 深链接（原 app-links.svelte.js，Tauri 模式） ============ */
  async function handleLink(url) {
    try {
      const uri = new URL(url);
      if (uri.protocol !== 'astral:' || !uri.host) return;

      if (uri.host === 'room') {
        const code = uri.searchParams.get('code');
        const cleaned = (code || '').replace(/\s+/g, '');
        if (!cleaned) {
          snackError('分享链接格式错误', '不是有效的房间分享链接');
          return;
        }
        if (cleaned.length < 10) {
          snackError('分享码无效', '分享码解析失败');
          return;
        }
        const room = await decryptRoom(cleaned);
        if (!room) {
          snackError('分享码解析失败', '无法解析房间信息');
          return;
        }
        const [valid, err] = validateRoom(room);
        if (!valid) {
          snackError('房间信息不完整', err || '房间数据无效');
          return;
        }
        /* 查重（加密房间比 name+roomName+password；非加密比 roomName+password） */
        const duplicate = sm.roomState.rooms.some((r) =>
          room.encrypted
            ? r.name === room.name && r.roomName === room.roomName && r.password === room.password
            : r.roomName === room.roomName && r.password === room.password
        );
        if (duplicate) {
          snackInfo('房间已存在', room.name);
          return;
        }
        const cleanedRoom = cleanRoom(room);
        const saved = await sm.addRoom(cleanedRoom);
        sm.uiState.goTo(1);
        await sm.setRoom(saved);
        snackSuccess('导入成功', `已成功添加并选中房间"${room.name}"`);
      } else if (uri.host === 'debug') {
        snackInfo('调试信息', '已触发调试链接： debug');
      }
    } catch (e) {
      snackError('处理分享链接失败', String(e));
    }
  }

  async function initAppLinks() {
    if (!isTauri()) return;
    try {
      const deepLink = await import('@tauri-apps/plugin-deep-link');
      /* 冷启动链接：getCurrent() 返回 URL[]（可多条），取首条再进 handleLink；
       * 直接传数组会让 new URL(数组) 抛 TypeError → 冷启动导入必失败 */
      const initial = await deepLink.getCurrent();
      const initialUrl = Array.isArray(initial) ? initial[0] : initial;
      if (initialUrl) handleLink(initialUrl);
      /* 运行中链接 */
      await deepLink.onDeepLink((event) => {
        const url = Array.isArray(event?.urls) ? event.urls[0] : event?.url;
        if (typeof url === 'string') handleLink(url);
      });
    } catch (e) {
      console.warn('深链接初始化失败:', e);
    }
  }

  /* ============ 启动序列（原 main.js） ============ */
  $effect(() => {
    let cancelled = false;
    (async () => {
      try {
        await initPlatform();
        await initI18n();
        applyDebugThemeOverride();
        initThemeEngine();
        await sm.init();
      } catch (e) {
        console.error('ServiceManager 初始化失败:', e);
      }
      if (!cancelled) {
        ready = true;
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

  /* 自定义背景响应：设置 → 软件设置 → 自定义背景。
   * v2.6：背景 URL 为 astralbg:// 自定义协议（Tauri，磁盘直服务，任意大小）
   * 或 Blob URL（浏览器预览）—— 均为短字符串，避开 Chromium 对 CSS
   * 自定义属性 ~1.5MB 的上限（旧 data URL 方案超限即静默失效）。
   * 写入根元素 CSS 变量 + has-custom-bg 类；各层半透明适配见 :global 样式。
   * GIF 经 background-image 引用时在 WebView 中自动逐帧播放。 */
  $effect(() => {
    const bg = sm.appSettingsState.customBackground;
    const root = document.documentElement;
    if (bg) {
      /* 协议/Blob URL 不含引号，可安全嵌入 url("...") */
      root.style.setProperty('--astral-bg-image', `url("${bg}")`);
      root.classList.add('has-custom-bg');
    } else {
      root.style.removeProperty('--astral-bg-image');
      root.classList.remove('has-custom-bg');
    }
  });
</script>

<!-- 液态玻璃系统：全局材质样式 + SVG 折射滤镜 + 指针高光跟随（自包含组件） -->
<LiquidGlass />

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

  /* ===== Material 3 设计系统（原 src/app.css，:global 全局作用） ===== */
  :global {
/* ============================================================
   Astral — Material 3 设计系统（对齐 Flutter useMaterial3 默认样式）
   颜色令牌由 Material3.svelte 动态注入 --md-* 变量。
   字体注册复刻原版 pubspec 的行为：
     MiSans 400 → Demibold（默认正文），800 → Regular
   ============================================================ */

@font-face {
  font-family: 'MiSans';
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url('/fonts/MiSans-Demibold.ttf') format('truetype');
}
@font-face {
  font-family: 'MiSans';
  font-style: normal;
  font-weight: 500;
  font-display: swap;
  src: url('/fonts/MiSans-Demibold.ttf') format('truetype');
}
@font-face {
  font-family: 'MiSans';
  font-style: normal;
  font-weight: 800;
  font-display: swap;
  src: url('/fonts/MiSans-Regular.ttf') format('truetype');
}
/* 同文件其它权重按 CSS 就近匹配规则回落到上述两档，与 Flutter 引擎行为一致 */

@font-face {
  font-family: 'Material Symbols Outlined';
  font-style: normal;
  font-weight: 100 700;
  font-display: block;
  src: url('/fonts/material-symbols-outlined.woff2') format('woff2');
}

/* ---------- 颜色令牌兜底（实际运行时由 JS 注入） ---------- */
:root {
  --md-primary: #0061a4;
  --md-on-primary: #ffffff;
  --md-primary-container: #d1e4ff;
  --md-on-primary-container: #001d36;
  --md-secondary: #535f70;
  --md-on-secondary: #ffffff;
  --md-secondary-container: #d7e3f7;
  --md-on-secondary-container: #101c2b;
  --md-tertiary: #6b5778;
  --md-on-tertiary: #ffffff;
  --md-tertiary-container: #f2daff;
  --md-on-tertiary-container: #27164d;
  --md-error: #ba1a1a;
  --md-on-error: #ffffff;
  --md-error-container: #ffdad6;
  --md-on-error-container: #410002;
  --md-background: #fdfcff;
  --md-on-background: #1a1c1e;
  --md-surface: #fdfcff;
  --md-on-surface: #1a1c1e;
  --md-on-surface-variant: #43474e;
  --md-surface-variant: #dfe2eb;
  --md-outline: #74777f;
  --md-outline-variant: #c4c6d0;
  --md-shadow: #000000;
  --md-inverse-surface: #2f3033;
  --md-inverse-on-surface: #f1f0f4;
  --md-inverse-primary: #a0c9ff;
  --md-surface-container-lowest: #ffffff;
  --md-surface-container-low: #f4f3f7;
  --md-surface-container: #eeedf1;
  --md-surface-container-high: #e8e8eb;
  --md-surface-container-highest: #e2e2e6;
  --md-surface-dim: #dadadd;
  --md-surface-bright: #faf9fc;
  --md-primary-06: rgba(0, 97, 164, 0.06);
  --md-primary-01: rgba(0, 97, 164, 0.1);
  --md-primary-02: rgba(0, 97, 164, 0.2);
  --md-primary-03: rgba(0, 97, 164, 0.3);
  --md-primary-06b: rgba(0, 97, 164, 0.6);
  --md-surface-container-highest-05: rgba(226, 226, 230, 0.5);
  --md-surface-container-highest-015: rgba(226, 226, 230, 0.15);
  --md-surface-container-highest-012: rgba(226, 226, 230, 0.12);
  --md-surface-container-highest-095: rgba(226, 226, 230, 0.95);
  --md-outline-02: rgba(116, 119, 127, 0.2);
  --md-outline-03: rgba(116, 119, 127, 0.3);
  --md-on-surface-06: rgba(26, 28, 30, 0.6);
  --md-on-surface-07: rgba(26, 28, 30, 0.7);
  --md-primary-container-012: rgba(209, 228, 255, 0.12);
  --md-primary-container-04: rgba(209, 228, 255, 0.4);
  --md-on-surface-012: rgba(26, 28, 30, 0.12);
  --md-on-surface-038: rgba(26, 28, 30, 0.38);
  --md-on-surface-045: rgba(26, 28, 30, 0.45);
  /* M3 状态层：InkWell 悬停 = onSurface 8%；按下 = 10%；文本/轮廓按钮悬停 = primary 8% */
  --md-on-surface-08: rgba(26, 28, 30, 0.08);
  --md-on-surface-04: rgba(26, 28, 30, 0.04);
  --md-on-surface-10: rgba(26, 28, 30, 0.1);
  --md-primary-08: rgba(0, 97, 164, 0.08);
  /* Flutter Material 硬编码色 */
  --flutter-green: #4caf50;
  --flutter-red: #f44336;
  --flutter-blue: #2196f3;
  --flutter-orange: #ff9800;
  --flutter-grey: #9e9e9e;
  --flutter-deep-purple: #673ab7;
  --flutter-red-700: #d32f2f;
  --flutter-green-700: #388e3c;
  --flutter-blue-700: #1976d2;
  --flutter-green-600: #43a047;
  --flutter-red-400: #ef5350;
  /* NAT 稀有度色（WoW 品质色） */
  --nat-legendary: #ff6b00;
  --nat-epic: #a335ee;
  --nat-rare: #0070dd;
  --nat-common: #1eff00;
  --nat-difficult: #9d9d9d;
}

/* ---------- 基础 ---------- */
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}
html,
body {
  height: 100%;
  overflow: hidden;
}
body {
  font-family: 'MiSans', sans-serif;
  font-size: 14px;
  line-height: 20px;
  color: var(--md-on-surface);
  background: var(--md-surface);
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
  user-select: none;
  cursor: default;
}
#app {
  height: 100%;
}
input,
textarea,
select,
button {
  font-family: inherit;
  font-size: inherit;
  color: inherit;
}
::selection {
  background: var(--md-primary-container);
  color: var(--md-on-primary-container);
}
img {
  display: block;
}
a {
  color: var(--md-primary);
  text-decoration: none;
}

/* 可选中文本（日志、分享码等） */
.selectable,
.selectable * {
  user-select: text;
  cursor: text;
}

/* ---------- 滚动条（近似 Flutter Scrollbar） ---------- */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: var(--md-outline);
  border-radius: 8px;
  opacity: 0.4;
}
::-webkit-scrollbar-thumb:hover {
  background: var(--md-on-surface-variant);
}
.scroll-thick::-webkit-scrollbar {
  width: 11px;
}
.scroll-thick::-webkit-scrollbar-thumb {
  border-radius: 16px;
}

/* ---------- Material Symbols 图标 ---------- */
.ms {
  font-family: 'Material Symbols Outlined';
  font-weight: normal;
  font-style: normal;
  line-height: 1;
  letter-spacing: normal;
  text-transform: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  white-space: nowrap;
  word-wrap: normal;
  direction: ltr;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
  font-feature-settings: 'liga';
  font-variation-settings: 'FILL' 0, 'wght' 400, 'GRAD' 0, 'opsz' 24;
  font-size: 24px;
  width: 1em;
  height: 1em;
  overflow: hidden;
  flex-shrink: 0;
}
.ms.filled {
  font-variation-settings: 'FILL' 1, 'wght' 400, 'GRAD' 0, 'opsz' 24;
}

/* ---------- 排版（Material 2021 / MiSans） ---------- */
.md-display-large { font-size: 57px; line-height: 64px; font-weight: 400; letter-spacing: -0.25px; }
.md-display-medium { font-size: 45px; line-height: 52px; font-weight: 400; }
.md-display-small { font-size: 36px; line-height: 44px; font-weight: 400; }
.md-headline-large { font-size: 32px; line-height: 40px; font-weight: 400; }
.md-headline-medium { font-size: 28px; line-height: 36px; font-weight: 400; }
.md-headline-small { font-size: 24px; line-height: 32px; font-weight: 400; }
.md-title-large { font-size: 22px; line-height: 28px; font-weight: 400; }
.md-title-medium { font-size: 16px; line-height: 24px; font-weight: 500; letter-spacing: 0.15px; }
.md-title-small { font-size: 14px; line-height: 20px; font-weight: 500; letter-spacing: 0.1px; }
.md-label-large { font-size: 14px; line-height: 20px; font-weight: 500; letter-spacing: 0.1px; }
.md-label-medium { font-size: 12px; line-height: 16px; font-weight: 500; letter-spacing: 0.5px; }
.md-label-small { font-size: 11px; line-height: 16px; font-weight: 500; letter-spacing: 0.5px; }
.md-body-large { font-size: 16px; line-height: 24px; font-weight: 400; letter-spacing: 0.5px; }
.md-body-medium { font-size: 14px; line-height: 20px; font-weight: 400; letter-spacing: 0.25px; }
.md-body-small { font-size: 12px; line-height: 16px; font-weight: 400; letter-spacing: 0.4px; }

/* ---------- 通用工具 ---------- */
.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ellipsis-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.flex-row { display: flex; flex-direction: row; align-items: center; }
.flex-col { display: flex; flex-direction: column; }
.grow { flex: 1 1 0; min-width: 0; }

/* ---------- 卡片（Flutter M3 Card：surfaceContainerLow + elevation1 + r12） ---------- */
.md-card {
  background: var(--md-surface-container-low);
  border-radius: 12px;
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.3),
    0 1px 3px 1px rgba(0, 0, 0, 0.15);
  position: relative;
}
.md-card.elev-0,
.md-card.flat {
  box-shadow: none;
}
.md-card.elev-2 {
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.3),
    0 2px 6px 2px rgba(0, 0, 0, 0.15);
}
.md-card.elev-3 {
  box-shadow:
    0 1px 3px rgba(0, 0, 0, 0.3),
    0 4px 8px 3px rgba(0, 0, 0, 0.15);
}

/* ---------- 按钮 ---------- */
.md-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 40px;
  padding: 0 24px;
  border-radius: 20px;
  border: none;
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.1px;
  line-height: 20px;
  cursor: pointer;
  white-space: nowrap;
  position: relative;
  background: transparent;
  transition: box-shadow 120ms ease, background-color 120ms ease;
}
.md-btn:disabled {
  cursor: default;
  pointer-events: none;
}
.md-btn .ms { font-size: 18px; }

.md-filled-btn {
  background: var(--md-primary);
  color: var(--md-on-primary);
}
.md-filled-btn:hover { box-shadow: 0 1px 2px rgba(0,0,0,.3), 0 1px 3px 1px rgba(0,0,0,.15); }
.md-filled-btn:active { background: color-mix(in srgb, var(--md-primary) 88%, var(--md-on-primary)); }
.md-filled-btn:disabled { background: var(--md-on-surface-012); color: var(--md-on-surface-038); }

.md-tonal-btn {
  background: var(--md-secondary-container);
  color: var(--md-on-secondary-container);
}
.md-tonal-btn:hover { box-shadow: 0 1px 2px rgba(0,0,0,.3), 0 1px 3px 1px rgba(0,0,0,.15); }
.md-tonal-btn:active { background: color-mix(in srgb, var(--md-secondary-container) 88%, var(--md-on-secondary-container)); }
.md-tonal-btn:disabled { background: var(--md-on-surface-012); color: var(--md-on-surface-038); }

.md-elevated-btn {
  background: var(--md-surface);
  color: var(--md-primary);
  box-shadow: 0 1px 2px rgba(0,0,0,.3), 0 1px 3px 1px rgba(0,0,0,.15);
}
.md-elevated-btn:hover { box-shadow: 0 1px 2px rgba(0,0,0,.3), 0 2px 6px 2px rgba(0,0,0,.15); }
.md-elevated-btn:active { background: var(--md-primary-06); }
.md-elevated-btn:disabled { background: var(--md-on-surface-012); color: var(--md-on-surface-038); box-shadow: none; }

.md-outlined-btn {
  border: 1px solid var(--md-outline);
  color: var(--md-primary);
  background: transparent;
}
.md-outlined-btn:hover { background: var(--md-primary-08); }
.md-outlined-btn:disabled { border-color: var(--md-on-surface-012); color: var(--md-on-surface-038); }

.md-text-btn {
  color: var(--md-primary);
  padding: 0 12px;
}
.md-text-btn:hover { background: var(--md-primary-08); }
.md-text-btn:disabled { color: var(--md-on-surface-038); }

/* ---------- IconButton（40×40，视觉密度紧凑时更小） ---------- */
.md-icon-btn {
  width: 40px;
  height: 40px;
  min-width: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  border: none;
  background: transparent;
  color: var(--md-on-surface-variant);
  cursor: pointer;
  position: relative;
  padding: 0;
  transition: background-color 100ms ease;
}
.md-icon-btn:hover { background: var(--md-on-surface-08); }
.md-icon-btn:active { background: var(--md-on-surface-10); }
.md-icon-btn:disabled { color: var(--md-on-surface-038); pointer-events: none; }
.md-icon-btn.compact { width: 32px; height: 32px; min-width: 32px; }
.md-icon-btn.tight {
  width: 24px;
  height: 24px;
  min-width: 24px;
}

/* ---------- FAB（M3 默认 56×56 r16 primaryContainer） ---------- */
.md-fab {
  width: 56px;
  height: 56px;
  min-width: 56px;
  border-radius: 16px;
  border: none;
  background: var(--md-primary-container);
  color: var(--md-on-primary-container);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  box-shadow: 0 1px 3px rgba(0,0,0,.3), 0 4px 8px 3px rgba(0,0,0,.15);
  position: relative;
  padding: 0;
  transition: box-shadow 120ms ease, background-color 120ms ease;
}
.md-fab:hover { box-shadow: 0 1px 3px rgba(0,0,0,.3), 0 6px 10px 4px rgba(0,0,0,.15); }
.md-fab:active { background: color-mix(in srgb, var(--md-primary-container) 88%, var(--md-on-primary-container)); }
.md-fab.small { width: 40px; height: 40px; min-width: 40px; border-radius: 12px; }
.md-fab.extended {
  width: auto;
  height: 56px;
  padding: 0 16px;
  gap: 12px;
  border-radius: 16px;
}
.md-fab.extended.has-icon { padding: 0 16px; }

/* ---------- ListTile ---------- */
.md-list-tile {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 56px;
  padding: 8px 16px;
  cursor: pointer;
  position: relative;
}
.md-list-tile.no-cursor { cursor: default; }
.md-list-tile.dense { min-height: 48px; padding: 6px 16px; }
.md-list-tile.two-line { min-height: 72px; }
.md-list-tile.three-line { min-height: 88px; align-items: flex-start; }
.md-list-tile .tile-leading {
  width: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.md-list-tile .tile-content {
  flex: 1 1 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}
.md-list-tile .tile-title { font-size: 16px; line-height: 24px; color: var(--md-on-surface); }
.md-list-tile .tile-subtitle { font-size: 14px; line-height: 20px; color: var(--md-on-surface-variant); margin-top: 2px; }
.md-list-tile.dense .tile-title { font-size: 14px; line-height: 20px; }
.md-list-tile.dense .tile-subtitle { font-size: 12px; line-height: 16px; }
.md-list-tile .tile-trailing {
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
  color: var(--md-on-surface-variant);
}
.md-list-tile:hover { background: var(--md-on-surface-08); }
.md-list-tile:active { background: var(--md-on-surface-012); }
.md-list-tile.disabled { pointer-events: none; }
.md-list-tile.disabled .tile-title,
.md-list-tile.disabled .tile-subtitle { color: var(--md-on-surface-038); }

/* SwitchListTile / CheckboxListTile 布局 */
.md-switch-list-tile {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 72px;
  padding: 8px 16px;
  cursor: pointer;
}
.md-switch-list-tile .tile-content { flex: 1 1 0; min-width: 0; }
.md-switch-list-tile .tile-title { font-size: 16px; line-height: 24px; color: var(--md-on-surface); }
.md-switch-list-tile .tile-subtitle { font-size: 14px; line-height: 20px; color: var(--md-on-surface-variant); margin-top: 2px; }
/* 单行 SwitchListTile（无 subtitle）= ListTile one-line 56px 高 */
.md-switch-list-tile.single { min-height: 56px; padding: 0 16px; }

/* ---------- M3 Switch（52×32，Flutter 默认无 thumb 图标） ---------- */
.md-switch {
  width: 52px;
  height: 32px;
  border-radius: 16px;
  border: 2px solid var(--md-outline);
  background: var(--md-surface-container-highest);
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  transition: background-color 200ms ease, border-color 200ms ease;
  padding: 0;
  display: block;
}
.md-switch .thumb {
  position: absolute;
  top: 50%;
  left: 6px;
  transform: translateY(-50%);
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--md-outline);
  transition:
    left 240ms cubic-bezier(0.2, 0, 0, 1),
    width 240ms cubic-bezier(0.2, 0, 0, 1),
    height 240ms cubic-bezier(0.2, 0, 0, 1),
    background-color 200ms ease;
  display: flex;
  align-items: center;
  justify-content: center;
  color: transparent;
}
.md-switch.on {
  background: var(--md-primary);
  border-color: var(--md-primary);
}
.md-switch.on .thumb {
  left: 22px; /* 22 + 2px 边框 = 视觉 24px：Flutter M3 ON thumb 在轨道 [24,48]，右缘留 4px */
  width: 24px;
  height: 24px;
  background: var(--md-on-primary);
}
/* 悬停 thumb 变色（Flutter M3：ON+hover → primaryContainer；OFF+hover → onSurfaceVariant；禁用无悬停效果） */
.md-switch:not(:disabled):hover.on .thumb { background: var(--md-primary-container); }
.md-switch:not(:disabled):hover:not(.on) .thumb { background: var(--md-on-surface-variant); }
.md-switch:disabled { opacity: 1; background: var(--md-surface-container-highest-012); border-color: var(--md-on-surface-012); cursor: default; }
.md-switch:disabled .thumb { background: var(--md-on-surface-038); }
.md-switch.on:disabled { background: var(--md-on-surface-012); border-color: transparent; }
.md-switch.on:disabled .thumb { background: var(--md-surface); }
.md-switch.scale-08 { transform: scale(0.8); transform-origin: center; } /* Transform.scale 仅视觉缩放，布局仍占 52×32 */

/* ---------- Checkbox（M3 18×18） ---------- */
.md-checkbox {
  width: 18px;
  height: 18px;
  border-radius: 2px;
  border: 2px solid var(--md-on-surface-variant);
  background: transparent;
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  color: transparent;
  transition: background-color 120ms, border-color 120ms;
}
.md-checkbox.checked {
  background: var(--md-primary);
  border-color: var(--md-primary);
  color: var(--md-on-primary);
}
.md-checkbox .ms { font-size: 16px; margin-top: -1px; }
.md-checkbox:disabled { border-color: var(--md-on-surface-038); cursor: default; }
.md-checkbox.checked:disabled { background: var(--md-on-surface-038); border-color: transparent; color: var(--md-surface); }

/* ---------- Radio ---------- */
.md-radio {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--md-on-surface-variant);
  background: transparent;
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  padding: 0;
}
.md-radio::after {
  content: '';
  position: absolute;
  inset: 3px;
  border-radius: 50%;
  background: var(--md-primary);
  opacity: 0;
  transform: scale(0.5);
  transition: all 100ms ease;
}
.md-radio.checked { border-color: var(--md-primary); }
.md-radio.checked::after { opacity: 1; transform: scale(1); }

/* ---------- 分隔线 ---------- */
.md-divider {
  height: 1px;
  background: var(--md-outline-variant);
  width: 100%;
  flex-shrink: 0;
}

/* ---------- 文本输入框（OutlinedTextField + 浮动标签） ---------- */
.md-text-field {
  position: relative;
  display: flex;
  align-items: center;
  min-height: 56px;
  border: 1px solid var(--md-outline);
  border-radius: 4px;
  background: transparent;
  transition: border-color 100ms ease;
}
.md-text-field:hover { border-color: var(--md-on-surface); }
.md-text-field.focused { border: 2px solid var(--md-primary); }
.md-text-field.error { border: 2px solid var(--md-error); }
.md-text-field.disabled { border-color: var(--md-on-surface-012); background: var(--md-on-surface-04); }
.md-text-field .tf-prefix,
.md-text-field .tf-suffix {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0 12px;
  color: var(--md-on-surface-variant);
  min-width: 24px;
  align-self: stretch;
  flex-shrink: 0;
}
.md-text-field .tf-prefix .ms,
.md-text-field .tf-suffix .ms { font-size: 24px; }
.md-text-field.focused .tf-prefix { color: var(--md-primary); }
.md-text-field.error .tf-prefix { color: var(--md-error); }
.md-text-field .tf-input-wrap {
  flex: 1 1 0;
  min-width: 0;
  position: relative;
  display: flex;
  align-items: center;
}
.md-text-field input,
.md-text-field textarea {
  width: 100%;
  border: none;
  outline: none;
  background: transparent;
  padding: 16px 0 14px;
  font-size: 16px;
  line-height: 24px;
  color: var(--md-on-surface);
  position: relative;
  z-index: 1;
}
.md-text-field textarea { resize: none; }
.md-text-field.disabled input,
.md-text-field.disabled textarea { color: var(--md-on-surface-038); }
.md-text-field .tf-label {
  position: absolute;
  left: 0;
  top: 0;
  height: 56px;
  display: flex;
  align-items: center;
  font-size: 16px;
  line-height: 24px;
  color: var(--md-on-surface-variant);
  pointer-events: none;
  transition: all 120ms ease;
  z-index: 2;
  background: transparent;
  max-width: calc(100% - 8px);
  overflow: hidden;
  white-space: nowrap;
}
.md-text-field.float-label .tf-label {
  top: 0;
  transform: translateY(-9px);
  height: auto;
  font-size: 12px;
  line-height: 16px;
  background: var(--md-surface);
  padding: 0 4px;
  margin-left: -4px;
}
.md-text-field.focused .tf-label { color: var(--md-primary); }
.md-text-field.error .tf-label { color: var(--md-error); }
.md-text-field.disabled .tf-label { color: var(--md-on-surface-038); }
.md-text-field .tf-helper {
  font-size: 12px;
  line-height: 16px;
  color: var(--md-on-surface-variant);
  padding: 4px 16px 0 16px;
}
.md-text-field .tf-error-text {
  font-size: 12px;
  line-height: 16px;
  color: var(--md-error);
  padding: 4px 16px 0 16px;
}
.md-text-field.dense input { padding: 10px 0 8px; }
.md-text-field.dense { min-height: 48px; }

/* ---------- 对话框（M3 emphasized：scrim 淡入 240ms + 面板缩放 300ms） ---------- */
.md-dialog-scrim {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.32);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  animation: scrim-in 240ms ease;
}
.md-dialog-scrim.closing {
  animation: scrim-out 200ms ease forwards;
  pointer-events: none; /* 收场中不拦截点击 */
}
@keyframes scrim-in {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes scrim-out {
  from { opacity: 1; }
  to { opacity: 0; }
}
.md-dialog {
  background: var(--md-surface-container-high);
  border-radius: 28px;
  box-shadow: 0 3px 3px rgba(0,0,0,.3), 0 12px 24px 6px rgba(0,0,0,.15);
  max-width: min(560px, 90vw);
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  min-width: 280px;
  overflow: hidden;
  position: relative;
  animation: md-dialog-in 300ms cubic-bezier(0.05, 0.7, 0.1, 1);
}
.md-dialog.closing {
  animation: md-dialog-out 200ms cubic-bezier(0.3, 0, 0.8, 0.15) forwards;
}
@keyframes md-dialog-in {
  from { opacity: 0; transform: scale(0.94) translateY(8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}
@keyframes md-dialog-out {
  from { opacity: 1; transform: scale(1) translateY(0); }
  to { opacity: 0; transform: scale(0.96) translateY(4px); }
}
.md-dialog .dialog-title {
  padding: 24px 24px 0;
  font-size: 24px;
  line-height: 32px;
  color: var(--md-on-surface);
}
.md-dialog .dialog-content {
  padding: 16px 24px;
  font-size: 14px;
  line-height: 20px;
  color: var(--md-on-surface);
  overflow: auto;
}
.md-dialog .dialog-actions {
  padding: 0 24px 24px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}
.md-dialog .dialog-actions.no-padding { padding: 0 24px 24px; }

/* ---------- 菜单（M3 Menu：elevation 3、项内边距 12；弹出 emphasized-decelerate 240ms） ---------- */
.md-menu {
  background: var(--md-surface-container);
  border-radius: 4px;
  box-shadow: 0 1px 3px rgba(0,0,0,.3), 0 4px 8px 3px rgba(0,0,0,.15);
  padding: 0;
  min-width: 112px;
  max-width: 360px;
  max-height: 480px;
  overflow: auto;
  z-index: 1200;
  position: fixed;
  transform-origin: top right;
  animation: md-menu-in 240ms cubic-bezier(0.05, 0.7, 0.1, 1);
}
@keyframes md-menu-in {
  from { opacity: 0; transform: scale(0.92); }
  to { opacity: 1; transform: scale(1); }
}
/* 关闭：收场淡出缩小（IconMenu 加 .closing 后延迟卸载） */
.md-menu.closing {
  animation: md-menu-out 160ms cubic-bezier(0.3, 0, 0.8, 0.15) forwards;
  pointer-events: none;
}
@keyframes md-menu-out {
  from { opacity: 1; transform: scale(1); }
  to { opacity: 0; transform: scale(0.94); }
}
.md-menu-item {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 48px;
  padding: 0 12px;
  cursor: pointer;
  font-size: 16px;
  line-height: 24px;
  color: var(--md-on-surface);
}
.md-menu-item:hover { background: var(--md-on-surface-08); }
.md-menu-item.disabled { color: var(--md-on-surface-038); pointer-events: none; }
.md-menu-item .ms { font-size: 24px; color: var(--md-on-surface-variant); }

/* ---------- DropdownButtonFormField ---------- */
.md-dropdown {
  position: relative;
  display: flex;
  align-items: center;
  min-height: 56px;
  border: 1px solid var(--md-outline);
  border-radius: 4px;
  cursor: pointer;
  transition: border-color 100ms ease;
}
.md-dropdown:hover { border-color: var(--md-on-surface); }
.md-dropdown.disabled { border-color: var(--md-on-surface-012); background: var(--md-on-surface-04); cursor: default; }
.md-dropdown .dd-value {
  flex: 1 1 0;
  min-width: 0;
  padding: 16px 4px 14px 12px;
  font-size: 16px;
  line-height: 24px;
  color: var(--md-on-surface);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.md-dropdown .dd-arrow {
  padding-right: 12px;
  color: var(--md-on-surface-variant);
  display: inline-flex;
}
.md-dropdown .dd-arrow .ms { font-size: 24px; }
.md-dropdown .dd-label {
  position: absolute;
  left: 12px;
  top: -9px;
  font-size: 12px;
  line-height: 16px;
  background: var(--md-surface);
  padding: 0 4px;
  color: var(--md-on-surface-variant);
  z-index: 2;
}
.md-dropdown.compact {
  min-height: 40px;
  border-radius: 8px;
}
.md-dropdown.compact .dd-value { padding: 8px 4px 8px 12px; font-size: 14px; }
.md-dropdown.compact .dd-label { display: none; }

/* ---------- Snackbar（floating） ---------- */
.md-snackbar-host {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 16px;
  display: flex;
  justify-content: center;
  z-index: 2000;
  pointer-events: none;
}
.md-snackbar {
  pointer-events: auto;
  background: var(--flutter-red-700);
  color: #ffffff;
  border-radius: 4px;
  min-height: 48px;
  box-shadow: 0 2px 2px rgba(0,0,0,.3), 0 6px 12px 4px rgba(0,0,0,.15);
  display: flex;
  align-items: center;
  padding: 14px 16px;
  gap: 8px;
  max-width: min(640px, calc(100vw - 32px));
  animation: snackbar-in 260ms cubic-bezier(0.05, 0.7, 0.1, 1);
}
.md-snackbar.success { background: var(--flutter-green-700); }
.md-snackbar.info { background: var(--flutter-blue-700); }
.md-snackbar.error { background: var(--flutter-red-700); }
.md-snackbar .snack-icon { color: #ffffff; display: inline-flex; }
.md-snackbar .snack-body { min-width: 0; }
.md-snackbar .snack-title { font-weight: 700; color: #fff; }
.md-snackbar .snack-message { font-size: 12px; line-height: 16px; color: #fff; margin-top: 2px; word-break: break-word; }
.md-snackbar .snack-action {
  color: #ffffff;
  font-weight: 500;
  background: none;
  border: none;
  cursor: pointer;
  padding: 8px;
  margin-left: 4px;
  white-space: nowrap;
  font-size: 14px;
}
@keyframes snackbar-in {
  from { transform: translateY(16px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

/* ---------- 进度指示器 ---------- */
.md-progress-linear {
  height: 4px;
  border-radius: 4px;
  background: var(--md-surface-container-highest);
  overflow: hidden;
  position: relative;
  width: 100%;
}
.md-progress-linear .bar {
  position: absolute;
  inset: 0 auto 0 0;
  background: var(--md-primary);
}
.md-progress-linear.indeterminate .bar {
  width: 40%;
  animation: linear-indeterminate 1.8s cubic-bezier(0.65, 0.815, 0.735, 1) infinite;
}
@keyframes linear-indeterminate {
  0% { left: -40%; width: 40%; }
  60% { left: 100%; width: 60%; }
  100% { left: 100%; width: 60%; }
}
.md-progress-circular {
  display: inline-block;
  position: relative;
  color: var(--md-primary);
}
.md-progress-circular svg {
  display: block;
  transform: rotate(-90deg);
}
.md-progress-circular circle {
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
}
.md-progress-circular .track { stroke: transparent; }

/* ---------- Tooltip（M3 plain tooltip） ---------- */
[data-tip] {
  position: relative;
}
[data-tip]:not([data-tip=""]):hover::after {
  content: attr(data-tip);
  position: absolute;
  left: 50%;
  top: calc(100% + 4px);
  transform: translateX(-50%);
  background: #313034;
  color: #f2f2f2;
  font-size: 12px;
  line-height: 16px;
  padding: 4px 8px;
  border-radius: 4px;
  white-space: nowrap;
  z-index: 3000;
  pointer-events: none;
  box-shadow: 0 1px 2px rgba(0,0,0,.3), 0 1px 3px 1px rgba(0,0,0,.15);
  font-weight: 400;
}

/* ---------- SegmentedButton ---------- */
.md-segmented {
  display: inline-flex;
  border: 1px solid var(--md-outline);
  border-radius: 20px;
  overflow: hidden;
}
.md-segmented .segment {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 40px;
  padding: 0 20px;
  font-size: 14px;
  font-weight: 500;
  color: var(--md-on-surface);
  cursor: pointer;
  border: none;
  background: transparent;
  position: relative;
}
.md-segmented .segment + .segment { border-left: 1px solid var(--md-outline); }
.md-segmented .segment.selected {
  background: transparent;
  color: var(--md-on-secondary-container);
}
/* M3 选中段为全圆角（stadium）胶囊填充 */
.md-segmented .segment.selected::after {
  content: '';
  position: absolute;
  inset: 2px;
  border-radius: 999px;
  background: var(--md-secondary-container);
  z-index: -1;
}

/* ---------- ExpansionTile ---------- */
.md-expansion {
  border-radius: 12px;
  border: 1px solid var(--md-outline-variant);
  background: var(--md-surface);
  overflow: hidden;
}
.md-expansion .expansion-header {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 56px;
  padding: 0 16px 0 20px;
  cursor: pointer;
}
.md-expansion .expansion-header .title { flex: 1 1 0; }
.md-expansion .chev { transition: transform 200ms ease; color: var(--md-on-surface-variant); }
.md-expansion.open .chev { transform: rotate(180deg); }

/* ---------- 悬停水波（近似 InkWell hover） ---------- */
.hover-primary:hover { background: var(--md-primary-08); }
.hover-surface:hover { background: var(--md-on-surface-08); }

/* ---------- 徽章通用 ---------- */
.badge {
  display: inline-flex;
  align-items: center;
  border-radius: 4px;
}

/* ---------- IndexedStack 等价 ---------- */
.stack-page { height: 100%; overflow: hidden; }
.stack-page.hidden { display: none; }

/* ---------- 布局残差 ---------- */
.no-scroll { overflow: hidden; }
.scroll-y { overflow-y: auto; overflow-x: hidden; }
.scroll-x { overflow-x: auto; overflow-y: hidden; }

/* ============================================================
 * 自定义背景（设置 → 软件设置 → 自定义背景）
 * html.has-custom-bg 由 App.svelte 按设置切换；
 * --astral-bg-image 为 astralbg:// 协议 URL（Tauri，磁盘直服务，
 * 任意大小、GIF 自动逐帧播放）或 Blob URL（浏览器预览）。
 * 分层策略：
 *   · .main-screen 承载图像 + 半透明 surface 遮罩（保文字可读）；
 *   · 主页面容器转透明 → 图像直达内容层；
 *   · 侧边栏 / 底部导航半透明 → 液态玻璃组件的 backdrop-filter
 *     直接折射/模糊背后的图像，玻璃质感最大化；
 *   · 子页（route-overlay / sub-page）高不透明度 surface → 轻微透出。
 * ============================================================ */
html.has-custom-bg body {
  background: var(--md-surface); /* 遮罩之外兜底 */
}
html.has-custom-bg .main-screen {
  background-image:
    linear-gradient(
      color-mix(in srgb, var(--md-surface) 50%, transparent),
      color-mix(in srgb, var(--md-surface) 50%, transparent)
    ),
    var(--astral-bg-image);
  background-size: cover, cover;
  background-position: center, center;
  background-repeat: no-repeat, no-repeat;
}
/* 主页面容器：让图像直达内容层（其余页面根默认无背景；
 * 房间/成员等推入页由 route-overlay/sub-page 半透明层承接） */
html.has-custom-bg .main-screen .home-page {
  background: transparent;
}
/* 侧边栏：半透明轨道 —— 玻璃选中框的折射带能吃到背景图像 */
html.has-custom-bg .left-nav {
  background: color-mix(in srgb, var(--md-surface) 64%, transparent);
}
/* 底部导航（移动端/窄窗）：同理半透明 */
html.has-custom-bg .bottom-nav {
  background: color-mix(in srgb, var(--md-surface-container-low) 66%, transparent);
}
/* 子页路由层 + 子页外壳：高不透明度 surface，可读性优先、轻微透出图像 */
html.has-custom-bg .route-overlay {
  background: color-mix(in srgb, var(--md-surface) 84%, transparent);
}
html.has-custom-bg .sub-page {
  background: color-mix(in srgb, var(--md-surface) 88%, transparent);
}
  }
</style>

