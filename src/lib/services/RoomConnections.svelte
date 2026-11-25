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
        if (!(await sm.isNpcapReady())) throw new Error('Npcap 未就绪，请先完成驱动安装');
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
      for (let attempt = 0; attempt < 20; attempt++) {
        if (this.sessions[roomId] !== session || session.status !== 'connecting') return false;
        try {
          const raw = await rust.getRunningInfoFor(session.instanceId);
          if (raw && raw !== 'null') {
            const info = JSON.parse(raw);
            const address = info?.my_node_info?.virtual_ipv4?.address?.addr;
            if (address != null && Number(address) !== 0) {
              session.ipv4 = intToIp(address);
              session.status = 'connected';
              session.netStatus = await rust.getNetworkStatusFor(session.instanceId);
              session.attempts = 0;
              await this._configureAndroidTun(session.ipv4);
              this.setActiveRoom(roomId);
              this._startMonitor(roomId);
              return true;
            }
          }
        } catch {
          // 实例刚创建时运行信息尚未就绪。
        }
        await delay(1000);
      }
      session.status = 'error';
      session.error = '等待虚拟 IP 超时';
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
            session.status = 'error';
            session.error = String(error?.message || error);
            this._stopMonitor(roomId);
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
