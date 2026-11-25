<script module>
  /** MediaRecorder → TCP 分片 → MediaSource 的通用工具。 */
  export async function tcpMediaPort() {
    const { invoke } = await import('@tauri-apps/api/core');
    const result = await invoke('room_media_listener_start');
    return result.port;
  }

  export function chooseMime(candidates) {
    return candidates.find((mime) => globalThis.MediaRecorder?.isTypeSupported?.(mime)) || candidates[0];
  }

  function blobBase64(blob) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).split(',')[1] || '');
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(blob);
    });
  }

  async function sendFrame(target, session, kind, codec, sequence, data) {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('room_media_send', {
      peer: target.peer, port: target.port, session, kind, codec, sequence, data,
    });
  }

  export class TcpRecorder {
    constructor({ stream, session, kind, mime, videoBitsPerSecond, audioBitsPerSecond = 96000, timeslice = 250, targets, onError = () => {} }) {
      this.stream = stream;
      this.session = session;
      this.kind = kind;
      this.mime = mime;
      this.videoBitsPerSecond = videoBitsPerSecond;
      this.audioBitsPerSecond = audioBitsPerSecond;
      this.timeslice = timeslice;
      this.targets = targets;
      this.onError = onError;
      this.sequence = 0;
      this.recorder = null;
      this.queue = Promise.resolve();
      this.stopped = false;
    }

    start() {
      const options = {
        mimeType: this.mime,
        audioBitsPerSecond: this.audioBitsPerSecond,
        ...(this.videoBitsPerSecond ? { videoBitsPerSecond: this.videoBitsPerSecond } : {}),
      };
      this.recorder = new MediaRecorder(this.stream, options);
      this.recorder.ondataavailable = (event) => {
        if (!event.data?.size) return;
        const sequence = this.sequence++;
        const blob = event.data;
        this.queue = this.queue.then(async () => {
          const data = await blobBase64(blob);
          await Promise.allSettled(this.targets().map((target) => sendFrame(target, this.session, this.kind, this.mime, sequence, data)));
        }).catch(this.onError);
      };
      this.recorder.onerror = (event) => this.onError(event.error || new Error('媒体编码失败'));
      this.recorder.onstop = () => {
        this.queue = this.queue.then(async () => {
          const sequence = this.sequence++;
          await Promise.allSettled(this.targets().map((target) => sendFrame(target, this.session, this.kind, this.mime, sequence, '')));
        }).catch(() => {});
      };
      this.recorder.start(this.timeslice);
    }

    async stop() {
      if (this.stopped) return;
      this.stopped = true;
      if (this.recorder && this.recorder.state !== 'inactive') this.recorder.stop();
      await this.queue.catch(() => {});
    }
  }

  function decodeBase64(data) {
    const binary = atob(data);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  }

  export class TcpMediaReceiver {
    constructor({ kind, session, peer, element = null, onUrl = () => {}, onError = () => {}, onEnded = () => {} }) {
      this.kind = kind;
      this.session = session;
      this.peer = peer;
      this.element = element;
      this.onUrl = onUrl;
      this.onError = onError;
      this.onEnded = onEnded;
      this.expected = 0;
      this.started = false;
      this.queue = [];
      this.source = null;
      this.buffer = null;
      this.url = '';
      this.disposed = false;
    }

    matches(payload) {
      return !this.disposed && payload.peer === this.peer && payload.kind === this.kind && payload.session === this.session;
    }

    accept(payload) {
      if (!this.matches(payload)) return false;
      if (payload.end) {
        this.onEnded();
        return true;
      }
      if (payload.sequence === 0 || !this.started) {
        this._resetSource(payload.codec);
        this.started = true;
      }
      if (payload.sequence < this.expected && payload.sequence !== 0) return true;
      this.expected = payload.sequence + 1;
      if (this.queue.length > 32) this.queue.splice(0, this.queue.length - 24);
      this.queue.push(decodeBase64(payload.data));
      this._append();
      return true;
    }

    _resetSource(codec = '') {
      if (this.url) URL.revokeObjectURL(this.url);
      this.queue = [];
      this.expected = 0;
      this.buffer = null;
      this.source = new MediaSource();
      this.url = URL.createObjectURL(this.source);
      this.onUrl(this.url);
      if (this.element) {
        this.element.src = this.url;
        this.element.play?.().catch(() => {});
      }
      this.source.addEventListener('sourceopen', () => {
        if (this.disposed) return;
        try {
          const selected = codec || 'video/webm;codecs=vp8,opus';
          this.buffer = this.source.addSourceBuffer(selected);
          this.buffer.mode = 'sequence';
          this.buffer.addEventListener('updateend', () => this._append());
          this._append();
        } catch (error) {
          this.onError(error);
        }
      }, { once: true });
    }

    _append() {
      if (!this.buffer || this.buffer.updating || !this.queue.length || this.source?.readyState !== 'open') return;
      try {
        this.buffer.appendBuffer(this.queue.shift());
      } catch (error) {
        this.onError(error);
      }
    }

    dispose() {
      this.disposed = true;
      this.queue = [];
      if (this.element) {
        this.element.pause?.();
        this.element.removeAttribute('src');
        this.element.load?.();
      }
      if (this.url) URL.revokeObjectURL(this.url);
      this.url = '';
      this.onUrl('');
      this.source = null;
      this.buffer = null;
    }
  }
</script>
