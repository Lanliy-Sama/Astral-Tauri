<script module>
  /** 多成员一起听：主机同步曲目与进度，成员操作回传主机。 */
  import { roomSocial } from './RoomSocial.svelte';
  import { sm } from './ServiceManager.svelte';
  import { bgmPlayOnlineTrack, bgmToggleAny, bgmNext, bgmPrevious, bgmSeek } from './BgmPlayer.svelte';
  import { t } from '../i18n/I18n.svelte';

  function id() { return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`; }
  function delay(ms) { return new Promise((resolve) => setTimeout(resolve, ms)); }

  class RoomListenState {
    active = $state(false);
    isHost = $state(false);
    sessionId = $state('');
    hostPeer = $state('');
    hostName = $state('');
    members = $state([]);
    incoming = $state(null);
    error = $state('');

    _pending = new Map();
    _applying = false;
    _lastTrackKey = '';

    constructor() {
      roomSocial.on('listen_invite', (p, peer) => this._invited(p, peer));
      roomSocial.on('listen_accept', (p, peer) => this._accepted(p, peer));
      roomSocial.on('listen_state', (p, peer) => this._state(p, peer));
      roomSocial.on('listen_action', (p, peer) => this._action(p, peer));
      roomSocial.on('listen_leave', (p, peer) => this._leaveMember(p, peer));
      roomSocial.on('listen_end', (p, peer) => this._ended(p, peer));
      roomSocial.on('local_stream_request', (p, peer) => this._localOffer(p, peer));
      roomSocial.on('local_stream_response', (p, peer) => this._localResponse(p, peer));
      roomSocial.on('room-leave', () => this.stop(false));
      setInterval(() => {
        if (this.active && this.isHost) this.broadcastState();
      }, 2000);
    }

    currentTrack() {
      const online = sm.bgmState.onlineTrack;
      if (online) {
        return {
          ...online,
          sourceAccount: online.sourceAccount || {
            peer: roomSocial.localIp,
            accountName: online.sharedAccountName || sm.playerState.playerName || '房员',
          },
        };
      }
      const local = sm.bgmState.tracks.find((track) => track.trackId === sm.bgmState.trackId);
      return local ? {
        key: `local:${local.trackId}`, id: local.trackId, localTrackId: local.trackId,
        platform: 'local', title: local.name, artists: ['本地音乐'], cover: '', durationMs: 0,
      } : null;
    }

    async start() {
      if (!roomSocial.connected) throw new Error(t('connect_room_first'));
      this.active = true;
      this.isHost = true;
      this.sessionId = id();
      this.hostPeer = roomSocial.localIp;
      this.hostName = sm.playerState.playerName || '房员';
      this.members = [];
      this.error = '';
      await roomSocial.broadcast('listen_invite', {
        sessionId: this.sessionId, hostName: this.hostName,
        track: this.currentTrack(),
      });
      await this.broadcastState();
    }

    async accept() {
      const invite = this.incoming;
      if (!invite) return;
      this.active = true;
      this.isHost = false;
      this.sessionId = invite.sessionId;
      this.hostPeer = invite.peer;
      this.hostName = invite.hostName;
      this.members = [];
      this.incoming = null;
      await roomSocial._sendOne(this.hostPeer, 'listen_accept', {
        sessionId: this.sessionId,
        nickname: sm.playerState.playerName || '房员',
      });
    }

    decline() { this.incoming = null; }

    async stop(notify = true) {
      if (notify && this.active) {
        await roomSocial.broadcast(this.isHost ? 'listen_end' : 'listen_leave', {
          sessionId: this.sessionId,
        }).catch(() => {});
      }
      this.active = false;
      this.isHost = false;
      this.sessionId = '';
      this.hostPeer = '';
      this.hostName = '';
      this.members = [];
      this.incoming = null;
      this._lastTrackKey = '';
    }

    _invited(payload, peer) {
      if (this.active || !payload.sessionId) return;
      this.incoming = { ...payload, peer };
    }

    async _accepted(payload, peer) {
      if (!this.active || !this.isHost || payload.sessionId !== this.sessionId) return;
      if (!this.members.some((member) => member.peer === peer)) {
        this.members = [...this.members, { peer, nickname: payload.nickname || '房员' }];
      }
      await this.broadcastState(peer);
    }

    async broadcastState(targetPeer = '') {
      if (!this.active || !this.isHost || this._applying) return;
      const payload = {
        sessionId: this.sessionId,
        track: this.currentTrack(),
        playing: sm.bgmState.playing,
        positionMs: Math.round((await import('./BgmPlayer.svelte')).bgmPosition() * 1000),
        sentAt: Date.now(),
      };
      if (targetPeer) await roomSocial._sendOne(targetPeer, 'listen_state', payload).catch(() => {});
      else await roomSocial.broadcast('listen_state', payload);
    }

    notify() {
      if (this.active && this.isHost && !this._applying) this.broadcastState();
    }

    handleLocalAction(action) {
      if (!this.active || this.isHost || this._applying) return false;
      roomSocial._sendOne(this.hostPeer, 'listen_action', { sessionId: this.sessionId, action }).catch(() => {});
      return true;
    }

    async _action(payload, peer) {
      if (!this.active || !this.isHost || payload.sessionId !== this.sessionId || !this.members.some((m) => m.peer === peer)) return;
      if (payload.action === 'next') await bgmNext();
      else if (payload.action === 'previous') await bgmPrevious();
      else if (payload.action === 'toggle') bgmToggleAny();
      else if (payload.action === 'seek' && Number.isFinite(payload.positionMs)) bgmSeek(payload.positionMs / 1000);
      await this.broadcastState();
    }

    async _state(payload, peer) {
      if (!this.active || this.isHost || peer !== this.hostPeer || payload.sessionId !== this.sessionId) return;
      this._applying = true;
      try {
        const track = payload.track;
        if (track?.key && track.key !== this._lastTrackKey) {
          this._lastTrackKey = track.key;
          await this._playRemote(track, payload.positionMs || 0);
        } else if (track) {
          if (payload.playing !== sm.bgmState.playing) bgmToggleAny();
          const current = (await import('./BgmPlayer.svelte')).bgmPosition() * 1000;
          const expected = (payload.positionMs || 0) + (payload.playing ? Date.now() - payload.sentAt : 0);
          if (Math.abs(current - expected) > 1600) bgmSeek(expected / 1000);
        }
      } catch (error) {
        this.error = String(error?.message || error);
      } finally {
        this._applying = false;
      }
    }

    async _playRemote(track, positionMs) {
      let playable = { ...track };
      if (track.platform === 'local' && track.localTrackId) {
        const offer = await this._requestLocal(track.localTrackId);
        const { invoke } = await import('@tauri-apps/api/core');
        playable.sharedStream = await invoke('room_music_stream_url', {
          peer: this.hostPeer, streamPort: offer.streamPort, streamId: offer.streamId,
        });
        playable.sharedLyrics = [];
      } else if (track.sourceAccount) {
        playable = await roomSocial.resolveSharedTrack(track);
      }
      sm.bgmState.onlineQueue = [playable];
      sm.bgmState.onlineIndex = 0;
      await bgmPlayOnlineTrack(playable);
      if (positionMs > 0) bgmSeek(positionMs / 1000);
    }

    _requestLocal(trackId) {
      const requestId = id();
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          this._pending.delete(requestId);
          reject(new Error(t('download_timeout')));
        }, 20000);
        this._pending.set(requestId, { resolve, reject, timer });
        roomSocial._sendOne(this.hostPeer, 'local_stream_request', { requestId, trackId }).catch(reject);
      });
    }

    async _localOffer(payload, peer) {
      const base = { requestId: payload.requestId };
      try {
        if (!this.active || !this.isHost || !this.members.some((m) => m.peer === peer)) throw new Error(t('listen_not_member'));
        const { invoke } = await import('@tauri-apps/api/core');
        const offer = await invoke('local_music_stream_offer', { trackId: payload.trackId });
        await roomSocial._sendOne(peer, 'local_stream_response', { ...base, ...offer });
      } catch (error) {
        await roomSocial._sendOne(peer, 'local_stream_response', { ...base, error: String(error?.message || error) }).catch(() => {});
      }
    }

    _localResponse(payload, peer) {
      const pending = this._pending.get(payload.requestId);
      if (!pending || peer !== this.hostPeer) return;
      this._pending.delete(payload.requestId);
      clearTimeout(pending.timer);
      if (payload.error) pending.reject(new Error(payload.error));
      else pending.resolve(payload);
    }

    _leaveMember(payload, peer) {
      if (this.isHost) this.members = this.members.filter((member) => member.peer !== peer);
    }

    _ended(payload, peer) {
      if (!this.isHost && peer === this.hostPeer && payload.sessionId === this.sessionId) {
        this.stop(false);
      }
    }
  }

  export const roomListen = new RoomListenState();
</script>
