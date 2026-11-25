<script module>
  /** 一对一远程桌面：TCP 视频分片 + TCP 输入消息，被控端手动授权。
   * 分辨率保持采集源原生值；码率/帧率变化只重建 MediaRecorder。 */
  import { roomSocial } from './RoomSocial.svelte';
  import { sm } from './ServiceManager.svelte';
  import { tcpMediaPort, chooseMime, TcpRecorder, TcpMediaReceiver } from './TcpMedia.svelte';

  const FRAME_RATES = [30, 60, 120, 144, 165, 240];
  function id() { return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`; }
  function clampBitrate(value) { return Math.min(500, Math.max(1, Math.round(Number(value) || 8))); }
  function clampFrameRate(value) { const rate = Number(value); return FRAME_RATES.includes(rate) ? rate : 60; }

  class RemoteDesktopState {
    incoming = $state(null);
    outgoing = $state(null);
    session = $state(null);
    hosting = $state(false);
    joined = $state(false);
    starting = $state(false);
    remoteUrl = $state('');
    bitrate = $state(8);
    frameRate = $state(60);
    actualFrameRate = $state(0);
    resolution = $state('');
    error = $state('');

    _stream = null;
    _recorder = null;
    _receiver = null;
    _peer = '';
    _mediaPort = 0;
    _restartSerial = Promise.resolve();

    constructor() {
      roomSocial.on('desktop_request', (p, peer) => this._request(p, peer));
      roomSocial.on('desktop_decline', (p, peer) => this._declined(p, peer));
      roomSocial.on('desktop_accept', (p, peer) => this._accepted(p, peer));
      roomSocial.on('desktop_quality', (p, peer) => this._quality(p, peer));
      roomSocial.on('desktop_input', (p, peer) => this._input(p, peer));
      roomSocial.on('desktop_end', (p, peer) => this._ended(p, peer));
      roomSocial.on('tcp-media', (p, peer) => this._tcpMedia(p, peer));
      roomSocial.on('room-leave', () => this.stop(false));
    }

    async request(member) {
      if (this.outgoing || this.session) return;
      this.error = '';
      const outgoing = { requestId: id(), peer: member.peer, nickname: member.nickname };
      this.outgoing = outgoing;
      try {
        const mediaPort = await tcpMediaPort();
        await roomSocial._sendOne(member.peer, 'desktop_request', {
          requestId: outgoing.requestId, requester: sm.playerState.playerName || '房员', mediaPort,
        });
      } catch (e) {
        this.outgoing = null;
        this.error = String(e?.message || e);
      }
    }

    cancelRequest() {
      const outgoing = this.outgoing;
      this.outgoing = null;
      if (outgoing) roomSocial._sendOne(outgoing.peer, 'desktop_decline', { requestId: outgoing.requestId }).catch(() => {});
    }

    decline() {
      const incoming = this.incoming;
      this.incoming = null;
      if (incoming) roomSocial._sendOne(incoming.peer, 'desktop_decline', { requestId: incoming.requestId }).catch(() => {});
    }

    async accept() {
      const incoming = this.incoming;
      if (!incoming?.mediaPort || this.starting) return;
      this.starting = true;
      this.error = '';
      try {
        if (!navigator.mediaDevices?.getDisplayMedia) throw new Error('当前平台不支持桌面采集');
        this._stream = await navigator.mediaDevices.getDisplayMedia({
          video: { frameRate: { ideal: this.frameRate, max: this.frameRate } }, audio: true,
        });
        this._peer = incoming.peer;
        this._mediaPort = incoming.mediaPort;
        this.session = {
          id: id(), host: sm.playerState.playerName || '被控端',
          controller: incoming.requester, resolution: '',
        };
        this.hosting = true;
        this.joined = false;
        this.incoming = null;
        await this._applyTrackRate();
        const track = this._stream.getVideoTracks()[0];
        const settings = track?.getSettings() || {};
        this.resolution = settings.width && settings.height ? `${settings.width} × ${settings.height}` : '采集源原生分辨率';
        this.session.resolution = this.resolution;
        track?.addEventListener('ended', () => this.stop());
        await roomSocial._sendOne(this._peer, 'desktop_accept', {
          requestId: incoming.requestId, session: this.session,
          bitrate: this.bitrate, frameRate: this.frameRate,
        });
        this._startRecorder();
      } catch (e) {
        this.error = String(e?.message || e);
        this._stopMedia();
        this.hosting = false;
        this.session = null;
        this._peer = '';
      } finally {
        this.starting = false;
      }
    }

    _request(payload, peer) {
      if (this.session || this.incoming || !payload.requestId || !payload.mediaPort) return;
      this.incoming = {
        peer, requestId: payload.requestId, mediaPort: payload.mediaPort,
        requester: String(payload.requester || '房员').slice(0, 32),
      };
    }

    _declined(payload, peer) {
      if (this.outgoing?.peer === peer && this.outgoing.requestId === payload.requestId) {
        this.outgoing = null;
        this.error = '对方拒绝了远程桌面请求';
      }
    }

    _accepted(payload, peer) {
      if (this.outgoing?.peer !== peer || this.outgoing.requestId !== payload.requestId || !payload.session) return;
      this.outgoing = null;
      this.session = payload.session;
      this.joined = true;
      this.hosting = false;
      this._peer = peer;
      this.resolution = payload.session.resolution || '采集源原生分辨率';
      this.bitrate = clampBitrate(payload.bitrate);
      this.frameRate = clampFrameRate(payload.frameRate);
      this._receiver?.dispose();
      this._receiver = new TcpMediaReceiver({
        kind: 'desktop', session: this.session.id, peer,
        onUrl: (url) => { this.remoteUrl = url; },
        onError: (error) => { this.error = `桌面接收失败：${error?.message || error}`; },
        onEnded: () => this.stop(false),
      });
    }

    _startRecorder() {
      if (!this._stream || !this._peer || !this._mediaPort || !this.session) return;
      const recorder = new TcpRecorder({
        stream: this._stream, session: this.session.id, kind: 'desktop',
        mime: chooseMime(['video/webm;codecs=vp9,opus', 'video/webm;codecs=vp8,opus', 'video/webm']),
        videoBitsPerSecond: this.bitrate * 1_000_000,
        audioBitsPerSecond: 128000, timeslice: 250,
        targets: () => [{ peer: this._peer, port: this._mediaPort }],
        onError: (error) => { this.error = `桌面发送失败：${error?.message || error}`; },
      });
      this._recorder = recorder;
      recorder.start();
    }

    _restartRecorder() {
      this._restartSerial = this._restartSerial.then(async () => {
        const previous = this._recorder;
        this._recorder = null;
        if (previous) await previous.stop();
        if (this.hosting && this.session) this._startRecorder();
      }).catch((error) => { this.error = String(error?.message || error); });
      return this._restartSerial;
    }

    async setBitrate(value) {
      this.bitrate = clampBitrate(value);
      if (this.hosting) await this._restartRecorder();
      await this._sendQuality();
    }

    async setFrameRate(value) {
      this.frameRate = clampFrameRate(value);
      if (this.hosting) {
        await this._applyTrackRate();
        await this._restartRecorder();
      }
      await this._sendQuality();
    }

    async _sendQuality() {
      if (!this._peer || !this.session) return;
      await roomSocial._sendOne(this._peer, 'desktop_quality', {
        sessionId: this.session.id, bitrate: this.bitrate, frameRate: this.frameRate,
      }).catch(() => {});
    }

    async _quality(payload, peer) {
      if (peer !== this._peer || payload.sessionId !== this.session?.id) return;
      const oldBitrate = this.bitrate;
      const oldRate = this.frameRate;
      this.bitrate = clampBitrate(payload.bitrate);
      this.frameRate = clampFrameRate(payload.frameRate);
      if (this.hosting && (oldBitrate !== this.bitrate || oldRate !== this.frameRate)) {
        await this._applyTrackRate();
        await this._restartRecorder();
      }
    }

    async _applyTrackRate() {
      const track = this._stream?.getVideoTracks()[0];
      if (!track) return;
      try {
        await track.applyConstraints({ frameRate: { ideal: this.frameRate, max: this.frameRate } });
        this.actualFrameRate = Math.round(track.getSettings().frameRate || 0);
      } catch {
        this.actualFrameRate = Math.round(track.getSettings().frameRate || 0);
        this.error = `采集端无法达到 ${this.frameRate} FPS，实际为 ${this.actualFrameRate || '未知'} FPS`;
      }
    }

    _tcpMedia(payload, peer) {
      if (this._receiver?.peer === peer) this._receiver.accept(payload);
    }

    sendPointer(kind, x = 0, y = 0, button = 'left') { this._sendInput({ kind, x, y, button }); }
    sendWheel(delta) { this._sendInput({ kind: 'wheel', delta }); }
    sendKey(down, vk) { this._sendInput({ kind: down ? 'keyDown' : 'keyUp', vk }); }

    _sendInput(event) {
      if (!this.joined || this.hosting || !this._peer || !this.session) return;
      roomSocial._sendOne(this._peer, 'desktop_input', { sessionId: this.session.id, event }).catch(() => {});
    }

    async _input(payload, peer) {
      if (!this.hosting || peer !== this._peer || payload.sessionId !== this.session?.id) return;
      const event = payload.event;
      if (!event || !['move', 'down', 'up', 'wheel', 'keyDown', 'keyUp'].includes(event.kind)) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('remote_desktop_input', { event });
      } catch (e) {
        this.error = String(e?.message || e);
      }
    }

    async stop(notify = true) {
      if (notify && this._peer && this.session) {
        await roomSocial._sendOne(this._peer, 'desktop_end', { sessionId: this.session.id }).catch(() => {});
      }
      const recorder = this._recorder;
      this._recorder = null;
      recorder?.stop();
      this._receiver?.dispose();
      this._receiver = null;
      this._stopMedia();
      this.session = null;
      this.incoming = null;
      this.outgoing = null;
      this.hosting = false;
      this.joined = false;
      this.starting = false;
      this.remoteUrl = '';
      this._peer = '';
      this._mediaPort = 0;
      this.resolution = '';
      this.actualFrameRate = 0;
    }

    _stopMedia() {
      if (this._stream) for (const track of this._stream.getTracks()) track.stop();
      this._stream = null;
    }

    _ended(payload, peer) {
      if (peer !== this._peer || payload.sessionId !== this.session?.id) return;
      this.stop(false);
      this.error = '远程桌面会话已结束';
    }
  }

  export const desktopService = new RemoteDesktopState();
</script>
