<script module>
/** Rust 核心 API 桥接层（Svelte 5 模块）。 */
/**
 * Rust 核心 API 桥接层。
 *
 * 所有调用都直接走 Tauri `invoke()`，桌面端与 Android 端共用同一套命令；
 * 不再包含任何浏览器 Mock 实现——非 Tauri 环境下调用会直接报错。
 */
import { isTauri } from '../utils/Platform.svelte';

const NOT_CONNECTED =
  '当前运行在浏览器中，未连接后端（请使用 Tauri 桌面端或 Android 端）';

const tauriInvoke = async (cmd, args) => {
  if (!isTauri()) throw new Error(NOT_CONNECTED);
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke(cmd, args);
};

/* ==================== EasyTier 核心 ==================== */

/** easytier 版本，格式 "版本号|平台名" */
export async function easytierVersion() {
  return tauriInvoke('easytier_version');
}

/** 运行信息 JSON 字符串（含 my_node_info.virtual_ipv4.address.addr 整数） */
export async function getRunningInfo() {
  return tauriInvoke('get_running_info');
}

/** KVNetworkStatus */
export async function getNetworkStatus() {
  return tauriInvoke('get_network_status');
}

/**
 * 创建虚拟网络服务器。
 * 参数与 ServerConfigBuilder.build() 输出一致。
 */
export async function createServer(config) {
  return tauriInvoke('create_server', { config });
}

/** 关闭服务器 */
export async function closeServer() {
  return tauriInvoke('close_server');
}

/* ==================== Android VPN ==================== */

/** 检查/申请系统 VPN 授权，返回 { granted } */
export async function vpnPrepare() {
  return tauriInvoke('vpn_prepare');
}

/**
 * 启动 VpnService 并取得 TUN fd。
 * @param {{ipv4Addr:string,routes:string[],dns?:string,disallowedApplications:string[],mtu:number}} options
 */
export async function vpnStart(options) {
  return tauriInvoke('vpn_start', { options });
}

/** 关闭 VpnService 并回收 TUN */
export async function vpnStop() {
  return tauriInvoke('vpn_stop');
}

/** 把 VpnService 的 TUN fd 交给 EasyTier 核心 */
export async function setTunFd(fd) {
  return tauriInvoke('set_tun_fd', { fd });
}

/* ==================== 防火墙 / 跃点 ==================== */

/** Windows 网卡跃点列表 [(名称, metric)] */
export async function getAllInterfacesMetrics() {
  return tauriInvoke('get_all_interfaces_metrics');
}

/** 设置网卡跃点 */
export async function setInterfaceMetric(interfaceName, metric) {
  return tauriInvoke('set_interface_metric', { interfaceName, metric });
}

/** Windows 防火墙状态（profileIndex 1/2/3） */
export async function getFirewallStatus(profileIndex) {
  return tauriInvoke('get_firewall_status', { profileIndex });
}

export async function setFirewallStatus(profileIndex, enable) {
  return tauriInvoke('set_firewall_status', { profileIndex, enable });
}

/* ==================== NAT 测试 ==================== */

export async function testNetworkConnectivity(stunServer) {
  return tauriInvoke('test_network_connectivity', { stunServer });
}

/* ==================== 魔法墙 ==================== */

export async function startMagicWall() {
  return tauriInvoke('start_magic_wall');
}

export async function stopMagicWall() {
  return tauriInvoke('stop_magic_wall');
}

export async function addMagicWallRule(rule) {
  return tauriInvoke('add_magic_wall_rule', { rule });
}

export async function removeMagicWallRule(ruleId) {
  return tauriInvoke('remove_magic_wall_rule', { ruleId });
}

export async function updateMagicWallRule(rule) {
  return tauriInvoke('update_magic_wall_rule', { rule });
}

export async function getMagicWallStatus() {
  return tauriInvoke('get_magic_wall_status');
}

/* ==================== 其它工具 ==================== */

/** 打开外部链接（原 url_launcher） */
export async function openExternal(url) {
  if (!isTauri()) {
    window.open(url, '_blank');
    return true;
  }
  try {
    const { open } = await import('@tauri-apps/plugin-opener');
    await open(url);
    return true;
  } catch {
    return tauriInvoke('open_external', { url });
  }
}

/** 检查 Windows Npcap 驱动（原 ConnectionConnectGuard.hasNpcapDriver） */
export async function hasNpcapDriver() {
  if (!isTauri()) return true;
  return tauriInvoke('has_npcap_driver');
}

/** TCP ping：超时/失败/>800ms 由后端返回 null */
export async function tcpPing(url, timeoutMs = 1000) {
  return tauriInvoke('tcp_ping', { url, timeoutMs });
}

/** 监听 Rust 事件日志（原 UDP 127.0.0.1:9999 通道 → Tauri 事件） */
export function listenRustLogs(callback) {
  if (!isTauri()) return () => {};
  let unlisten = null;
  let disposed = false;
  import('@tauri-apps/api/event')
    .then(({ listen }) => listen('rust-log', (event) => callback(event.payload)))
    .then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    })
    .catch(() => {});
  return () => {
    disposed = true;
    if (unlisten) unlisten();
  };
}

/** 前端日志落盘（fire-and-forget：诊断用，不抛错） */
export async function frontendLog(msg) {
  try {
    await tauriInvoke('frontend_log', { msg });
  } catch {
    /* 诊断日志失败忽略 */
  }
}

/** 写剪贴板 */
export async function writeClipboard(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** 读剪贴板 */
export async function readClipboard() {
  try {
    return await navigator.clipboard.readText();
  } catch {
    return '';
  }
}
</script>
