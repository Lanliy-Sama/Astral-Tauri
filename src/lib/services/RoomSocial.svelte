<script module>
  /** 房间协作总线：节点消息、账号代理搜索、音频流协商。
   * 消息只包含公开元数据；账号凭据始终由 Rust 在账号端本地读取。 */
  import { sm } from './ServiceManager.svelte';
  import { isTauri } from '../utils/Platform.svelte';
  import { isServerNode } from '../utils/Network.svelte';

  const PLATFORM_LABELS = { netease: '网易云', qq: 'QQ 音乐', kugou: '酷狗' };

  function uuid() {
    return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  }

  function intToIp(value) {
    const n = Number(value) >>> 0;
    return [24, 16, 8, 0].map((shift) => (n >>> shift) & 255).join('.');
  }

  async function roomIdentity(room) {
    const text = `${room?.roomName || ''}\u0000${room?.password || ''}`;
    try {
      const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
      return Array.from(new Uint8Array(digest)).map((b) => b.toString(16).padStart(2, '0')).join('').slice(0, 40);
    } catch {
      let hash = 2166136261;
      for (const ch of text) hash = Math.imul(hash ^ ch.charCodeAt(0), 16777619);
      return `room-${hash >>> 0}`;
    }
  }

  class RoomSocialState {
    connected = $state(false);
    starting = $state(false);
    peers = $state([]);
    accounts = $state([]);
    members = $state([]);
    ownAccounts = $state([]);
    messages = $state([]);
    error = $state('');
    localIp = $state('');

    _roomId = '';
    _nodes = [];
    _unlisten = null;
    _mediaUnlisten = null;
    _presenceTimer = 0;
    _pending = new Map();
    _handlers = new Map();

    async _resolveLocalIp() {
      this.localIp = sm.networkConfigState.ipv4 || this.localIp;
      if (!isTauri()) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const raw = await invoke('get_running_info');
        const info = typeof raw === 'string' ? JSON.parse(raw) : raw;
        const address = info?.my_node_info?.virtual_ipv4?.address?.addr;
        if (address != null) this.localIp = intToIp(address);
      } catch {
        // DHCP 下状态刷新后会再次带入本机虚拟 IP。
      }
    }

    async start(room, nodes = []) {
      if (!room || !isTauri()) return;
      this._nodes = nodes;
      const id = await roomIdentity(room);
      if (this.connected && this._roomId === id) {
        this.setNodes(nodes);
        return;
      }
      this.starting = true;
      this.error = '';
      try {
        await this._resolveLocalIp();
        const [{ invoke }, { listen }] = await Promise.all([
          import('@tauri-apps/api/core'),
          import('@tauri-apps/api/event'),
        ]);
        await invoke('room_social_stop');
        if (!this._unlisten) {
          this._unlisten = await listen('room-social', (event) => this._receive(event.payload));
        }
        if (!this._mediaUnlisten) {
          this._mediaUnlisten = await listen('room-media', (event) => {
            const payload = event.payload;
            this._emit('tcp-media', payload, payload?.peer || '');
          });
        }
        await invoke('room_social_start', { roomId: id });
        this._roomId = id;
        this.connected = true;
        this.messages = [];
        this.accounts = [];
        this.members = [];
        this.setNodes(nodes);
        clearInterval(this._presenceTimer);
        await this.refreshOwnAccounts();
        await this.broadcastPresence();
        this._presenceTimer = setInterval(async () => {
          await this.refreshOwnAccounts();
          await this.broadcastPresence();
          const cutoff = Date.now() - 15000;
          this.accounts = this.accounts.filter((a) => a.lastSeen >= cutoff);
          this.members = this.members.filter((m) => m.lastSeen >= cutoff);
        }, 5000);
      } catch (e) {
        this.error = String(e?.message || e);
        this.connected = false;
      } finally {
        this.starting = false;
      }
    }

    async stop() {
      clearInterval(this._presenceTimer);
      this._presenceTimer = 0;
      this.connected = false;
      this._roomId = '';
      this.accounts = [];
      this.members = [];
      this.peers = [];
      for (const pending of this._pending.values()) {
        clearTimeout(pending.timer);
        pending.reject(new Error('已离开房间'));
      }
      this._pending.clear();
      this._emit('room-leave', {});
      if (isTauri()) {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('room_social_stop');
        } catch { /* 已断开 */ }
      }
    }

    setNodes(nodes) {
      this._nodes = nodes || [];
      const localIps = new Set([this.localIp, sm.networkConfigState.ipv4].filter(Boolean));
      this.peers = [...new Set(this._nodes
        .filter((n) => n?.ipv4 && n.ipv4 !== '0.0.0.0' && !localIps.has(n.ipv4) && !isServerNode(n))
        .map((n) => n.ipv4))];
    }

    on(kind, handler) {
      const list = this._handlers.get(kind) || [];
      list.push(handler);
      this._handlers.set(kind, list);
      return () => {
        this._handlers.set(kind, (this._handlers.get(kind) || []).filter((h) => h !== handler));
      };
    }

    _emit(kind, payload, peer = '') {
      for (const handler of this._handlers.get(kind) || []) {
        try { handler(payload, peer); } catch (e) { console.warn(`房间事件 ${kind} 处理失败`, e); }
      }
    }

    async refreshOwnAccounts() {
      if (!isTauri()) return [];
      const { invoke } = await import('@tauri-apps/api/core');
      const list = [];
      for (const platform of Object.keys(PLATFORM_LABELS)) {
        if (sm.bgmState.roomShare?.[platform] === false) continue;
        try {
          const status = await invoke('music_status', { platform });
          if (status?.loggedIn) {
            list.push({ platform, nickname: status.nickname || sm.playerState.playerName || PLATFORM_LABELS[platform] });
          }
        } catch { /* 未登录平台不广播 */ }
      }
      this.ownAccounts = list;
      return list;
    }

    async _sendOne(peer, kind, payload = {}) {
      if (!this.connected || !peer) throw new Error('房间协作尚未连接');
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('room_social_send', {
        peer,
        message: {
          room: this._roomId,
          kind,
          from: this.localIp,
          sender: sm.playerState.playerName || '房员',
          sentAt: Date.now(),
          payload,
        },
      });
    }

    async broadcast(kind, payload = {}) {
      if (!this.connected) return [];
      return Promise.allSettled(this.peers.map((peer) => this._sendOne(peer, kind, payload)));
    }

    async broadcastPresence(replyTo = '') {
      const payload = { accounts: this.ownAccounts, reply: !!replyTo };
      if (replyTo) await this._sendOne(replyTo, 'presence', payload);
      else await this.broadcast('presence', payload);
    }

    async sendChat(text) {
      const message = text.trim();
      if (!message) return;
      if (message.length > 2000) throw new Error('单条消息不能超过 2000 字');
      const item = {
        id: uuid(), text: message, sender: sm.playerState.playerName || '我',
        peer: this.localIp, sentAt: Date.now(), local: true,
      };
      this.messages = [...this.messages.slice(-199), item];
      await this.broadcast('chat', { id: item.id, text: message });
    }

    _receive(event) {
      const message = event?.message;
      const peer = event?.peer || message?.from || '';
      if (!this.connected || message?.room !== this._roomId) return;
      if (peer && this.peers.length > 0 && !this.peers.includes(peer)) return;
      if (peer && peer === this.localIp) return;
      const kind = message.kind;
      const payload = message.payload || {};
      if (kind === 'presence') {
        const now = Date.now();
        this.members = [
          ...this.members.filter((m) => m.peer !== peer),
          { peer, nickname: message.sender || '房员', lastSeen: now },
        ];
        this.accounts = this.accounts.filter((a) => a.peer !== peer);
        const additions = (payload.accounts || []).map((a) => ({
          peer,
          owner: message.sender || '房员',
          accountName: a.nickname || message.sender || PLATFORM_LABELS[a.platform],
          platform: a.platform,
          label: PLATFORM_LABELS[a.platform] || a.platform,
          lastSeen: now,
        }));
        this.accounts = [...this.accounts, ...additions];
        if (!payload.reply) this.broadcastPresence(peer).catch(() => {});
        return;
      }
      if (kind === 'chat') {
        if (typeof payload.text !== 'string' || payload.text.length > 2000) return;
        const item = {
          id: payload.id || uuid(), text: payload.text,
          sender: message.sender || '房员', peer, sentAt: message.sentAt || Date.now(), local: false,
        };
        if (!this.messages.some((m) => m.id === item.id)) {
          this.messages = [...this.messages.slice(-199), item];
        }
        return;
      }
      if (kind === 'music_search_request') {
        this._answerSearch(peer, message.sender, payload).catch(() => {});
        return;
      }
      if (kind === 'music_stream_request') {
        this._answerStream(peer, payload).catch(() => {});
        return;
      }
      if (kind === 'music_search_response' || kind === 'music_stream_response') {
        const pending = this._pending.get(payload.requestId);
        if (!pending || pending.peer !== peer) return;
        this._pending.delete(payload.requestId);
        clearTimeout(pending.timer);
        if (payload.error) pending.reject(new Error(payload.error));
        else pending.resolve(payload);
        return;
      }
      this._emit(kind, payload, peer);
    }

    async _canShare(platform) {
      if (sm.bgmState.roomShare?.[platform] === false) return null;
      const { invoke } = await import('@tauri-apps/api/core');
      const status = await invoke('music_status', { platform });
      return status?.loggedIn ? status : null;
    }

    async _answerSearch(peer, _sender, payload) {
      const base = { requestId: payload.requestId, platform: payload.platform };
      try {
        if (!Object.keys(PLATFORM_LABELS).includes(payload.platform) || typeof payload.query !== 'string' || payload.query.length > 200) {
          throw new Error('搜索请求无效');
        }
        const status = await this._canShare(payload.platform);
        if (!status) throw new Error('账号共享已关闭');
        const { invoke } = await import('@tauri-apps/api/core');
        const result = await invoke('music_search', { platform: payload.platform, query: payload.query.trim() });
        await this._sendOne(peer, 'music_search_response', { ...base, tracks: result?.tracks || [] });
      } catch (e) {
        await this._sendOne(peer, 'music_search_response', { ...base, error: String(e?.message || e) }).catch(() => {});
      }
    }

    _request(kind, account, payload, timeoutMs = 25000) {
      const requestId = uuid();
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          this._pending.delete(requestId);
          reject(new Error('账号端响应超时'));
        }, timeoutMs);
        this._pending.set(requestId, { peer: account.peer, resolve, reject, timer });
        this._sendOne(account.peer, kind, { ...payload, requestId }).catch((e) => {
          clearTimeout(timer);
          this._pending.delete(requestId);
          reject(e);
        });
      });
    }

    async search(account, query) {
      if (!account || account.platform == null) throw new Error('请选择共享账号');
      const response = await this._request('music_search_request', account, {
        platform: account.platform, query: query.trim(),
      });
      return (response.tracks || []).map((track) => ({ ...track, sourceAccount: account }));
    }

    async resolveSharedTrack(track) {
      const account = track.sourceAccount;
      if (!account?.peer) throw new Error('曲目缺少共享账号');
      const response = await this._request('music_stream_request', account, {
        platform: track.platform,
        track: {
          platform: track.platform, key: track.key, id: track.id,
          mediaMid: track.mediaMid || '', title: track.title || '',
          artists: track.artists || [], durationMs: track.durationMs || 0,
          vip: track.vip === true,
          cover: track.cover || '', albumId: track.albumId || '', albumAudioId: track.albumAudioId || '',
        },
      }, 35000);
      const { invoke } = await import('@tauri-apps/api/core');
      const sharedStream = await invoke('room_music_stream_url', {
        peer: account.peer,
        streamPort: response.streamPort,
        streamId: response.streamId,
      });
      return {
        ...track,
        sharedStream,
        sharedLyrics: response.lyrics?.lines || [],
        sharedAccountName: account.accountName,
      };
    }

    async _answerStream(peer, payload) {
      const base = { requestId: payload.requestId };
      try {
        const track = payload.track;
        const status = await this._canShare(payload.platform);
        if (!status) throw new Error('账号共享已关闭');
        if (!track || track.platform !== payload.platform || typeof track.key !== 'string') {
          throw new Error('共享曲目无效');
        }
        const { invoke } = await import('@tauri-apps/api/core');
        const offer = await invoke('music_stream_offer', { track });
        await this._sendOne(peer, 'music_stream_response', { ...base, ...offer });
      } catch (e) {
        await this._sendOne(peer, 'music_stream_response', { ...base, error: String(e?.message || e) }).catch(() => {});
      }
    }
  }

  export const roomSocial = new RoomSocialState();
</script>
