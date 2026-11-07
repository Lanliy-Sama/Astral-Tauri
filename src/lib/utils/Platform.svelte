<script module>
/** 平台检测（Svelte 5 模块）。 */
/**
 * 平台检测工具 —— 替代 Flutter 的 Platform.isXXX。
 * Tauri 环境读取 OS 信息；浏览器预览模式下默认按 Windows 桌面处理
 * （原应用在桌面平台行为一致，仅 Windows 有少量专属 UI）。
 */

let cachedPlatform = null;

export function isTauri() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function detectPlatform() {
  if (cachedPlatform) return cachedPlatform;
  if (isTauri()) {
    try {
      const { platform } = await import('@tauri-apps/plugin-os');
      cachedPlatform = await platform();
    } catch {
      const { invoke } = await import('@tauri-apps/api/core');
      cachedPlatform = await invoke('plugin:os|platform');
    }
  } else {
    cachedPlatform = 'browser';
  }
  return cachedPlatform;
}

export function platformSync() {
  return cachedPlatform || 'browser';
}

export async function initPlatform() {
  return detectPlatform();
}

export function isWindows() {
  return platformSync() === 'windows';
}
export function isLinux() {
  return platformSync() === 'linux';
}
export function isMacOS() {
  return platformSync() === 'macos';
}
export function isAndroid() {
  return platformSync() === 'android';
}
export function isIOS() {
  return platformSync() === 'ios';
}
export function isDesktopPlatform() {
  const p = platformSync();
  return p === 'windows' || p === 'linux' || p === 'macos';
}
export function isMobilePlatform() {
  const p = platformSync();
  return p === 'android' || p === 'ios';
}

/** 浏览器/系统语言（近似 device_info_plus + locale 检测） */
export function browserLocale() {
  try {
    return navigator.language || 'zh';
  } catch {
    return 'zh';
  }
}

/** 本机主机名（Dart Platform.localHostname 的近似） */
export function localHostname() {
  try {
    return (
      window.__astralHostname ||
      (navigator.userAgentData && navigator.userAgentData.platform) ||
      'localhost'
    ).toLowerCase();
  } catch {
    return 'localhost';
  }
}
</script>
