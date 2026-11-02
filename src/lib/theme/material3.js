/**
 * Material 3 主题引擎 —— 与 Flutter `ThemeData(colorSchemeSeed: ..., useMaterial3: true)` 100% 等价。
 *
 * Flutter 内部同样使用 Material Color Utilities（Dart 版）的 SchemeTonalSpot
 * 生成色彩方案；此处使用官方 JS 版 @material/material-color-utilities，
 * 并按 Flutter 3.22+ 的 tone 映射补全 surfaceContainer 系列令牌：
 *   light: lowest=100 low=96 container=94 high=92 highest=90 dim=87 bright=98
 *   dark:  lowest=4  low=10 container=12 high=17 highest=21 dim=6  bright=24
 */
import {
  themeFromSourceColor,
  argbFromHex,
  hexFromArgb,
  rgbaFromArgb,
} from '@material/material-color-utilities';

/** Flutter ColorScheme.fromSeed 的 surface 容器 tone 映射 */
const SURFACE_TONES = {
  light: {
    surfaceContainerLowest: 100,
    surfaceContainerLow: 96,
    surfaceContainer: 94,
    surfaceContainerHigh: 92,
    surfaceContainerHighest: 90,
    surfaceDim: 87,
    surfaceBright: 98,
  },
  dark: {
    surfaceContainerLowest: 4,
    surfaceContainerLow: 10,
    surfaceContainer: 12,
    surfaceContainerHigh: 17,
    surfaceContainerHighest: 21,
    surfaceDim: 6,
    surfaceBright: 24,
  },
};

/** Scheme 基础字段 → CSS 变量名（kebab-case） */
const SCHEME_KEYS = [
  'primary',
  'onPrimary',
  'primaryContainer',
  'onPrimaryContainer',
  'secondary',
  'onSecondary',
  'secondaryContainer',
  'onSecondaryContainer',
  'tertiary',
  'onTertiary',
  'tertiaryContainer',
  'onTertiaryContainer',
  'error',
  'onError',
  'errorContainer',
  'onErrorContainer',
  'background',
  'onBackground',
  'surface',
  'onSurface',
  'surfaceVariant',
  'onSurfaceVariant',
  'outline',
  'outlineVariant',
  'shadow',
  'scrim',
  'inverseSurface',
  'inverseOnSurface',
  'inversePrimary',
];

function kebab(name) {
  return name.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase());
}

/**
 * 生成完整色彩方案对象（含 surfaceContainer 系列）。
 * @param {string} seedHex 种子色，如 '#2196F3'
 * @returns {{light: object, dark: object}}
 */
export function buildSchemes(seedHex) {
  const theme = themeFromSourceColor(argbFromHex(seedHex));
  const result = {};
  for (const mode of ['light', 'dark']) {
    const scheme = theme.schemes[mode];
    const colors = {};
    for (const key of SCHEME_KEYS) {
      colors[key] = hexFromArgb(scheme[key]);
    }
    // surfaceContainer 系列（与 Flutter 一致的 tone）
    const neutral = theme.palettes.neutral;
    for (const [name, tone] of Object.entries(SURFACE_TONES[mode])) {
      colors[name] = hexFromArgb(neutral.tone(tone));
    }
    // Flutter 3.22+ 的 surfaceTint = primary
    colors.surfaceTint = colors.primary;
    // scrim 默认黑色
    colors.scrim = '#000000';
    result[mode] = colors;
  }
  return result;
}

/**
 * 将方案应用为 documentElement 上的 CSS 变量。
 * 同时输出若干 JS 侧需要的派生色（带透明度的常用组合）。
 */
export function applyTheme(colors, mode) {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(colors)) {
    root.style.setProperty(`--md-${kebab(key)}`, value);
  }
  root.style.setProperty('--md-surface-tint', colors.primary);
  root.setAttribute('data-theme-mode', mode);
  // 透明度辅助变量（Flutter 中 withValues(alpha: x) 的常用组合）
  const hex = (c) => colors[c];
  root.style.setProperty('--md-primary-06', withAlpha(hex('primary'), 0.06));
  root.style.setProperty('--md-primary-01', withAlpha(hex('primary'), 0.1));
  root.style.setProperty('--md-primary-02', withAlpha(hex('primary'), 0.2));
  root.style.setProperty('--md-primary-03', withAlpha(hex('primary'), 0.3));
  root.style.setProperty('--md-primary-06b', withAlpha(hex('primary'), 0.6));
  root.style.setProperty(
    '--md-surface-container-highest-05',
    withAlpha(hex('surfaceContainerHighest'), 0.5)
  );
  root.style.setProperty(
    '--md-surface-container-highest-015',
    withAlpha(hex('surfaceContainerHighest'), 0.15)
  );
  root.style.setProperty(
    '--md-surface-container-highest-012',
    withAlpha(hex('surfaceContainerHighest'), 0.12)
  );
  root.style.setProperty(
    '--md-surface-container-highest-095',
    withAlpha(hex('surfaceContainerHighest'), 0.95)
  );
  root.style.setProperty('--md-outline-02', withAlpha(hex('outline'), 0.2));
  root.style.setProperty('--md-outline-03', withAlpha(hex('outline'), 0.3));
  root.style.setProperty('--md-on-surface-06', withAlpha(hex('onSurface'), 0.6));
  root.style.setProperty('--md-on-surface-07', withAlpha(hex('onSurface'), 0.7));
  root.style.setProperty(
    '--md-primary-container-012',
    withAlpha(hex('primaryContainer'), 0.12)
  );
  root.style.setProperty(
    '--md-primary-container-04',
    withAlpha(hex('primaryContainer'), 0.4)
  );
  root.style.setProperty('--md-on-surface-012', withAlpha(hex('onSurface'), 0.12));
  root.style.setProperty('--md-on-surface-038', withAlpha(hex('onSurface'), 0.38));
  root.style.setProperty('--md-on-surface-045', withAlpha(hex('onSurface'), 0.45));
  // M3 状态层（InkWell hover 8% / pressed 10% / 按钮 hover primary 8% / 禁用容器 4%）
  root.style.setProperty('--md-on-surface-08', withAlpha(hex('onSurface'), 0.08));
  root.style.setProperty('--md-on-surface-04', withAlpha(hex('onSurface'), 0.04));
  root.style.setProperty('--md-on-surface-10', withAlpha(hex('onSurface'), 0.1));
  root.style.setProperty('--md-primary-08', withAlpha(hex('primary'), 0.08));
}

/** #RRGGBB + alpha → rgba() 字符串 */
export function withAlpha(hexColor, alpha) {
  const c = hexColor.replace('#', '');
  const r = parseInt(c.substring(0, 2), 16);
  const g = parseInt(c.substring(2, 4), 16);
  const b = parseInt(c.substring(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** 读取当前生效的 CSS 变量颜色（供 canvas / JS 绘制使用） */
export function cssVar(name) {
  return getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
}

export { hexFromArgb, argbFromHex };
