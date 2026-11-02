/**
 * Rust 核心 API 桥接层。
 *
 * Tauri 运行时：invoke() 调用 src-tauri 中的命令（复用原 rust/ 网络核心）。
 * 浏览器预览：内置 Mock 模拟器，完整模拟连接状态机、节点数据、
 * NAT 检测、防火墙、魔法墙等行为，保证 UI 可演示可截图。
 */
import { isTauri } from '../utils/platform.js';

const tauriInvoke = async (cmd, args) => {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke(cmd, args);
};

/* ================================================================
 *  Mock 模拟器（浏览器模式）
 * ================================================================ */
const mock = {
  serverRunning: false,
  connectedAt: 0,
  virtualIp: '0.0.0.0',
  nodes: [],
  firewall: [true, true, true],
  magicWall: { running: false, rules: new Map() },
  startTime: Date.now(),
};

const MOCK_PLAYERS = [
  { hostname: 'Kevin-PC', ip: null, nat: 'FullCone', latency: [12, 30], connType: 'p2p', cost: 1 },
  { hostname: '浪浪云的主机', nat: 'Restricted', latency: [35, 68], connType: 'p2p', cost: 1 },
  { hostname: '小探GamePC', nat: 'Symmetric', latency: [88, 160], connType: 'relay', cost: 2 },
  { hostname: 'xiaoming-Laptop', nat: 'OpenInternet', latency: [8, 20], connType: 'p2p', cost: 1 },
  { hostname: 'SteamDeck-Player', nat: 'PortRestricted', latency: [45, 90], connType: 'p2p', cost: 1 },
  { hostname: '网吧七号机', nat: 'SymUdpFirewall', latency: [110, 210], connType: 'relay', cost: 3 },
  { hostname: 'TT_HOST_SERVER', nat: 'Unknown', latency: [20, 45], connType: 'server', cost: 2 },
];

function randBetween(min, max) {
  return min + Math.random() * (max - min);
}

function buildMockNodes(localIp) {
  const nodes = [];
  let peerId = 2;
  // 本机节点
  nodes.push({
    peerId: 1,
    hostname: 'local-player',
    ipv4: localIp,
    latencyMs: 0,
    nat: 'Unknown',
    hops: [],
    lossRate: 0,
    connections: [
      { connType: 'tcp', rxBytes: 1024 * 512, txBytes: 1024 * 1024, rxPackets: 3200, txPackets: 6400 },
    ],
    tunnelProto: '',
    connType: '',
    rxBytes: 1024 * 512,
    txBytes: 1024 * 1024,
    version: '2.6.4|browser',
    cost: 0,
    proxyCidrs: [],
  });
  for (const p of MOCK_PLAYERS) {
    const isServer = p.connType === 'server';
    const latency = randBetween(p.latency[0], p.latency[1]);
    const loss = Math.random() * 3;
    const rx = Math.floor(Math.random() * 500 * 1024 * 1024);
    const tx = Math.floor(Math.random() * 300 * 1024 * 1024);
    const ip = isServer ? '0.0.0.0' : `10.126.0.${peerId}`;
    nodes.push({
      peerId,
      hostname: isServer ? `PublicServer_${p.hostname}` : p.hostname,
      ipv4: ip,
      latencyMs: latency,
      nat: p.nat,
      hops:
        p.connType === 'relay'
          ? [
              {
                peerId: 100,
                targetIp: '10.126.0.100',
                latencyMs: latency * 0.5,
                packetLoss: loss * 0.4,
                nodeName: '中转服务器',
              },
            ]
          : [],
      lossRate: loss,
      connections: [
        {
          connType: 'tcp',
          rxBytes: rx,
          txBytes: tx,
          rxPackets: Math.floor(rx / 1400),
          txPackets: Math.floor(tx / 1400),
        },
      ],
      tunnelProto: Math.random() > 0.5 ? 'tcp,udp' : 'udp',
      connType: '',
      rxBytes: rx,
      txBytes: tx,
      version: '2.6.4|windows',
      cost: p.cost,
      proxyCidrs: [],
    });
    peerId += 1;
  }
  return nodes;
}

function jitterNodes() {
  mock.nodes = mock.nodes.map((n) => {
    if (n.peerId === 1) return n;
    const factor = 0.9 + Math.random() * 0.25;
    return {
      ...n,
      latencyMs: Math.max(2, n.latencyMs * factor),
      lossRate: Math.max(0, Math.min(8, n.lossRate + (Math.random() - 0.5))),
      rxBytes: n.rxBytes + Math.floor(Math.random() * 80000),
      txBytes: n.txBytes + Math.floor(Math.random() * 60000),
      connections: [
        {
          ...n.connections[0],
          rxBytes: n.rxBytes + Math.floor(Math.random() * 80000),
          txBytes: n.txBytes + Math.floor(Math.random() * 60000),
        },
      ],
    };
  });
}

setInterval(() => {
  if (mock.serverRunning && mock.nodes.length) jitterNodes();
}, 1000).unref?.();

/* ================================================================
 *  API 面（与 lib/src/rust/api/*.dart 一一对应）
 * ================================================================ */

/** easytier 版本，格式 "版本号|平台名" */
export async function easytierVersion() {
  if (isTauri()) return tauriInvoke('easytier_version');
  return '2.6.4|browser';
}

/** 运行信息 JSON 字符串（含 my_node_info.virtual_ipv4.address.addr 整数） */
export async function getRunningInfo() {
  if (isTauri()) return tauriInvoke('get_running_info');
  if (!mock.serverRunning) return 'null';
  const ipInt = mock.virtualIp === '0.0.0.0'
    ? 0
    : mock.virtualIp
        .split('.')
        .map(Number)
        .reduce((acc, part) => (acc * 256) + part, 0);
  return JSON.stringify({
    dev_name: 'astral',
    my_node_info: mock.virtualIp === '0.0.0.0'
      ? null
      : { virtual_ipv4: { address: { addr: ipInt } } },
    routes: [],
    peer_route_pairs: [],
  });
}

/** KVNetworkStatus */
export async function getNetworkStatus() {
  if (isTauri()) return tauriInvoke('get_network_status');
  if (!mock.serverRunning) return { totalNodes: 0, nodes: [] };
  return { totalNodes: mock.nodes.length, nodes: mock.nodes };
}

/**
 * 创建虚拟网络服务器。
 * 参数与 ServerConfigBuilder.build() 输出一致。
 */
export async function createServer(config) {
  if (isTauri()) {
    return tauriInvoke('create_server', { config });
  }
  // Mock：启动连接流程
  mock.serverRunning = true;
  mock.virtualIp = '0.0.0.0';
  mock.nodes = [];
  const useDhcp = config.enableDhcp;
  const specified = config.specifiedIp || '';
  setTimeout(() => {
    if (!mock.serverRunning) return;
    mock.virtualIp = useDhcp || !specified ? '10.126.0.1' : specified.split('/')[0];
    mock.nodes = buildMockNodes(mock.virtualIp);
  }, 1600 + Math.random() * 1200);
  return null;
}

/** 关闭服务器 */
export async function closeServer() {
  if (isTauri()) return tauriInvoke('close_server');
  mock.serverRunning = false;
  mock.virtualIp = '0.0.0.0';
  mock.nodes = [];
  return null;
}

/** Windows 网卡跃点列表 [(名称, metric)] */
export async function getAllInterfacesMetrics() {
  if (isTauri()) return tauriInvoke('get_all_interfaces_metrics');
  return [
    ['astral', 0],
    ['以太网', 25],
    ['WLAN', 50],
    ['Loopback Pseudo-Interface 1', 75],
  ];
}

/** 设置网卡跃点 */
export async function setInterfaceMetric(interfaceName, metric) {
  if (isTauri()) return tauriInvoke('set_interface_metric', { interfaceName, metric });
  return null;
}

/** Windows 防火墙状态（profileIndex 1/2/3） */
export async function getFirewallStatus(profileIndex) {
  if (isTauri()) return tauriInvoke('get_firewall_status', { profileIndex });
  return mock.firewall[profileIndex - 1] ?? false;
}

export async function setFirewallStatus(profileIndex, enable) {
  if (isTauri()) return tauriInvoke('set_firewall_status', { profileIndex, enable });
  mock.firewall[profileIndex - 1] = enable;
  return null;
}

/** NAT 测试 */
export async function testNetworkConnectivity(stunServer) {
  if (isTauri()) return tauriInvoke('test_network_connectivity', { stunServer });
  await new Promise((r) => setTimeout(r, 1500 + Math.random() * 1500));
  return {
    natTypeV4: Math.random() > 0.5 ? 'Restricted Cone NAT' : 'Full Cone NAT',
    natTypeV6: Math.random() > 0.3 ? 'Symmetric NAT' : 'Port Restricted Cone NAT',
    ipv4Latency: Math.floor(randBetween(12, 90)),
    ipv6Latency: Math.random() > 0.5 ? Math.floor(randBetween(15, 120)) : -1,
  };
}

/* ---------------- 魔法墙 API ---------------- */
export async function startMagicWall() {
  if (isTauri()) return tauriInvoke('start_magic_wall');
  if (mock.magicWall.running) throw new Error('魔法墙已经在运行');
  mock.magicWall.running = true;
  return null;
}
export async function stopMagicWall() {
  if (isTauri()) return tauriInvoke('stop_magic_wall');
  if (!mock.magicWall.running) throw new Error('魔法墙未运行');
  mock.magicWall.running = false;
  mock.magicWall.rules.clear();
  return null;
}
export async function addMagicWallRule(rule) {
  if (isTauri()) return tauriInvoke('add_magic_wall_rule', { rule });
  if (mock.magicWall.rules.has(rule.id)) {
    mock.magicWall.rules.set(rule.id, rule);
    return null;
  }
  mock.magicWall.rules.set(rule.id, rule);
  return null;
}
export async function removeMagicWallRule(ruleId) {
  if (isTauri()) return tauriInvoke('remove_magic_wall_rule', { ruleId });
  if (!mock.magicWall.rules.has(ruleId)) throw new Error('规则不存在');
  mock.magicWall.rules.delete(ruleId);
  return null;
}
export async function updateMagicWallRule(rule) {
  if (isTauri()) return tauriInvoke('update_magic_wall_rule', { rule });
  mock.magicWall.rules.set(rule.id, rule);
  return null;
}
export async function getMagicWallStatus() {
  if (isTauri()) return tauriInvoke('get_magic_wall_status');
  let active = 0;
  for (const r of mock.magicWall.rules.values()) if (r.enabled) active += 1;
  return {
    isRunning: mock.magicWall.running,
    activeRules: active,
    totalRules: mock.magicWall.rules.size,
  };
}

/* ---------------- 其它工具 ---------------- */

/** 打开外部链接（原 url_launcher） */
export async function openExternal(url) {
  if (isTauri()) {
    try {
      const { open } = await import('@tauri-apps/plugin-opener');
      await open(url);
      return true;
    } catch {
      return tauriInvoke('open_external', { url });
    }
  }
  window.open(url, '_blank');
  return true;
}

/** 检查 Windows Npcap 驱动（原 ConnectionConnectGuard.hasNpcapDriver） */
export async function hasNpcapDriver() {
  if (isTauri()) return tauriInvoke('has_npcap_driver');
  return true; // 浏览器模式视为就绪
}

/** 监听 Rust 事件日志（原 UDP 127.0.0.1:9999 通道 → Tauri 事件） */
export function listenRustLogs(callback) {
  if (isTauri()) {
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen('rust-log', (event) => callback(event.payload));
    });
    return () => {};
  }
  // Mock：连接时模拟若干事件
  window.__astralMockLog = callback;
  return () => {
    delete window.__astralMockLog;
  };
}

/** Mock 模式下向日志通道推送一条事件（模拟 Rust handle_event） */
export function pushMockLog(text) {
  if (typeof window !== 'undefined' && window.__astralMockLog) {
    window.__astralMockLog(text);
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
