<script module>
  /** 桌面端多房间连接：每个房间独立 EasyTier 实例、IP、状态和成员列表。 */
  import { sm } from './ServiceManager.svelte';
  import * as rust from '../rust/Api.svelte';
  import { isAndroid } from '../utils/Platform.svelte';

  function intToIp(value) {
    const n = Number(value) >>> 0;
    return [24, 16, 8, 0].map((shift) => (n >>> shift) & 255).join('.');
  }
  function delay(ms) { return new Promise((resolve) => setTimeout(resolve, ms)); }

  class RoomConnectionState {
    sessions = $state({});
    activeRoomId = $state(null);
    tabs = $derived(Object.values(this.sessions).filter((session) => ['connecting', 'connected', 'error'].includes(session.status)));
    _monitors = new Map();
    _androidFd = null;

    async connectRoom(room) {
      if (!room) return false;
      const existing = this.sessions[room.id];
      if (existing && ['connecting', 'connected'].includes(existing.status)) {
        this.setActiveRoom(room.id);
        return existing.status === 'connected';
      }
      const enabledServers = sm.serverState.servers.filter((server) => server.enable);
      if (!enabledServers.length && !(room.servers || []).length) {
        sm.pushLog(`[multi-room] ${room.name} 没有可用服务器`);
        return false;
      }
      const session = {
        room, status: 'connecting', instanceId: '', netStatus: null,
        ipv4: '', error: '', attempts: 0,
      };
      this.sessions[room.id] = session;
      this.setActiveRoom(room.id);
      try {
        const needsNpcap = (room.servers || []).some((url) => String(url).toLowerCase().startsWith('faketcp://'))
          || enabledServers.some((server) => server.faketcp === true || String(server.url).toLowerCase().startsWith('faketcp://'));
        if (needsNpcap && !(await sm.isNpcapReady())) throw new Error('Npcap 未就绪，请先完成驱动安装');
        const config = sm._buildServerConfig(room);
        if (isAndroid()) config.flag.noTun = true;
        session.instanceId = await rust.createServerWithId(config);
        return await this._waitUntilConnected(room.id);
      } catch (error) {
        session.status = 'error';
        session.error = String(error?.message || error);
        sm.pushLog(`[multi-room] ${room.name} 连接失败：${session.error}`);
        if (session.instanceId) await rust.closeServerFor(session.instanceId).catch(() => {});
        this.setActiveRoom(room.id);
        return false;
      }
    }

    async _waitUntilConnected(roomId) {
      const session = this.sessions[roomId];
      if (!session) return false;
      let lastError = '';
      for (let attempt = 0; attempt < 45; attempt++) {
        if (this.sessions[roomId] !== session || session.status !== 'connecting') return false;
        try {
          const raw = await rust.getRunningInfoFor(session.instanceId);
          if (raw && raw !== 'null') {
            const info = typeof raw === 'string' ? JSON.parse(raw) : raw;
            const address = info?.my_node_info?.virtual_ipv4?.address?.addr;
            if (address != null && Number(address) !== 0) {
              session.ipv4 = typeof address === 'string' && address.includes('.') ? address : intToIp(address);
              session.status = 'connected';
              // 虚拟 IP 已就绪即表示 ET 启动成功；成员状态可在监控中重试，
              // 初始桥接查询失败不应把已有连接改判为失败并关闭实例。
              session.netStatus = await rust.getNetworkStatusFor(session.instanceId).catch(() => null);
              session.attempts = 0;
              await this._configureAndroidTun(session.ipv4);
              this.setActiveRoom(roomId);
              this._startMonitor(roomId);
              return true;
            }
          }
        } catch (error) {
          lastError = String(error?.message || error);
        }
        await delay(1000);
      }
      session.status = 'error';
      session.error = lastError ? `获取虚拟 IP 失败：${lastError}` : '获取虚拟 IP 超时，请检查 DHCP、TUN 驱动和房间配置';
      this.setActiveRoom(roomId);
      return false;
    }

    async _configureAndroidTun(ipv4) {
      if (!isAndroid()) return;
      if (this._androidFd == null) {
        const prepared = await rust.vpnPrepare();
        if (!prepared?.granted) throw new Error('未获得 Android VPN 权限');
        const result = await rust.vpnStart({
          ipv4Addr: `${ipv4}/24`,
          routes: sm.vpnState.customVpn || [],
          dns: null,
          disallowedApplications: [],
          mtu: sm.networkConfigState.mtu || 1380,
        });
        this._androidFd = Number(result?.fd);
        if (!Number.isInteger(this._androidFd) || this._androidFd < 0) {
          this._androidFd = null;
          throw new Error('未取得 Android TUN fd');
        }
      }
      // 同一 fd 默认重新挂载全部运行实例，不会撤销其他房间。
      await rust.setTunFd(this._androidFd);
    }

    _startMonitor(roomId) {
      this._stopMonitor(roomId);
      const timer = setInterval(async () => {
        const session = this.sessions[roomId];
        if (!session || session.status !== 'connected') {
          this._stopMonitor(roomId);
          return;
        }
        try {
          session.netStatus = await rust.getNetworkStatusFor(session.instanceId);
          session.attempts = 0;
          if (this.activeRoomId === roomId) this._mirror(session);
        } catch (error) {
          session.attempts += 1;
          if (session.attempts >= 6) {
            // 状态桥接的瞬时失败不等于 ET 已断线；保留连接并继续重试。
            session.error = String(error?.message || error);
            session.attempts = 0;
            sm.pushLog(`[multi-room] ${session.room.name} 成员状态刷新重试中：${session.error}`);
            if (this.activeRoomId === roomId) this._mirror(session);
          }
        }
      }, 1000);
      this._monitors.set(roomId, timer);
    }

    _stopMonitor(roomId) {
      const timer = this._monitors.get(roomId);
      if (timer) clearInterval(timer);
      this._monitors.delete(roomId);
    }

    setActiveRoom(roomId) {
      const session = this.sessions[roomId];
      if (!session) return;
      this.activeRoomId = roomId;
      sm.roomState.selectedRoom = session.room;
      this._mirror(session);
    }

    _mirror(session) {
      sm.connectionState.connectionState = session.status === 'connected' ? 'connected' : session.status === 'connecting' ? 'connecting' : 'idle';
      sm.connectionState.netStatus = session.netStatus;
      if (session.ipv4) sm.networkConfigState.ipv4 = session.ipv4;
    }

    async disconnectRoom(roomId = this.activeRoomId) {
      const session = this.sessions[roomId];
      if (!session) return;
      this._stopMonitor(roomId);
      const instanceId = session.instanceId;
      session.status = 'idle';
      session.netStatus = null;
      session.instanceId = '';
      delete this.sessions[roomId];
      if (instanceId) await rust.closeServerFor(instanceId).catch(() => {});
      const next = this.tabs[0];
      if (isAndroid()) {
        if (this.tabs.some((item) => item.status === 'connected') && this._androidFd != null) {
          await rust.setTunFd(this._androidFd).catch(() => {});
        } else if (this._androidFd != null) {
          await rust.vpnStop().catch(() => {});
          this._androidFd = null;
        }
      }
      if (next) this.setActiveRoom(next.room.id);
      else {
        this.activeRoomId = null;
        sm.connectionState.connectionState = 'idle';
        sm.connectionState.netStatus = null;
      }
    }
  }

  export const roomConnections = new RoomConnectionState();
</script>
