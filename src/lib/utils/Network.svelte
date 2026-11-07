<script module>
/** 网络工具（Svelte 5 模块）。 */
/**
 * 网络工具 —— 翻译自 lib/shared/utils/network/ip_utils.dart 等。
 */

/** 整数(主机序) → 点分 IPv4（原 intToIp） */
export function intToIp(addr) {
  if (addr == null || addr < 0) return '0.0.0.0';
  return [
    (addr >>> 24) & 255,
    (addr >>> 16) & 255,
    (addr >>> 8) & 255,
    addr & 255,
  ].join('.');
}

const IPV4_RE =
  /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;

/** 严格 IPv4 校验（排除 0.0.0.0 / 255.255.255.255，默认排除回环） */
export function isValidIpAddress(ip, { excludeLoopback = true } = {}) {
  if (!ip || !IPV4_RE.test(ip.trim())) return false;
  const t = ip.trim();
  if (t === '0.0.0.0' || t === '255.255.255.255') return false;
  if (excludeLoopback && t.startsWith('127.')) return false;
  return true;
}

/** IPv4(/mask) 校验（UserIpBox 输入校验规则） */
export function isValidIPv4WithMask(input) {
  if (!input) return false;
  const parts = input.split('/');
  if (parts.length > 2) return false;
  const ip = parts[0].trim();
  if (!IPV4_RE.test(ip)) return false;
  if (parts.length === 2) {
    const mask = Number(parts[1].trim());
    if (!Number.isInteger(mask) || mask < 0 || mask > 32) return false;
  }
  return true;
}

/** CIDR 校验 */
export function isValidCIDR(cidr) {
  if (!cidr) return false;
  const parts = cidr.split('/');
  if (parts.length !== 2) return false;
  if (!isValidIpAddress(parts[0].trim(), { excludeLoopback: false })) return false;
  if (['0.0.0.0', '255.255.255.255'].includes(parts[0].trim())) return false;
  const mask = Number(parts[1].trim());
  return Number.isInteger(mask) && mask >= 0 && mask <= 32;
}

/** TCP ping（Tauri：后端命令；浏览器：模拟） */
export async function pingHost(url, timeoutMs = 5000) {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke('tcp_ping', { url, timeoutMs });
    } catch {
      return null;
    }
  }
  // Mock：随机延迟，>800ms 视为离线（与原 PingUtil 一致）
  await new Promise((r) => setTimeout(r, 80 + Math.random() * 300));
  const ms = Math.random();
  if (ms > 0.85) return null;
  return Math.floor(20 + Math.random() * 700);
}

/** isServerNode 判定（node_utils.dart） */
export function isServerNode(node) {
  return (node.hostname || '').startsWith('PublicServer_') || node.ipv4 === '0.0.0.0';
}

/** BlockedServers 黑名单（blocked_servers.dart） */
export const BLOCKED_SERVERS = [
  'js.629957.xyz:11012',
  'nmg.629957.xyz:11010',
  'nmg.629957.xyz:11010',
];
export function isBlockedServer(url) {
  return BLOCKED_SERVERS.includes(url);
}
/** ServerDao 启动清理的废弃域名 */
export const DEPRECATED_DOMAINS = [
  'turn.bj.629957.xyz',
  'turn.js.629957.xyz',
  'turn.hn.629957.xyz',
  'turn.hb.629957.xyz',
  'turn.nmg.629957.xyz',
  'bj.629957.xyz',
  'nmg.629957.xyz',
  'hn.629957.xyz',
  'hb.629957.xyz',
];
export function isDeprecatedServer(url) {
  return DEPRECATED_DOMAINS.some((d) => (url || '').includes(d));
}
</script>
