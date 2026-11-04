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
