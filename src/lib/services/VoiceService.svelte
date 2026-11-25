<script module>
  /** 多频道语音：频道主维护成员/邀请/禁言名单，音频走房员间 TCP 媒体通道。
   * 隐藏频道不公开广播；邀请、批准和踢出均为定向房间消息。 */
  import { roomSocial } from './RoomSocial.svelte';
  import { sm } from './ServiceManager.svelte';
  import { tcpMediaPort, chooseMime, TcpRecorder, TcpMediaReceiver } from './TcpMedia.svelte';

  function id() {
    return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`;
  }
  function copyChannel(channel) {
    return channel ? { ...channel, members: (channel.members || []).map((m) => ({ ...m })) } : null;
  }

  class VoiceState {
    channels = $state([]);
    invites = $state([]);
    activeChannelId = $state('');
    muted = $state(false);
    busy = $state(false);
    error = $state('');
    get nickname() {
      return sm.playerState.playerName || '房员';
    }

    _activeChannel = null;
    _owned = new Map();
    _remote = new Map();
    _invited = new Map();
    _pendingJoins = new Map();
    _stream = null;
    _mediaPeers = new Map();
    _receivers = new Map();
    _recorder = null;
    _mediaPort = 0;
    _mediaSession = '';
    _timer = 0;
    _started = false;
    _manualMuted = false;

    constructor() {
      roomSocial.on('voice_presence', (p, peer) => this._presence(p, peer));
      roomSocial.on('voice_invite', (p, peer) => this._invitation(p, peer));
      roomSocial.on('voice_join_request', (p, peer) => this._joinRequest(p, peer));
      roomSocial.on('voice_join_accept', (p, peer) => this._joinAccepted(p, peer));
      roomSocial.on('voice_join_reject', (p, peer) => this._joinRejected(p, peer));
      roomSocial.on('voice_join', (p, peer) => this._joined(p, peer));
      roomSocial.on('voice_leave', (p, peer) => this._memberLeft(p, peer));
      roomSocial.on('voice_channel_state', (p, peer) => this._channelState(p, peer));
      roomSocial.on('voice_kick', (p, peer) => this._kicked(p, peer));
      roomSocial.on('tcp-media', (p, peer) => this._tcpMedia(p, peer));
      roomSocial.on('room-leave', () => this._roomLeft());
    }

    start() {
      if (this._started) return;
      this._started = true;
      this._timer = setInterval(() => {
        const now = Date.now();
        for (const [peer, row] of this._remote) {
          if (row.time < now - 15000) this._remote.delete(peer);
        }
        const validInvites = this.invites.filter((invite) => invite.expiresAt > now);
        if (validInvites.length !== this.invites.length) this.invites = validInvites;
        for (const [channelId, invite] of this._invited) {
          if (invite.expiresAt <= now && this.activeChannelId !== channelId) this._invited.delete(channelId);
        }
        this._rebuild();
        if (roomSocial.connected && this._owned.size) this._broadcastPresence();
      }, 4000);
    }

    isOwner(channel = this._activeChannel) {
      return !!channel && channel.owner === roomSocial.localIp;
    }

    selfMuted(channel = this._activeChannel) {
      return !!channel?.members?.find((m) => m.peer === roomSocial.localIp)?.muted;
    }

    async createChannel(name, policy = {}) {
      const channelName = name.trim();
      if (!channelName) throw new Error('请输入频道名称');
      if (channelName.length > 32) throw new Error('频道名称不能超过 32 字');
      const self = roomSocial.localIp;
      const channel = {
        id: id(), name: channelName, owner: self,
        ownerName: this.nickname.trim() || sm.playerState.playerName || '频道主',
        hidden: !!policy.hidden, inviteOnly: !!policy.inviteOnly,
        members: [{ peer: self, nickname: this.nickname.trim() || '频道主', muted: false }],
        banned: [], invites: {},
      };
      await this._activate(channel);
      this._owned.set(channel.id, channel);
      this._activeChannel = copyChannel(channel);
      this._rebuild();
      await this._broadcastPresence();
    }

    async join(channel) {
      if (this.busy || this.activeChannelId === channel.id) return;
      this.busy = true;
      this.error = '';
      try {
        if (!roomSocial.connected) throw new Error('请先连接房间');
        const owned = this._owned.get(channel.id);
        if (owned) {
          if (!owned.members.some((m) => m.peer === roomSocial.localIp)) {
            owned.members.push({ peer: roomSocial.localIp, nickname: this.nickname.trim() || '频道主', muted: false });
          }
          await this._activate(owned);
          await this._broadcastPresence();
          return;
        }
        if (channel.inviteOnly || channel.hidden) {
          const invite = this._invited.get(channel.id);
          if (!invite || invite.expiresAt <= Date.now()) throw new Error('需要频道主的有效邀请');
          const approved = await this._requestJoin(channel, invite);
          await this._activate(approved);
          this.invites = this.invites.filter((item) => item.id !== invite.id);
        } else {
          await this._activate(channel);
        }
      } catch (e) {
        this.error = String(e?.message || e);
        throw e;
      } finally {
        this.busy = false;
      }
    }

    async acceptInvite(invite) {
      await this.join(invite.channel);
      this.invites = this.invites.filter((item) => item.id !== invite.id);
    }

    declineInvite(inviteId) {
      this.invites = this.invites.filter((invite) => invite.id !== inviteId);
      for (const [channelId, invite] of this._invited) {
        if (invite.id === inviteId && this.activeChannelId !== channelId) this._invited.delete(channelId);
      }
      this._rebuild();
    }

    async _requestJoin(channel, invite) {
      const requestId = id();
      const approved = new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          this._pendingJoins.delete(requestId);
          reject(new Error('频道主响应超时'));
        }, 12000);
        this._pendingJoins.set(requestId, { resolve, reject, timer, owner: channel.owner });
      });
      await roomSocial._sendOne(channel.owner, 'voice_join_request', {
        requestId, channelId: channel.id, inviteId: invite.id,
        nickname: this.nickname.trim() || '房员',
      }).catch((e) => {
        const pending = this._pendingJoins.get(requestId);
        if (pending) { clearTimeout(pending.timer); this._pendingJoins.delete(requestId); pending.reject(e); }
      });
      return approved;
    }

    _joinPayload(reply = false) {
      return {
        channelId: this.activeChannelId, nickname: this.nickname.trim() || '房员',
        mediaPort: this._mediaPort, mediaSession: this._mediaSession, reply,
      };
    }

    async _activate(channel) {
      if (!navigator.mediaDevices?.getUserMedia) throw new Error('当前平台不支持麦克风采集');
      await this.leave();
      this._stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: true, noiseSuppression: true, autoGainControl: true }, video: false,
      });
      this.activeChannelId = channel.id;
      this._mediaPort = await tcpMediaPort();
      this._mediaSession = `voice-${channel.id}-${roomSocial.localIp}-${id()}`;
      const owned = this._owned.get(channel.id);
      if (owned && !owned.members.some((m) => m.peer === roomSocial.localIp)) {
        owned.members.push({ peer: roomSocial.localIp, nickname: this.nickname.trim() || '频道主', muted: false });
      }
      this._activeChannel = copyChannel(owned || channel);
      this._manualMuted = false;
      this._syncMute();
      this._rebuild();
      await roomSocial.broadcast('voice_join', this._joinPayload());
      if (this.isOwner(channel)) {
        await this._broadcastPresence();
        await this._broadcastState(this._owned.get(channel.id) || channel);
      }
    }

    async leave(notify = true) {
      const channelId = this.activeChannelId;
      if (!channelId && !this._stream) return;
      if (notify && roomSocial.connected && channelId) {
        await roomSocial.broadcast('voice_leave', { channelId }).catch(() => {});
      }
      for (const peer of [...this._mediaPeers.keys()]) this._closePeer(peer);
      this._stopMedia();
      const owned = this._owned.get(channelId);
      if (owned) {
        owned.members = owned.members.filter((m) => m.peer !== roomSocial.localIp);
        this._activeChannel = null;
        if (notify) {
          await this._broadcastPresence().catch(() => {});
          await this._broadcastState(owned).catch(() => {});
        }
      }
      this.activeChannelId = '';
      this._activeChannel = null;
      this.muted = false;
      this._manualMuted = false;
      this._rebuild();
    }

    toggleMute() {
      if (this.selfMuted()) {
        this.error = '频道主已将你禁言';
        return;
      }
      this._manualMuted = !this._manualMuted;
      this._syncMute();
    }

    _syncMute() {
      this.muted = this._manualMuted || this.selfMuted();
      if (this._stream) for (const track of this._stream.getAudioTracks()) track.enabled = !this.muted;
    }

    _stopMedia() {
      const recorder = this._recorder;
      this._recorder = null;
      recorder?.stop();
      for (const receiver of this._receivers.values()) receiver.dispose();
      this._receivers.clear();
      this._mediaPeers.clear();
      if (this._stream) for (const track of this._stream.getTracks()) track.stop();
      this._stream = null;
      this._mediaSession = '';
    }

    async setPolicy(channelId, patch) {
      const channel = this._owned.get(channelId);
      if (!channel) throw new Error('只有频道主可以修改频道');
      if (typeof patch.hidden === 'boolean') channel.hidden = patch.hidden;
      if (typeof patch.inviteOnly === 'boolean') channel.inviteOnly = patch.inviteOnly;
      if (this.activeChannelId === channelId) this._activeChannel = copyChannel(channel);
      this._rebuild();
      await this._broadcastPresence();
      await this._broadcastState(channel);
    }

    async invite(channelId, peer) {
      const channel = this._owned.get(channelId);
      if (!channel) throw new Error('只有频道主可以邀请');
      if (!roomSocial.members.some((m) => m.peer === peer)) throw new Error('该房员不在线');
      if (channel.banned.includes(peer)) throw new Error('该房员已被移出频道');
      const invite = { id: id(), peer, expiresAt: Date.now() + 5 * 60_000 };
      channel.invites[invite.id] = invite;
      await roomSocial._sendOne(peer, 'voice_invite', {
        invite: { id: invite.id, expiresAt: invite.expiresAt },
        channel: this._safeChannel(channel),
      });
    }

    async setMemberMuted(channelId, peer, muted) {
      const channel = this._owned.get(channelId);
      const member = channel?.members.find((m) => m.peer === peer);
      if (!channel || !member) throw new Error('成员不在频道中');
      member.muted = !!muted;
      if (peer === roomSocial.localIp) this._syncMute();
      if (muted) this._closePeer(peer);
      if (this.activeChannelId === channelId) this._activeChannel = copyChannel(channel);
      this._rebuild();
      await this._broadcastState(channel);
      await this._broadcastPresence();
    }

    async kickMember(channelId, peer) {
      const channel = this._owned.get(channelId);
      if (!channel || peer === roomSocial.localIp) return;
      if (!channel.banned.includes(peer)) channel.banned.push(peer);
      channel.members = channel.members.filter((m) => m.peer !== peer);
      for (const invite of Object.values(channel.invites)) {
        if (invite.peer === peer) delete channel.invites[invite.id];
      }
      this._closePeer(peer);
      await roomSocial._sendOne(peer, 'voice_kick', { channelId }).catch(() => {});
      this._rebuild();
      await this._broadcastPresence();
      await this._broadcastState(channel);
    }

    _safeChannel(channel) {
      return {
        id: channel.id, name: channel.name, owner: channel.owner, ownerName: channel.ownerName,
        hidden: !!channel.hidden, inviteOnly: !!channel.inviteOnly,
        members: (channel.members || []).map((m) => ({ peer: m.peer, nickname: m.nickname, muted: !!m.muted })),
      };
    }

    async _broadcastPresence() {
      const channels = [...this._owned.values()].filter((c) => !c.hidden).map((c) => this._safeChannel(c));
      await roomSocial.broadcast('voice_presence', { channels });
    }

    async _broadcastState(channel) {
      if (!channel) return;
      const safe = this._safeChannel(channel);
      await Promise.allSettled(channel.members
        .filter((m) => m.peer !== roomSocial.localIp)
        .map((m) => roomSocial._sendOne(m.peer, 'voice_channel_state', { channel: safe })));
    }

    _presence(payload, peer) {
      const channels = (Array.isArray(payload.channels) ? payload.channels : [])
        .filter((c) => c?.owner === peer && !c.hidden);
      this._remote.set(peer, { channels, time: Date.now() });
      this._rebuild();
    }

    _invitation(payload, peer) {
      const invite = payload.invite;
      const channel = payload.channel;
      if (!invite?.id || !channel?.id || channel.owner !== peer || invite.expiresAt <= Date.now()) return;
      const row = { id: invite.id, expiresAt: invite.expiresAt, channel };
      this.invites = [...this.invites.filter((item) => item.id !== invite.id), row];
      this._invited.set(channel.id, row);
      this._rebuild();
    }

    async _joinRequest(payload, peer) {
      const channel = this._owned.get(payload.channelId);
      const base = { requestId: payload.requestId, channelId: payload.channelId };
      if (!channel) {
        await roomSocial._sendOne(peer, 'voice_join_reject', { ...base, error: '频道已不存在' }).catch(() => {});
        return;
      }
      const invite = channel.invites[payload.inviteId];
      if (!invite || invite.peer !== peer || invite.expiresAt <= Date.now() || channel.banned.includes(peer)) {
        await roomSocial._sendOne(peer, 'voice_join_reject', { ...base, error: '邀请无效或已被移出频道' });
        return;
      }
      delete channel.invites[invite.id];
      if (!channel.members.some((m) => m.peer === peer)) {
        channel.members.push({ peer, nickname: String(payload.nickname || '房员').slice(0, 32), muted: false });
      }
      await roomSocial._sendOne(peer, 'voice_join_accept', { ...base, channel: this._safeChannel(channel) });
      await this._broadcastPresence();
      await this._broadcastState(channel);
      this._rebuild();
    }

    _joinAccepted(payload, peer) {
      const pending = this._pendingJoins.get(payload.requestId);
      if (!pending || pending.owner !== peer || !payload.channel) return;
      clearTimeout(pending.timer);
      this._pendingJoins.delete(payload.requestId);
      pending.resolve(payload.channel);
    }

    _joinRejected(payload, peer) {
      const pending = this._pendingJoins.get(payload.requestId);
      if (!pending || pending.owner !== peer) return;
      clearTimeout(pending.timer);
      this._pendingJoins.delete(payload.requestId);
      pending.reject(new Error(payload.error || '频道主拒绝了加入请求'));
    }

    async _joined(payload, peer) {
      const owned = this._owned.get(payload.channelId);
      if (owned) {
        if (owned.banned.includes(peer)) {
          await roomSocial._sendOne(peer, 'voice_kick', { channelId: owned.id }).catch(() => {});
          return;
        }
        const alreadyMember = owned.members.some((m) => m.peer === peer);
        if (!alreadyMember && (owned.inviteOnly || owned.hidden)) {
          // 批准流程会先把受邀者加入 members；除此之外的加入广播一律拒绝。
          await roomSocial._sendOne(peer, 'voice_kick', { channelId: owned.id }).catch(() => {});
          return;
        }
        if (!alreadyMember) {
          owned.members.push({ peer, nickname: String(payload.nickname || '房员').slice(0, 32), muted: false });
          await this._broadcastPresence();
          await this._broadcastState(owned);
          this._rebuild();
        }
      }
      if (payload.channelId !== this.activeChannelId || !this._stream) return;
      this._addMediaPeer(peer, payload);
      if (!payload.reply) {
        await roomSocial._sendOne(peer, 'voice_join', this._joinPayload(true)).catch(() => {});
      }
    }

    async _memberLeft(payload, peer) {
      this._closePeer(peer);
      const owned = this._owned.get(payload.channelId);
      if (owned) {
        owned.members = owned.members.filter((m) => m.peer !== peer);
        await this._broadcastPresence();
        await this._broadcastState(owned);
        this._rebuild();
      }
    }

    _channelState(payload, peer) {
      const channel = payload.channel;
      if (!channel?.id || channel.owner !== peer) return;
      const previous = this._activeChannel;
      this._invited.set(channel.id, { id: `member-${channel.id}`, expiresAt: Date.now() + 15000, channel });
      if (this.activeChannelId === channel.id) {
        if (!channel.members.some((m) => m.peer === roomSocial.localIp)) {
          this.error = '你已不在此频道中';
          this.leave(false);
          return;
        }
        const wasMuted = this.selfMuted(previous);
        this._activeChannel = copyChannel(channel);
        this._syncMute();
        const nowMuted = this.selfMuted(channel);
        if (wasMuted && !nowMuted) roomSocial.broadcast('voice_join', this._joinPayload()).catch(() => {});
      }
      this._rebuild();
    }

    async _kicked(payload, peer) {
      if (this._activeChannel?.owner !== peer || payload.channelId !== this.activeChannelId) {
        this.invites = this.invites.filter((invite) => invite.channel.owner !== peer || invite.channel.id !== payload.channelId);
        return;
      }
      this.error = '你已被频道主移出频道';
      this._invited.delete(payload.channelId);
      await this.leave(false);
    }

    _rebuild() {
      const map = new Map();
      const add = (channel) => {
        if (!channel?.id || !channel.owner) return;
        map.set(channel.id, copyChannel(this._safeChannel(channel)));
      };
      for (const channel of this._owned.values()) add(channel);
      for (const row of this._remote.values()) for (const channel of row.channels) add(channel);
      for (const invite of this._invited.values()) add(invite.channel);
      if (this._activeChannel) add(this._activeChannel);
      this.channels = [...map.values()].sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'));
    }

    async _roomLeft() {
      await this.leave(false);
      this._owned.clear();
      this._remote.clear();
      this._invited.clear();
      this.invites = [];
      this.channels = [];
      for (const pending of this._pendingJoins.values()) { clearTimeout(pending.timer); pending.reject(new Error('已离开房间')); }
      this._pendingJoins.clear();
    }

    _addMediaPeer(peer, payload) {
      if (!payload.mediaPort || !payload.mediaSession || peer === roomSocial.localIp) return;
      this._mediaPeers.set(peer, { peer, port: payload.mediaPort, session: payload.mediaSession });
      let receiver = this._receivers.get(peer);
      if (receiver && receiver.session !== payload.mediaSession) {
        receiver.dispose();
        receiver = null;
      }
      if (!receiver) {
        const audio = new Audio();
        audio.autoplay = true;
        receiver = new TcpMediaReceiver({
          kind: 'voice', session: payload.mediaSession, peer, element: audio,
          onError: (error) => { this.error = `语音接收失败：${error?.message || error}`; },
          onEnded: () => this._closePeer(peer),
        });
        this._receivers.set(peer, receiver);
      }
      this._ensureRecorder();
    }

    _ensureRecorder() {
      if (this._recorder || !this._stream || !this._mediaPeers.size || this.selfMuted()) return;
      this._recorder = new TcpRecorder({
        stream: this._stream, session: this._mediaSession, kind: 'voice',
        mime: chooseMime(['audio/webm;codecs=opus', 'audio/webm']),
        audioBitsPerSecond: 96000, timeslice: 100,
        targets: () => [...this._mediaPeers.values()],
        onError: (error) => { this.error = `语音发送失败：${error?.message || error}`; },
      });
      this._recorder.start();
    }

    _tcpMedia(payload, peer) {
      this._receivers.get(peer)?.accept(payload);
    }

    _closePeer(peer) {
      this._mediaPeers.delete(peer);
      const receiver = this._receivers.get(peer);
      receiver?.dispose();
      this._receivers.delete(peer);
    }
  }

  export const voiceService = new VoiceState();
</script>
