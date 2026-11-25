<script module>
  /** 一起看：主机 MediaRecorder 编码，经独立 TCP 分片发送给已同意房员。
   * 不转发网站 Cookie、DOM 或输入事件；DRM 黑屏由平台/浏览器决定。 */
  import { roomSocial } from './RoomSocial.svelte';
  import { sm } from './ServiceManager.svelte';
  import { openExternal } from '../rust/Api.svelte';
  import { isTauri } from '../utils/Platform.svelte';
  import { t } from '../i18n/I18n.svelte';
  import { tcpMediaPort, chooseMime, TcpRecorder, TcpMediaReceiver } from './TcpMedia.svelte';

  function id() { return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`; }

  class WatchState {
    incoming = $state(null);
    session = $state(null);
    hosting = $state(false);
    joined = $state(false);
    starting = $state(false);
    remoteUrl = $state('');
    guests = $state([]);
    error = $state('');

    _stream = null;
    _recorder = null;
    _receiver = null;

    constructor() {
      roomSocial.on('watch_request', (p, peer) => this._request(p, peer));
      roomSocial.on('watch_accept', (p, peer) => this._accept(p, peer));
      roomSocial.on('watch_end', (p, peer) => this._ended(p, peer));
      roomSocial.on('watch_leave', (_p, peer) => this._closePeer(peer));
      roomSocial.on('tcp-media', (p, peer) => this._tcpMedia(p, peer));
      roomSocial.on('room-leave', () => this.stop(false));
    }

    async requestWatch(url, platform = '网页') {
      if (this.starting || this.hosting) return;
      this.starting = true;
      this.error = '';
      try {
        if (!roomSocial.connected) throw new Error(t('connect_room_first'));
        const target = new URL(url);
        if (target.protocol !== 'https:') throw new Error(t('https_only'));
        if (!navigator.mediaDevices?.getDisplayMedia) throw new Error(t('unsupported_capture'));
        let windowLabel = '';
        if (isTauri()) {
          const { invoke } = await import('@tauri-apps/api/core');
          windowLabel = await invoke('open_watch_window', { url: target.toString() });
        } else {
          await openExternal(target.toString());
        }
        try {
          this._stream = await navigator.mediaDevices.getDisplayMedia({
            video: { frameRate: { ideal: 30, max: 60 } }, audio: true,
          });
        } catch (error) {
          if (windowLabel && isTauri()) {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('close_watch_window', { label: windowLabel }).catch(() => {});
          }
          throw error;
        }
        this.session = {
          id: id(), url: target.toString(), platform, windowLabel,
          host: sm.playerState.playerName || '房员', startedAt: Date.now(),
        };
        this.hosting = true;
        this.joined = false;
        this.guests = [];
        this._stream.getVideoTracks()[0]?.addEventListener('ended', () => this.stop());
        await roomSocial.broadcast('watch_request', { session: this.session });
      } catch (e) {
        this.error = String(e?.message || e);
        this._stopAllMedia();
        this.hosting = false;
        this.session = null;
        throw e;
      } finally {
        this.starting = false;
      }
    }

    async accept() {
      if (!this.incoming || this.joined) return;
      this.error = '';
      try {
        const request = this.incoming;
        const mediaPort = await tcpMediaPort();
        this.session = request.session;
        this.joined = true;
        this.hosting = false;
        this._receiver = new TcpMediaReceiver({
          kind: 'watch', session: request.session.id, peer: request.peer,
          onUrl: (url) => { this.remoteUrl = url; },
          onError: (error) => { this.error = `视频接收失败：${error?.message || error}`; },
          onEnded: () => this.stop(false),
        });
        await roomSocial._sendOne(request.peer, 'watch_accept', {
          sessionId: request.session.id,
          nickname: sm.playerState.playerName || '房员', mediaPort,
        });
        this.incoming = null;
      } catch (e) {
        this.error = String(e?.message || e);
        this.joined = false;
        this.session = null;
        this._receiver?.dispose();
        this._receiver = null;
      }
    }

    decline() { this.incoming = null; }

    async stop(notify = true) {
      const current = this.session;
      if (notify && current && roomSocial.connected) {
        await roomSocial.broadcast(this.hosting ? 'watch_end' : 'watch_leave', { sessionId: current.id }).catch(() => {});
      }
      const recorder = this._recorder;
      if (current?.windowLabel && isTauri()) {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('close_watch_window', { label: current.windowLabel }).catch(() => {});
      }
      this._recorder = null;
      recorder?.stop();
      this._receiver?.dispose();
      this._receiver = null;
      this._stopAllMedia();
      this.session = null;
      this.hosting = false;
      this.joined = false;
      this.starting = false;
      this.guests = [];
      this.remoteUrl = '';
      this.incoming = null;
    }

    _stopAllMedia() {
      if (this._stream) for (const track of this._stream.getTracks()) track.stop();
      this._stream = null;
    }

    _request(payload, peer) {
      const session = payload.session;
      if (!session?.id || typeof session.url !== 'string' || !session.url.startsWith('https://')) return;
      if (this.hosting || this.joined || this.starting) return;
      this.incoming = { session, peer };
    }

    async _accept(payload, peer) {
      if (!this.hosting || payload.sessionId !== this.session?.id || !this._stream || !payload.mediaPort) return;
      if (!this.guests.some((g) => g.peer === peer)) {
        this.guests = [...this.guests, { peer, port: payload.mediaPort, nickname: String(payload.nickname || '房员').slice(0, 32) }];
      }
      this._ensureRecorder();
    }

    _ensureRecorder() {
      if (this._recorder || !this._stream || !this.guests.length) return;
      this._recorder = new TcpRecorder({
        stream: this._stream, session: this.session.id, kind: 'watch',
        mime: chooseMime(['video/webm;codecs=vp9,opus', 'video/webm;codecs=vp8,opus', 'video/webm']),
        videoBitsPerSecond: 12_000_000, timeslice: 250,
        targets: () => this.guests,
        onError: (error) => { this.error = `视频发送失败：${error?.message || error}`; },
      });
      this._recorder.start();
    }

    _tcpMedia(payload, peer) {
      if (this._receiver?.peer === peer) this._receiver.accept(payload);
    }

    _ended(payload, peer) {
      if (payload.sessionId !== this.session?.id || this._receiver?.peer !== peer) return;
      this.stop(false);
      this.error = t('watch_ended');
    }

    _closePeer(peer) {
      this.guests = this.guests.filter((g) => g.peer !== peer);
      if (!this.guests.length && this._recorder) {
        const recorder = this._recorder;
        this._recorder = null;
        recorder.stop();
      }
    }
  }

  export const watchService = new WatchState();
</script>
