<script module>
/**
 * ServiceManager —— 对应 lib/core/services/service_manager.dart 的全部装配。
 * 包含：数据库初始化、15 个状态、领域服务（房间/服务器/网络配置/设置/主题/防火墙）、
 * 连接管理器（ServerConnectionManager + 状态轮询 + 网络监视器）、更新检查、
 * 日志捕获（Tauri 事件 / Mock）、启动自动连接守卫。
 */
import * as db from '../db/Db.svelte';
import * as rust from '../rust/Api.svelte';
import { CoState } from '../stores/Enums.svelte';
import {
  isDeprecatedServer,
  isValidIpAddress,
  isValidCIDR,
  pingHost,
  intToIp,
} from '../utils/Network.svelte';
import {
  parseNetworkConfigShare,
  networkConfigShareFromState,
} from '../utils/RoomShareCodec.svelte';
import { localHostname, isTauri, isWindows, isAndroid } from '../utils/Platform.svelte';

/* ============ 状态类（原 src/lib/stores/states.svelte.js，已内联） ============ */
/**
 * 全部响应式状态（Svelte 5 runes 版 ServiceManager 状态层）。
 * 每个 class 与原 Dart 的 core/states/*.dart 字段一一对应。
 * （Svelte 5 的 $state 类字段会生成响应式代理，方法内 this.x 读写自动追踪）
 */

/* ============ ThemeState ============ */
export class ThemeState {
  themeColor = $state('#2196F3'); // 默认 Colors.blue
  themeMode = $state('system');
}

/* ============ UIState（临时态，不持久化） ============ */
export class UIState {
  selectedIndex = $state(0);
  hoveredIndex = $state(null);
  isDesktop = $state(false);
  isInBackground = $state(false);
  trayHidden = $state(false);

  goTo(tab) {
    this.selectedIndex = tab;
  }
  updateScreenSplitWidth(width) {
    this.isDesktop = width > 480;
  }
  setBackground(v) {
    this.isInBackground = v;
  }
}

/* ============ RoomState ============ */
export class RoomState {
  rooms = $state([]);
  selectedRoom = $state(null);
  setRooms(list) {
    this.rooms = list;
  }
}

/* ============ ServerState ============ */
export class ServerState {
  servers = $state([]);
  setServers(list) {
    this.servers = list;
  }
}

/* ============ NetworkConfigState（与 NetConfig 字段一致） ============ */
export class NetworkConfigState {
  instanceName = $state('default');
  ipv4 = $state('');
  dhcp = $state(true);
  defaultProtocol = $state('tcp');
  devName = $state('');
  enableEncryption = $state(true);
  enableIpv6 = $state(true);
  mtu = $state(1360);
  latencyFirst = $state(false);
  enableExitNode = $state(false);
  noTun = $state(false);
  enableSocks5 = $state(false);
  socks5Port = $state(1080);
  useSmoltcp = $state(false);
  relayNetworkWhitelist = $state('*');
  disableP2p = $state(false);
  enableUdpBroadcastRelay = $state(false);
  relayAllPeerRpc = $state(false);
  disableUdpHolePunching = $state(false);
  disableTcpHolePunching = $state(false);
  disableSymHolePunching = $state(false);
  multiThread = $state(true);
  dataCompressAlgo = $state(1);
  bindDevice = $state(true);
  enableKcpProxy = $state(true);
  disableKcpInput = $state(false);
  disableRelayKcp = $state(true);
  proxyForwardBySystem = $state(false);
  acceptDns = $state(false);
  privateMode = $state(false);
  enableQuicProxy = $state(false);
  disableQuicInput = $state(false);
  tcpWhitelist = $state('');
  udpWhitelist = $state('');
}

/* ============ PlayerState ============ */
export class PlayerState {
  playerName = $state('');
  listenList = $state(['tcp://0.0.0.0:0', 'udp://0.0.0.0:0']);
}

/* ============ DisplayState ============ */
export class DisplayState {
  sortOption = $state(0);
  sortOrder = $state(0);
  displayMode = $state(0);
  userListStyle = $state(0);

  get userListSimple() {
    return this.userListStyle === 0;
  }
}

/* ============ StartupState ============ */
export class StartupState {
  startup = $state(false);
  startupMinimize = $state(false);
  startupAutoConnect = $state(false);
}

/* ============ UpdateState ============ */
export class UpdateState {
  beta = $state(false);
  autoCheckUpdate = $state(true);
  downloadAccelerate = $state('auto');
  resolvedDownloadAccelerate = $state(null);
  latestVersion = $state(null);

  get hasNewVersion() {
    return this.latestVersion != null && this.latestVersion !== '';
  }
}

/* ============ ConnectionState ============ */
export class ConnectionState {
  connectionState = $state('idle');
  netStatus = $state(null);
}

/* ============ WindowState ============ */
export class WindowState {
  closeMinimize = $state(true);
}

/* ============ FirewallState ============ */
export class FirewallState {
  firewallStatus = $state(false);
  setFirewallStatus(v) {
    this.firewallStatus = v;
  }
}

/* ============ VpnState（自定义 VPN 网段） ============ */
export class VpnState {
  customVpn = $state([]);
  setCustomVpn(list) {
    this.customVpn = list;
  }
  addCustomVpn(value) {
    this.customVpn = [...this.customVpn, value];
  }
  removeCustomVpn(index) {
    const list = [...this.customVpn];
    list.splice(index, 1);
    this.customVpn = list;
  }
}

/* ============ AppSettingsState ============ */
export class AppSettingsState {
  appName = $state('Astral');
  enableConnectionNotification = $state(true);
  autoRetryOnFailure = $state(true);
  maxRetryCount = $state(3);
  reduceAnimationUpdates = $state(false);
  liquidGlass = $state(false);
  /* 自定义背景（data URL，null = 默认纯色背景）：存独立 app_assets 表 */
  customBackground = $state(null);
  logs = $state([]);
}

/* ============ ServerStatusState ============ */
export class ServerStatusState {
  serverStatuses = $state({});
  activeServerIds = $state([]);
  _timer = null;

  setActiveServers(ids) {
    this.activeServerIds = ids;
  }

  async _checkOnce(servers, pingFn) {
    const map = {};
    for (const server of servers) {
      if (this.activeServerIds.includes(server.id)) {
        map[server.id] = 'inUse';
      } else {
        const ms = await pingFn(server.url);
        map[server.id] = ms != null ? 'online' : 'offline';
      }
    }
    this.serverStatuses = map;
  }

  startPeriodicCheck(servers, intervalMs, pingFn) {
    this.stopPeriodicCheck();
    this._checkOnce(servers, pingFn);
    this._timer = setInterval(() => {
      this._checkOnce(servers, pingFn);
    }, intervalMs);
  }

  stopPeriodicCheck() {
    if (this._timer) {
      clearInterval(this._timer);
      this._timer = null;
    }
  }
}

/* ============ 常量 ============ */
const ANDROID_PACKAGE = 'com.kevin.astral';
const GITHUB_OWNER = 'ldoubil';
const GITHUB_REPO = 'astral';
const RELEASES_URL = `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases?per_page=20`;
const RELEASE_PAGE = `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases`;
const NET_DISK_URL = 'https://astral.fan/quick-start/download-install/';
const MIRRORS = ['https://gh.xmly.dev/', 'https://gh-proxy.org/', 'https://ghfast.top/'];
const SPEED_TEST_URL = 'https://github.com/ldoubil/astral';

/* ============ 应用版本 ============ */
const APP_VERSION = '2.9.60';

class ServiceManager {
  constructor() {
    /* 状态 */
    this.themeState = new ThemeState();
    this.uiState = new UIState();
    this.roomState = new RoomState();
    this.serverState = new ServerState();
    this.networkConfigState = new NetworkConfigState();
    this.playerState = new PlayerState();
    this.displayState = new DisplayState();
    this.startupState = new StartupState();
    this.updateState = new UpdateState();
    this.connectionState = new ConnectionState();
    this.windowState = new WindowState();
    this.firewallState = new FirewallState();
    this.vpnState = new VpnState();
    this.appSettingsState = new AppSettingsState();
    this.latencyHistory = $state({}); /* 曲线图数据源：peerKey -> [ms...] */
    this.serverStatusState = new ServerStatusState();

    /* 连接管理 */
    this._retryCount = 0;
    this._cancelCompleter = null; // { done: bool, complete(v) }
    this._timeoutTimer = null;
    this._statusTimer = null;
    this._monitorTimer = null;
    this._connectionDuration = 0;
    this._configBuilder = null;
    this._androidTunAttaching = false;

    this.appVersion = APP_VERSION;
    this.initialized = false;
  }

  /* ==================== 初始化 ==================== */
  async init() {
    try {
      await db.initDb();
    } catch (e) {
      /* 初始化失败不阻断 UI：真实错误显示在日志页 */
      console.error('数据库初始化失败:', e);
      this.pushLog(String(e?.message || e));
    }

    const tasks = [
      this._initTheme(),
      this._initRooms(),
      this._initServers(),
      this._initNetworkConfig(),
      this._initAppSettings(),
      this._initFirewall(),
    ];
    await Promise.allSettled(tasks);

    /* 日志通道 */
    this._initLogCapture();

    this.initialized = true;

    /* 启动自动连接 */
    this._tryStartupAutoConnect();
  }

  /* ---------- 主题 ---------- */
  async _initTheme() {
    const rows = await db.listAll('theme_settings');
    const row = rows.find((r) => r.id === 1);
    if (!row) {
      // 原版首次建库实际写入 0xFFFF5722（ThemeSettings 构造默认）
      await db.putRow('theme_settings', {
        id: 1,
        colorValue: 0xffff5722,
        themeModeValue: 'system',
      });
      this.themeState.themeColor = '#FF5722';
      this.themeState.themeMode = 'system';
    } else {
      const c = row.colorValue ?? 0xffff5722;
      const hex =
        '#' +
        Number(c)
          .toString(16)
          .padStart(8, '0')
          .slice(2);
      this.themeState.themeColor = hex.toUpperCase() === '#2196F3' ? '#2196F3' : hex;
      this.themeState.themeMode = row.themeModeValue || 'system';
    }
  }

  async updateThemeColor(colorHex) {
    this.themeState.themeColor = colorHex;
    try {
      const c = parseInt(colorHex.replace('#', ''), 16);
      await db.putRow('theme_settings', {
        id: 1,
        colorValue: c,
        themeModeValue: this.themeState.themeMode,
      });
    } catch (e) {
      /* 浏览器预览无 SQLite：状态已生效，仅持久化失败（与自定义背景一致） */
      console.warn('主题色持久化失败:', e);
    }
  }

  async updateThemeMode(mode) {
    this.themeState.themeMode = mode;
    try {
      const rows = await db.listAll('theme_settings');
      const row = rows.find((r) => r.id === 1) || { id: 1 };
      await db.putRow('theme_settings', {
        ...row,
        id: 1,
        colorValue: parseInt(this.themeState.themeColor.replace('#', ''), 16),
        themeModeValue: mode,
      });
    } catch (e) {
      /* 同上：浏览器预览下仅持久化失败 */
      console.warn('主题模式持久化失败:', e);
    }
  }

  /* ---------- 房间 ---------- */
  async _initRooms() {
    let rooms = await db.listAll('rooms');
    rooms.sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
    if (rooms.length === 0) {
      const defaultRoom = {
        id: null,
        name: '默认房间',
        encrypted: true,
        roomName: String(Math.floor(100000 + Math.random() * 900000)),
        password: String(Math.floor(100000 + Math.random() * 900000)),
        messageKey: '',
        tags: [],
        servers: [],
        customParam: '',
        networkConfigJson: '',
        sortOrder: 0,
      };
      const saved = await db.putRow('rooms', defaultRoom);
      rooms = [saved];
    }
    this.roomState.rooms = rooms;
    const settings = await this._getSettings();
    if (settings.room != null) {
      this.roomState.selectedRoom = rooms.find((r) => r.id === settings.room) || null;
    }
  }

  async addRoom(room) {
    const saved = await db.putRow('rooms', { ...room, id: null });
    await this._refreshRooms();
    return saved;
  }

  async deleteRoom(id) {
    await db.deleteRow('rooms', id);
    if (this.roomState.selectedRoom?.id === id) {
      this.roomState.selectedRoom = null;
      const settings = await this._getSettings();
      await this._saveSettings({ ...settings, room: null });
    }
    await this._refreshRooms();
  }

  async updateRoom(room) {
    await db.putRow('rooms', room);
    await this._refreshRooms();
  }

  async reorderRooms(orderedRooms) {
    for (let i = 0; i < orderedRooms.length; i++) {
      await db.putRow('rooms', { ...orderedRooms[i], sortOrder: i });
    }
    await this._refreshRooms();
  }

  async setRoom(room) {
    this.roomState.selectedRoom = room;
    const settings = await this._getSettings();
    await this._saveSettings({ ...settings, room: room ? room.id : null });
  }

  async _refreshRooms() {
    let rooms = await db.listAll('rooms');
    rooms.sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
    this.roomState.rooms = rooms;
    if (this.roomState.selectedRoom) {
      this.roomState.selectedRoom =
        rooms.find((r) => r.id === this.roomState.selectedRoom.id) || null;
    }
  }

  /* ---------- 服务器 ---------- */
  async _initServers() {
    let servers = await db.listAll('servers');
    // 清理废弃域名（ServerDao.init 逻辑）
    let changed = false;
    for (const s of servers) {
      if (isDeprecatedServer(s.url)) {
        await db.deleteRow('servers', s.id);
        changed = true;
      }
    }
    if (changed) servers = await db.listAll('servers');
    servers.sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
    if (servers.length === 0) {
      const seeded = await db.putRow('servers', {
        id: null,
        name: '[小探][可中转A]',
        url: 'js.629957.xyz:11012',
        enable: false,
        tcp: true,
        faketcp: false,
        udp: false,
        ws: false,
        wss: false,
        quic: false,
        wg: false,
        txt: false,
        srv: false,
        http: false,
        https: false,
        sortOrder: 0,
      });
      servers = [seeded];
    }
    this.serverState.servers = servers;
  }

  async addServer(server) {
    const saved = await db.putRow('servers', { ...server, id: null });
    await this._refreshServers();
    return saved;
  }

  async updateServer(server) {
    await db.putRow('servers', server);
    await this._refreshServers();
  }

  async deleteServer(server) {
    await db.deleteRow('servers', server.id);
    await this._refreshServers();
  }

  async reorderServers(orderedServers) {
    for (let i = 0; i < orderedServers.length; i++) {
      await db.putRow('servers', { ...orderedServers[i], sortOrder: i });
    }
    await this._refreshServers();
  }

  async setServerEnable(server, enable) {
    await db.putRow('servers', { ...server, enable });
    await this._refreshServers();
  }

  getAllServers() {
    return this.serverState.servers;
  }

  async _refreshServers() {
    let servers = await db.listAll('servers');
    servers.sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
    this.serverState.servers = servers;
  }

  /* ---------- 网络配置 ---------- */
  async _initNetworkConfig() {
    let rows = await db.listAll('net_config');
    let row = rows.find((r) => r.id === 1);
    if (!row) {
      row = { id: 1 };
      await db.putRow('net_config', row);
    }
    const settings = await this._getSettings();
    this._applyNetConfig(row, { autoSetMtu: settings.autoSetMTU !== false });
  }

  _applyNetConfig(row, { autoSetMtu }) {
    const s = this.networkConfigState;
    s.instanceName = row.instanceName ?? 'default';
    s.ipv4 = row.ipv4 ?? '';
    s.dhcp = row.dhcp !== false;
    s.defaultProtocol = row.default_protocol || 'tcp';
    s.enableEncryption = row.enable_encryption !== false;
    s.enableIpv6 = row.enable_ipv6 !== false;
    s.mtu = row.mtu ?? 1360;
    s.latencyFirst = row.latency_first === true;
    s.enableExitNode = row.enable_exit_node === true;
    s.noTun = row.no_tun === true;
    s.enableSocks5 = row.enable_socks5 === true;
    s.socks5Port = this._normalizeSocks5Port(row.socks5_port);
    s.useSmoltcp = row.use_smoltcp === true;
    s.relayNetworkWhitelist = row.relay_network_whitelist || '*';
    s.disableP2p = row.disable_p2p === true;
    s.enableUdpBroadcastRelay = row.enable_udp_broadcast_relay === true;
    s.relayAllPeerRpc = row.relay_all_peer_rpc === true;
    s.disableUdpHolePunching = row.disable_udp_hole_punching === true;
    s.disableTcpHolePunching = row.disable_tcp_hole_punching === true;
    s.disableSymHolePunching = row.disable_sym_hole_punching === true;
    s.multiThread = row.multi_thread !== false;
    s.dataCompressAlgo = row.data_compress_algo ?? 1;
    s.bindDevice = row.bind_device !== false;
    s.enableKcpProxy = row.enable_kcp_proxy !== false;
    s.disableKcpInput = row.disable_kcp_input === true;
    s.disableRelayKcp = row.disable_relay_kcp !== false;
    s.proxyForwardBySystem = row.proxy_forward_by_system === true;
    s.acceptDns = row.accept_dns === true;
    s.privateMode = row.private_mode === true;
    s.enableQuicProxy = row.enable_quic_proxy === true;
    s.disableQuicInput = row.disable_quic_input === true;
    s.tcpWhitelist = row.tcp_whitelist || '';
    s.udpWhitelist = row.udp_whitelist || '';
  }

  _normalizeSocks5Port(p) {
    const n = Number(p);
    if (!Number.isFinite(n) || n <= 0 || n > 65535) return 1080;
    return n;
  }

  async _persistNetConfig() {
    const s = this.networkConfigState;
    await db.putRow('net_config', {
      id: 1,
      netns: '',
      hostname: localHostname(),
      instance_name: s.instanceName,
      ipv4: s.ipv4,
      dhcp: s.dhcp,
      network_name: '',
      network_secret: '',
      listeners: [],
      peer: [],
      cidrproxy: [],
      connectionManagers: [],
      default_protocol: s.defaultProtocol,
      dev_name: '',
      enable_encryption: s.enableEncryption,
      enable_ipv6: s.enableIpv6,
      mtu: s.mtu,
      latency_first: s.latencyFirst,
      enable_exit_node: s.enableExitNode,
      no_tun: s.noTun,
      enable_socks5: s.enableSocks5,
      socks5_port: s.socks5Port,
      use_smoltcp: s.useSmoltcp,
      relay_network_whitelist: s.relayNetworkWhitelist,
      disable_p2p: s.disableP2p,
      enable_udp_broadcast_relay: s.enableUdpBroadcastRelay,
      relay_all_peer_rpc: s.relayAllPeerRpc,
      disable_udp_hole_punching: s.disableUdpHolePunching,
      disable_tcp_hole_punching: s.disableTcpHolePunching,
      disable_sym_hole_punching: s.disableSymHolePunching,
      multi_thread: s.multiThread,
      data_compress_algo: s.dataCompressAlgo,
      bind_device: s.bindDevice,
      enable_kcp_proxy: s.enableKcpProxy,
      disable_kcp_input: s.disableKcpInput,
      disable_relay_kcp: s.disableRelayKcp,
      proxy_forward_by_system: s.proxyForwardBySystem,
      accept_dns: s.acceptDns,
      private_mode: s.privateMode,
      enable_quic_proxy: s.enableQuicProxy,
      disable_quic_input: s.disableQuicInput,
      tcp_whitelist: s.tcpWhitelist,
      udp_whitelist: s.udpWhitelist,
    });
  }

  /* 各 update 方法：更新 signal + 持久化 */
  async updateIpv4(value) {
    this.networkConfigState.ipv4 = value;
    await this._persistNetConfig();
  }
  async updateDhcp(value) {
    this.networkConfigState.dhcp = value;
    await this._persistNetConfig();
  }
  async updateEnableEncryption(value) {
    this.networkConfigState.enableEncryption = value;
    this.networkConfigState.mtu = value ? 1360 : 1380; // 原版 MTU 联动
    await this._persistNetConfig();
  }
  async updateMtu(value) {
    this.networkConfigState.mtu = value;
    await this._persistNetConfig();
  }
  async updateLatencyFirst(value) {
    this.networkConfigState.latencyFirst = value;
    await this._persistNetConfig();
  }
  async updateDefaultProtocol(value) {
    this.networkConfigState.defaultProtocol = value;
    await this._persistNetConfig();
  }
  async updateNoTun(value) {
    this.networkConfigState.noTun = value;
    await this._persistNetConfig();
  }
  async updateEnableSocks5(value) {
    this.networkConfigState.enableSocks5 = value;
    await this._persistNetConfig();
  }
  async updateSocks5Port(value) {
    this.networkConfigState.socks5Port = this._normalizeSocks5Port(value);
    await this._persistNetConfig();
  }
  async updateDisableP2p(value) {
    this.networkConfigState.disableP2p = value;
    await this._persistNetConfig();
  }
  async updateEnableUdpBroadcastRelay(value) {
    this.networkConfigState.enableUdpBroadcastRelay = value;
    await this._persistNetConfig();
  }
  async updateDisableUdpHolePunching(value) {
    this.networkConfigState.disableUdpHolePunching = value;
    await this._persistNetConfig();
  }
  async updateDisableTcpHolePunching(value) {
    this.networkConfigState.disableTcpHolePunching = value;
    await this._persistNetConfig();
  }
  async updateDisableSymHolePunching(value) {
    this.networkConfigState.disableSymHolePunching = value;
    await this._persistNetConfig();
  }
  async updateDataCompressAlgo(value) {
    this.networkConfigState.dataCompressAlgo = value;
    await this._persistNetConfig();
  }
  async updateBindDevice(value) {
    this.networkConfigState.bindDevice = value;
    await this._persistNetConfig();
  }
  async updateEnableKcpProxy(value) {
    this.networkConfigState.enableKcpProxy = value;
    await this._persistNetConfig();
  }
  async updateTcpWhitelist(value) {
    this.networkConfigState.tcpWhitelist = value;
    await this._persistNetConfig();
  }
  async updateUdpWhitelist(value) {
    this.networkConfigState.udpWhitelist = value;
    await this._persistNetConfig();
  }
  async getAutoSetMTU() {
    const settings = await this._getSettings();
    return settings.autoSetMTU !== false;
  }
  async setAutoSetMTU(value) {
    const settings = await this._getSettings();
    await this._saveSettings({ ...settings, autoSetMTU: value });
  }

  /* ---------- 应用设置（AllSettings 单例 id=1） ---------- */
  async _getSettings() {
    const rows = await db.listAll('all_settings');
    let row = rows.find((r) => r.id === 1);
    if (!row) {
      row = { id: 1 };
      await db.putRow('all_settings', row);
    }
    return row;
  }

  async _saveSettings(row) {
    await db.putRow('all_settings', { ...row, id: 1 });
  }

  async _initAppSettings() {
    let s = await this._getSettings();
    let migrated = false;
    /* downloadAccelerate 迁移 */
    if (s.downloadAccelerate === 'https://gh.xmly.dev/') {
      s.downloadAccelerate = 'auto';
      migrated = true;
    }
    /* userListStyle 迁移（-1 时由 userListSimple 推导） */
    if (s.userListStyle == null || s.userListStyle === -1) {
      s.userListStyle = s.userListSimple === false ? 1 : 0;
      migrated = true;
    }
    if (migrated) await this._saveSettings(s);

    this.playerState.playerName =
      s.playerName != null && s.playerName !== '' ? s.playerName : localHostname();
    this.playerState.listenList = s.listenList || [
      'tcp://0.0.0.0:0',
      'udp://0.0.0.0:0',
    ];
    this.vpnState.customVpn = s.customVpn || [];
    this.displayState.userListStyle = s.userListStyle ?? 0;
    this.displayState.sortOption = s.sortOption ?? 0;
    this.displayState.sortOrder = s.sortOrder ?? 0;
    this.displayState.displayMode = s.displayMode ?? 0;
    this.startupState.startup = s.startup === true;
    this.startupState.startupMinimize = s.startupMinimize === true;
    this.startupState.startupAutoConnect = s.startupAutoConnect === true;
    this.updateState.beta = s.beta === true;
    this.updateState.autoCheckUpdate = s.autoCheckUpdate !== false;
    this.updateState.downloadAccelerate = s.downloadAccelerate || 'auto';
    this.updateState.latestVersion = s.latestVersion || null;
    this.windowState.closeMinimize = s.closeMinimize !== false;
    this.appSettingsState.enableConnectionNotification =
      s.enableConnectionNotification !== false;
    this.appSettingsState.autoRetryOnFailure = s.autoRetryOnFailure !== false;
    this.appSettingsState.maxRetryCount = s.maxRetryCount ?? 3;
    this.appSettingsState.reduceAnimationUpdates =
      s.reduceAnimationUpdates === true;
    this.appSettingsState.liquidGlass = s.liquidGlass === true;

    /* 自定义背景（v2.6）：Tauri = 磁盘文件（astralbg:// 自定义协议服务，
     * SoftwareSettingsPage 分块写入）；兼容 v2.5 及更早存于 app_assets 的
     * data URL —— 后台迁移为磁盘文件。浏览器预览为会话级。 */
    try {
      if (isTauri()) {
        const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
        const info = await invoke('custom_background_info');
        if (info && info.version != null) {
          this.appSettingsState.customBackground =
            convertFileSrc('bg', 'astralbg') + '?v=' + info.version;
        } else {
          const assets = await db.listAll('app_assets');
          const asset = assets.find((r) => r.id === 1);
          if (typeof asset?.image === 'string' && asset.image.startsWith('data:image/')) {
            void this._migrateLegacyBackground(asset.image); // 异步迁移，不阻塞启动
          }
        }
      }
    } catch (e) {
      console.warn('自定义背景加载失败:', e);
    }

    /* 同步关闭行为到 Tauri 窗口层 */
    this._syncCloseMinimize(this.windowState.closeMinimize);
  }

  async _persistSettings(patch) {
    const s = await this._getSettings();
    await this._saveSettings({ ...s, ...patch });
  }

  async updatePlayerName(name) {
    this.playerState.playerName = name;
    await this._persistSettings({ playerName: name });
  }
  async updateListenList(list) {
    this.playerState.listenList = list;
    await this._persistSettings({ listenList: list });
  }
  async addListen(value) {
    await this.updateListenList([...this.playerState.listenList, value]);
  }
  async updateListen(index, value) {
    const list = [...this.playerState.listenList];
    list[index] = value;
    await this.updateListenList(list);
  }
  async deleteListen(index) {
    const list = [...this.playerState.listenList];
    list.splice(index, 1);
    await this.updateListenList(list);
  }
  async updateCustomVpn(list) {
    this.vpnState.customVpn = list;
    await this._persistSettings({ customVpn: list });
  }
  async setUserListStyle(v) {
    this.displayState.userListStyle = v;
    await this._persistSettings({ userListStyle: v });
  }
  async setDisplayMode(v) {
    this.displayState.displayMode = v;
    await this._persistSettings({ displayMode: v });
  }
  async setSortOption(v) {
    this.displayState.sortOption = v;
    await this._persistSettings({ sortOption: v });
  }
  async setSortOrder(v) {
    this.displayState.sortOrder = v;
    await this._persistSettings({ sortOrder: v });
  }
  async updateStartup(v) {
    this.startupState.startup = v;
    await this._persistSettings({ startup: v });
    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('set_startup', { enable: v });
      } catch (e) {
        console.warn('设置开机自启失败:', e);
      }
    }
  }
  async updateStartupMinimize(v) {
    this.startupState.startupMinimize = v;
    await this._persistSettings({ startupMinimize: v });
  }
  async updateStartupAutoConnect(v) {
    this.startupState.startupAutoConnect = v;
    await this._persistSettings({ startupAutoConnect: v });
  }
  async updateBeta(v) {
    this.updateState.beta = v;
    await this._persistSettings({ beta: v });
  }
  async updateAutoCheckUpdate(v) {
    this.updateState.autoCheckUpdate = v;
    await this._persistSettings({ autoCheckUpdate: v });
  }
  async updateDownloadAccelerate(v) {
    this.updateState.downloadAccelerate = v;
    await this._persistSettings({ downloadAccelerate: v });
  }
  async updateLatestVersion(v) {
    this.updateState.latestVersion = v;
    await this._persistSettings({ latestVersion: v });
  }
  async updateCloseMinimize(v) {
    this.windowState.closeMinimize = v;
    await this._persistSettings({ closeMinimize: v });
    this._syncCloseMinimize(v);
  }

  /** 同步关闭行为到 Tauri 窗口层（Tauri 模式生效） */
  _syncCloseMinimize(v) {
    if (!isTauri()) return;
    import('@tauri-apps/api/core')
      .then(({ invoke }) => invoke('set_close_minimize', { value: v }))
      .catch(() => {});
  }
  async updateEnableConnectionNotification(v) {
    this.appSettingsState.enableConnectionNotification = v;
    await this._persistSettings({ enableConnectionNotification: v });
  }
  async updateAutoRetryOnFailure(v) {
    this.appSettingsState.autoRetryOnFailure = v;
    await this._persistSettings({ autoRetryOnFailure: v });
  }
  async updateMaxRetryCount(v) {
    this.appSettingsState.maxRetryCount = v;
    await this._persistSettings({ maxRetryCount: v });
  }
  async updateReduceAnimationUpdates(v) {
    this.appSettingsState.reduceAnimationUpdates = v;
    await this._persistSettings({ reduceAnimationUpdates: v });
  }
  async updateLiquidGlass(v) {
    this.appSettingsState.liquidGlass = v;
    await this._persistSettings({ liquidGlass: v });
  }

  /* ---------- 自定义背景（v2.6） ----------
   * Tauri 端由磁盘文件 + astralbg:// 协议承载（写入/清除由
   * SoftwareSettingsPage 调用 bg_write_* / clear_custom_background 命令完成）；
   * 浏览器预览为 Blob URL（会话级）。此处仅维护会话状态。 */
  async updateCustomBackground(url) {
    this.appSettingsState.customBackground = url || null;
  }

  /* v2.5 data URL 背景 → 磁盘文件迁移（后台执行，失败仅警告） */
  async _migrateLegacyBackground(dataUrl) {
    try {
      const comma = dataUrl.indexOf(',');
      const baseType = dataUrl.slice(5, comma).split(';')[0];
      const ext = {
        'image/jpeg': 'jpg', 'image/png': 'png', 'image/apng': 'apng',
        'image/gif': 'gif', 'image/webp': 'webp', 'image/bmp': 'bmp',
        'image/svg+xml': 'svg', 'image/x-icon': 'ico', 'image/tiff': 'tiff',
        'image/avif': 'avif', 'image/heic': 'heic', 'image/heif': 'heif',
        'image/heic-sequence': 'heic', 'image/jxl': 'jxl',
      }[baseType];
      if (!ext) return;
      const blob = await (await fetch(dataUrl)).blob();
      const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
      await invoke('bg_write_begin', { ext });
      const CHUNK = 1 << 20;
      for (let off = 0; off < blob.size; off += CHUNK) {
        const b64 = await new Promise((resolve, reject) => {
          const r = new FileReader();
          r.onload = () => resolve(String(r.result).split(',')[1] || '');
          r.onerror = () => reject(r.error);
          r.readAsDataURL(blob.slice(off, Math.min(off + CHUNK, blob.size)));
        });
        await invoke('bg_write_append', { b64 });
      }
      const info = await invoke('bg_write_end');
      await db.deleteRow('app_assets', 1); // 迁移完成，清理旧行
      this.appSettingsState.customBackground =
        convertFileSrc('bg', 'astralbg') + '?v=' + info.version;
      console.info('自定义背景已由 data URL 迁移为磁盘文件');
    } catch (e) {
      console.warn('自定义背景迁移失败:', e);
    }
  }

  /* ---------- 防火墙 ---------- */
  async _initFirewall() {
    try {
      await this.updateFirewallStatus();
    } catch {
      this.firewallState.firewallStatus = false;
    }
  }

  async updateFirewallStatus() {
    const results = await Promise.all([
      rust.getFirewallStatus(1),
      rust.getFirewallStatus(2),
      rust.getFirewallStatus(3),
    ]);
    this.firewallState.firewallStatus = results.every(Boolean);
  }

  async setFirewall(enable) {
    this.firewallState.firewallStatus = enable;
    try {
      await rust.setFirewallStatus(1, enable);
      await rust.setFirewallStatus(2, enable);
      await rust.setFirewallStatus(3, enable);
    } catch (e) {
      await this.updateFirewallStatus();
      throw e;
    }
  }

  /* ---------- 日志捕获 ---------- */
  _initLogCapture() {
    rust.listenRustLogs((text) => {
      this.pushLog(text);
    });
  }

  pushLog(text) {
    const now = new Date();
    const pad = (n) => String(n).padStart(2, '0');
    const line = `[${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}] ${text}`;
    const logs = [...this.appSettingsState.logs, line];
    if (logs.length > 1000) logs.splice(0, logs.length - 1000);
    this.appSettingsState.logs = logs;
    /* 诊断镜像：日志同时落盘（astral-cmd.log），不依赖日志页可见性 */
    rust.frontendLog(line).catch(() => {});
  }

  clearLogs() {
    this.appSettingsState.logs = [];
  }

  /* ==================== 连接管理（ServerConnectionManager） ==================== */

  /** 连接守卫：返回 'noRoom'|'noneEnabled'|'noServers'|null */
  connectTargetIssue() {
    const room = this.roomState.selectedRoom;
    if (room == null) return 'noRoom';
    const enabledServers = this.serverState.servers.filter((s) => s.enable);
    const roomHasServers = room.servers && room.servers.length > 0;
    if (this.serverState.servers.length === 0 && !roomHasServers) return 'noServers';
    if (enabledServers.length === 0 && !roomHasServers) return 'noneEnabled';
    return null;
  }

  hasConnectTarget() {
    return this.connectTargetIssue() == null;
  }

  async isNpcapReady() {
    return rust.hasNpcapDriver();
  }

  async _tryStartupAutoConnect() {
    if (this.startupState.startupAutoConnect && this.hasConnectTarget()) {
      try {
        if (await this.isNpcapReady()) {
          await this.connect({ isManual: false });
        }
      } catch (e) {
        console.warn('自动连接失败:', e);
      }
    }
  }

  /** 服务器 URL 展开（协议开关 → proto://url，顺序固定） */
  _expandServerUrls(server) {
    const urls = [];
    const protocols = [
      ['tcp', server.tcp],
      ['faketcp', server.faketcp],
      ['udp', server.udp],
      ['ws', server.ws],
      ['wss', server.wss],
      ['quic', server.quic],
      ['wg', server.wg],
      ['txt', server.txt],
      ['srv', server.srv],
      ['http', server.http],
      ['https', server.https],
    ];
    for (const [proto, on] of protocols) {
      if (on) urls.push(`${proto}://${server.url}`);
    }
    return urls;
  }

  /** ServerConfigBuilder.build() —— 组装 createServer 参数 */
  _buildServerConfig(room) {
    const player = this.playerState;
    const net = this.networkConfigState;

    /* 房间携带的网络配置覆盖 */
    let roomConfig = null;
    if (room.networkConfigJson && room.networkConfigJson.length > 0) {
      roomConfig = parseNetworkConfigShare(room.networkConfigJson);
    }

    /* 用户名与 IP（非法 IP 强制 DHCP） */
    let enableDhcp = roomConfig ? roomConfig.dhcp : net.dhcp;
    let specifiedIp = '';
    if (!enableDhcp) {
      const ip = net.ipv4 || '';
      if (!ip || ip === '0.0.0.0' || !isValidIpAddress(ip, { excludeLoopback: false })) {
        enableDhcp = true;
        specifiedIp = '';
      } else {
        specifiedIp = ip;
      }
    }

    /* 服务器 URL 合并：房间服务器在前，全局启用去重 */
    const enabledServers = this.serverState.servers.filter((s) => s.enable);
    const globalUrls = enabledServers.flatMap((s) => this._expandServerUrls(s));
    const roomUrls = room.servers || [];
    const severurl = roomUrls.length > 0
      ? [...roomUrls, ...globalUrls.filter((u) => !roomUrls.includes(u))]
      : globalUrls;

    /* 监听列表（过滤 [::]） */
    const onurl = (player.listenList || []).filter((l) => !l.includes('[::]'));

    /* 网段 */
    const cidrs = (this.vpnState.customVpn || []).filter((c) => isValidCIDR(c));

    const pick = (roomVal, stateVal) => (roomVal != null ? roomVal : stateVal);

    const flag = {
      defaultProtocol: roomConfig?.defaultProtocol || net.defaultProtocol,
      devName: 'astral',
      enableEncryption: pick(roomConfig?.enableEncryption, net.enableEncryption),
      enableIpv6: net.enableIpv6,
      mtu: net.mtu,
      latencyFirst: pick(roomConfig?.latencyFirst, net.latencyFirst),
      enableExitNode: net.enableExitNode,
      noTun: pick(roomConfig?.noTun, net.noTun),
      useSmoltcp: net.useSmoltcp,
      relayNetworkWhitelist: '*',
      disableP2p: pick(roomConfig?.disableP2p, net.disableP2p),
      enableUdpBroadcastRelay: net.enableUdpBroadcastRelay,
      relayAllPeerRpc: net.relayAllPeerRpc,
      disableUdpHolePunching: pick(
        roomConfig?.disableUdpHolePunching,
        net.disableUdpHolePunching
      ),
      disableTcpHolePunching: pick(
        roomConfig?.disableTcpHolePunching,
        net.disableTcpHolePunching
      ),
      disableSymHolePunching: pick(
        roomConfig?.disableSymHolePunching,
        net.disableSymHolePunching
      ),
      multiThread: net.multiThread,
      dataCompressAlgo: pick(roomConfig?.dataCompressAlgo, net.dataCompressAlgo),
      bindDevice: pick(roomConfig?.bindDevice, net.bindDevice),
      enableKcpProxy: pick(roomConfig?.enableKcpProxy, net.enableKcpProxy),
      disableKcpInput: net.disableKcpInput,
      disableRelayKcp: net.disableRelayKcp,
      proxyForwardBySystem: net.proxyForwardBySystem,
      acceptDns: net.acceptDns,
      privateMode: net.privateMode,
      enableQuicProxy: net.enableQuicProxy,
      disableQuicInput: net.disableQuicInput,
      socks5Port: net.enableSocks5 ? net.socks5Port : 0,
      tcpWhitelist: net.tcpWhitelist,
      udpWhitelist: net.udpWhitelist,
    };

    return {
      username: player.playerName || 'player',
      enableDhcp,
      specifiedIp,
      roomName: room.roomName,
      roomPassword: room.password,
      severurl,
      onurl,
      cidrs,
      forwards: [],
      flag,
    };
  }

  /**
   * 发起连接。
   * @returns {Promise<boolean|null>} true 成功 / false 失败 / null 用户取消
   */
  async connect({ isManual = true } = {}) {
    try {
      return await this._connectInner({ isManual });
    } catch (e) {
      /* 连接流程任何未预期异常：写入应用日志页并保持 idle */
      console.error('connect() 异常:', e);
      this.pushLog('[connect] 异常: ' + String(e?.message || e));
      this.connectionState.connectionState = CoState.idle;
      return false;
    }
  }

  async _connectInner({ isManual = true } = {}) {
    if (this.connectionState.connectionState !== CoState.idle) return false;
    const room = this.roomState.selectedRoom;
    if (room == null) return false;
    this.pushLog(
      '[connect] 开始: room=' + (room.name || room.roomName || '?') +
      ' roomServers=' + JSON.stringify((room.servers || []).length) +
      ' allServers=' + JSON.stringify(
        this.serverState.servers.map((s) => ({ n: s.name, e: !!s.enable, u: s.url }))
      )
    );

    if (isManual) this._retryCount = 0;
    this._cancelCompleter = { done: false };

    const autoRetry = this.appSettingsState.autoRetryOnFailure;
    const maxRetries = this.appSettingsState.maxRetryCount || 3;

    let result = false;
    do {
      this._retryCount += 1;
      this.pushLog('[connect] 第 ' + this._retryCount + ' 次尝试');
      if (this._retryCount > 1) {
        await this._delay(2000);
      }
      if (this._cancelCompleter?.done) {
        this.connectionState.connectionState = CoState.idle;
        return null;
      }
      try {
        await rust.closeServer();
        /* 目标校验 */
        const enabledServers = this.serverState.servers.filter((s) => s.enable);
        const roomHasServers = room.servers && room.servers.length > 0;
        if (enabledServers.length === 0 && !roomHasServers) {
          this.connectionState.connectionState = CoState.idle;
          return false;
        }

        /* Android：先申请系统 VPN 授权，未授权则视为用户取消 */
        if (isAndroid()) {
          const prepare = await rust.vpnPrepare();
          if (!prepare?.granted) {
            this.connectionState.connectionState = CoState.idle;
            return null;
          }
        }

        const config = this._buildServerConfig(room);
        if (isAndroid()) {
          /* TUN 设备由 VpnService 提供，核心不再自行创建 */
          config.flag.noTun = true;
        }
        this.pushLog('[connect] createServer 提交');
        await rust.createServer(config);
        this.pushLog('[connect] createServer 已返回');

        this.connectionState.connectionState = CoState.connecting;
        this.pushLog('[connect] 进入 connecting（等待虚拟 IP）');
        this._setupConnectionTimeout();
        this._startConnectionStatusCheck();

        result = await this._waitForConnectionResult();

        if (result) {
          this._retryCount = 0;
          return true;
        }
      } catch (e) {
        console.warn('连接过程异常:', e);
        this.pushLog('[connect] 第 ' + this._retryCount + ' 次尝试异常: ' + String(e?.message || e));
        result = false;
      }
      if (!autoRetry || this._retryCount >= maxRetries) {
        this.connectionState.connectionState = CoState.idle;
        return false;
      }
      await this.disconnect({ silent: true });
    } while (this._retryCount <= maxRetries && !this._cancelCompleter?.done);

    if (this._cancelCompleter?.done) {
      this.connectionState.connectionState = CoState.idle;
      return null;
    }
    this.connectionState.connectionState = CoState.idle;
    return false;
  }

  _delay(ms) {
    return new Promise((r) => setTimeout(r, ms));
  }

  _setupConnectionTimeout() {
    this._cancelTimeout();
    this._timeoutTimer = setTimeout(() => {
      if (this.connectionState.connectionState === CoState.connecting) {
        this.disconnect();
      }
    }, 15000);
  }

  _cancelTimeout() {
    if (this._timeoutTimer) {
      clearTimeout(this._timeoutTimer);
      this._timeoutTimer = null;
    }
  }

  /** 1 秒周期状态检查（ConnectionStatusTracker） */
  _startConnectionStatusCheck() {
    this._stopStatusCheck();
    this._statusTimer = setInterval(async () => {
      if (this.connectionState.connectionState !== CoState.connecting) {
        this._stopStatusCheck();
        return;
      }
      try {
        const infoJson = await rust.getRunningInfo();
        let ipv4 = '0.0.0.0';
        if (infoJson && infoJson !== 'null') {
          try {
            const info = JSON.parse(infoJson);
            const addr =
              info?.my_node_info?.virtual_ipv4?.address?.addr;
            if (addr != null && addr !== 0) ipv4 = intToIp(Number(addr));
          } catch {
            /* 忽略 */
          }
        }
        if (ipv4 !== '0.0.0.0') {
          if (ipv4 !== this.networkConfigState.ipv4) {
            this.networkConfigState.ipv4 = ipv4;
          }
          this._stopStatusCheck();
          if (isAndroid()) {
            this._attachAndroidTun(ipv4);
          } else {
            this._handleSuccessfulConnection();
          }
        }
      } catch {
        /* 忽略 */
      }
    }, 1000);
  }

  _stopStatusCheck() {
    if (this._statusTimer) {
      clearInterval(this._statusTimer);
      this._statusTimer = null;
    }
  }

  /**
   * Android：拿到虚拟 IP 后建立 VpnService，并把 TUN fd 交给 EasyTier 核心。
   * set_tun_fd 依赖已存在的运行实例，因此必须在 createServer 成功之后再调用。
   */
  async _attachAndroidTun(ipv4) {
    if (this._androidTunAttaching) return;
    this._androidTunAttaching = true;
    try {
      const routes = (this.vpnState.customVpn || []).filter((c) => isValidCIDR(c));
      const res = await rust.vpnStart({
        ipv4Addr: `${ipv4}/24`,
        routes,
        dns: null,
        disallowedApplications: [ANDROID_PACKAGE],
        mtu: this.networkConfigState.mtu || 1380,
      });
      const fd = Number(res?.fd);
      if (!Number.isInteger(fd) || fd < 0) throw new Error('未取得 TUN fd');
      await rust.setTunFd(fd);
      this._handleSuccessfulConnection();
    } catch (e) {
      console.warn('Android TUN 挂载失败:', e);
      this.pushLog(`Android TUN 挂载失败: ${e?.message || e}`);
      await this.disconnect({ silent: true });
    } finally {
      this._androidTunAttaching = false;
    }
  }

  /** 最多 30 次每次 1 秒等待终态 */
  async _waitForConnectionResult() {
    for (let i = 0; i < 30; i++) {
      await this._delay(1000);
      const state = this.connectionState.connectionState;
      if (state === CoState.connected) return true;
      if (state === CoState.idle) return false;
      if (this._cancelCompleter?.done) return null;
    }
    return false;
  }

  _handleSuccessfulConnection() {
    this._cancelTimeout();
    this._connectionDuration = 0;
    this.connectionState.connectionState = CoState.connected;

    /* 标记活跃服务器（inUse） */
    const activeIds = this.serverState.servers
      .filter((s) => s.enable)
      .map((s) => s.id);
    this.serverStatusState.setActiveServers(activeIds);

    /* Windows：设置 TUN 网卡跃点最小 */
    if (isWindows()) {
      rust.setInterfaceMetric('astral', 0).catch(() => {});
    }

    this._startNetworkMonitor();
  }

  /** 连接后网络监视器（1 秒周期） */
  _startNetworkMonitor() {
    this._stopNetworkMonitor();
    this._monitorTimer = setInterval(async () => {
      this._connectionDuration += 1;
      try {
        const infoJson = await rust.getRunningInfo();
        if (infoJson && infoJson !== 'null') {
          try {
            const info = JSON.parse(infoJson);
            const addr = info?.my_node_info?.virtual_ipv4?.address?.addr;
            if (addr != null && addr !== 0) {
              const ip = intToIp(Number(addr));
              if (ip !== this.networkConfigState.ipv4) {
                this.networkConfigState.ipv4 = ip;
                this._persistNetConfig();
              }
            }
          } catch {
            /* 忽略 */
          }
        }
      } catch {
        /* 忽略 */
      }
      try {
        this.connectionState.netStatus = await rust.getNetworkStatus();
        /* 按节点采样延迟历史（曲线图数据源，保留 120 点） */
        const nodes = this.connectionState.netStatus?.nodes || [];
        for (const n of nodes) {
          if (n.latency_ms == null) continue;
          const key = String(n.peer_id ?? n.hostname ?? n.ipv4);
          const hist = this.latencyHistory[key] || [];
          hist.push(n.latency_ms);
          if (hist.length > 120) hist.shift();
          this.latencyHistory[key] = hist;
        }
      } catch {
        /* 忽略 */
      }
    }, 1000);
  }

  _stopNetworkMonitor() {
    if (this._monitorTimer) {
      clearInterval(this._monitorTimer);
      this._monitorTimer = null;
    }
  }

  get connectionDuration() {
    return this._connectionDuration;
  }

  /** 用户取消连接 */
  async cancelConnection() {
    if (this._cancelCompleter && !this._cancelCompleter.done) {
      this._cancelCompleter.done = true;
    }
    await this.disconnect();
  }

  /** 断开连接 */
  async disconnect({ silent = false } = {}) {
    if (this._cancelCompleter && !this._cancelCompleter.done) {
      this._cancelCompleter.done = true;
    }
    this._retryCount = 0;
    this._cancelTimeout();
    this._stopStatusCheck();
    this._stopNetworkMonitor();
    try {
      await rust.closeServer();
    } catch {
      /* 忽略 */
    }
    if (isAndroid()) {
      try {
        await rust.vpnStop();
      } catch {
        /* 忽略 */
      }
    }
    this.connectionState.connectionState = CoState.idle;
    this.connectionState.netStatus = null;
    this.serverStatusState.setActiveServers([]);
  }

  /* ==================== 服务器状态检测 ==================== */
  startServerStatusCheck() {
    this.serverStatusState.startPeriodicCheck(
      this.serverState.servers,
      30000,
      pingHost
    );
  }
  stopServerStatusCheck() {
    this.serverStatusState.stopPeriodicCheck();
  }

  /* ==================== 更新检查（UpdateChecker） ==================== */
  async checkForUpdates({ beta = this.updateState.beta } = {}) {
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 15000);
      const resp = await fetch(RELEASES_URL, {
        headers: {
          Accept: 'application/vnd.github.v3+json',
          'User-Agent': 'astral',
        },
        signal: controller.signal,
      });
      clearTimeout(timer);
      if (!resp.ok) {
        return { kind: 'failed', error: `无法获取最新版本信息 (HTTP ${resp.status})` };
      }
      const releases = await resp.json();
      let release = releases.find((r) => !r.draft);
      if (!beta) {
        release = releases.find((r) => !r.draft && !r.prerelease);
      }
      if (!release) {
        return { kind: 'failed', error: '无法获取最新版本信息' };
      }
      const latest = (release.tag_name || '').replace(/^v/, '');
      const current = APP_VERSION;
      const shouldUpdate = this._versionGreaterThan(latest, current);
      await this.updateLatestVersion(shouldUpdate ? latest : null);
      return {
        kind: shouldUpdate ? 'updateAvailable' : 'upToDate',
        version: latest,
        releaseNotes: release.body || '新版本已发布',
        releasePage: release.html_url || RELEASE_PAGE,
        releaseInfo: release,
      };
    } catch (e) {
      return { kind: 'failed', error: `检查更新时发生错误: ${e}` };
    }
  }

  _versionGreaterThan(a, b) {
    const pa = a.split('-');
    const pb = b.split('-');
    const va = pa[0].split('.').map(Number);
    const vb = pb[0].split('.').map(Number);
    for (let i = 0; i < 3; i++) {
      if ((va[i] || 0) !== (vb[i] || 0)) return (va[i] || 0) > (vb[i] || 0);
    }
    const ra = pa.slice(1);
    const rb = pb.slice(1);
    if (ra.length === 0 && rb.length === 0) return false;
    if (ra.length === 0) return true;
    if (rb.length === 0) return false;
    return false; // 简化：prerelease 段相等视为不更新
  }

  /* 下载加速（GitHubProxySelector） */
  async resolveAcceleratedUrl(rawUrl) {
    const mode = this.updateState.downloadAccelerate;
    if (!mode) return rawUrl; // 关闭：直连
    if (mode === 'auto') {
      /* 测速缓存 1 小时 */
      const cached = this._speedCache;
      if (cached && Date.now() - cached.at < 3600000) {
        return cached.prefix + rawUrl;
      }
      let best = null;
      let bestTime = Infinity;
      for (const mirror of MIRRORS) {
        const t = await this._probeSpeed(mirror);
        if (t != null && t < bestTime) {
          bestTime = t;
          best = mirror;
        }
      }
      if (best) {
        this._speedCache = { at: Date.now(), prefix: best };
        return best + rawUrl;
      }
      return rawUrl;
    }
    /* 手动前缀 */
    let prefix = mode;
    if (!prefix.endsWith('/')) prefix += '/';
    return prefix + rawUrl;
  }

  async _probeSpeed(prefix) {
    try {
      const start = performance.now();
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 6000);
      await fetch(prefix + SPEED_TEST_URL.replace('https://', ''), {
        method: 'HEAD',
        signal: controller.signal,
        mode: 'no-cors',
      }).catch(() => {});
      clearTimeout(timer);
      return performance.now() - start;
    } catch {
      return null;
    }
  }
}

/* 单例 */
export const sm = new ServiceManager();
</script>
