/**
 * 主题应用器 —— 生成并注入 M3 色彩令牌。
 * 对应 Flutter: MaterialApp(colorSchemeSeed, brightness, themeMode)。
 *
 * 注意：$effect 必须在组件内运行（见 App.svelte），
 * 此模块只提供纯函数与系统主题监听。
 */
import { buildSchemes, applyTheme } from '../theme/material3.js';
import { sm } from '../services/service-manager.svelte.js';

let mediaQuery = null;

export function initThemeEngine() {
  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  mediaQuery.addEventListener('change', () => {
    if (sm.themeState.themeMode === 'system') applyThemeNow();
  });
}

export function resolveMode() {
  const mode = sm.themeState.themeMode;
  if (mode === 'light') return 'light';
  if (mode === 'dark') return 'dark';
  return mediaQuery && mediaQuery.matches ? 'dark' : 'light';
}

/** 立即应用当前主题（幂等） */
export function applyThemeNow() {
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
export function applyDebugThemeOverride() {
  const params = new URLSearchParams(location.search);
  const forced = params.get('theme');
  if (forced === 'dark' || forced === 'light') {
    sm.themeState.themeMode = forced;
  }
}
