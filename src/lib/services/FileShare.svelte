<script module>
  /** 房间文件分享：元数据广播，下载时按需建立 TCP 文件流。 */
  import { roomSocial } from './RoomSocial.svelte';
  import { sm } from './ServiceManager.svelte';

  function id() { return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`; }
  function blobBase64(blob) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).split(',')[1] || '');
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(blob);
    });
  }

  class FileShareState {
    files = $state([]);
    downloads = $state({});
    sharing = $state(false);
    error = $state('');
    _pending = new Map();

    constructor() {
      roomSocial.on('file_metadata', (payload, peer) => this._metadata(payload, peer));
      roomSocial.on('file_download_request', (payload, peer) => this._offer(payload, peer));
      roomSocial.on('file_download_response', (payload, peer) => this._response(payload, peer));
      roomSocial.on('room-leave', () => {
        this.files = [];
        this.downloads = {};
        for (const pending of this._pending.values()) {
          clearTimeout(pending.timer);
          pending.reject(new Error('已离开房间'));
        }
        this._pending.clear();
      });
    }

    async shareFiles(fileList) {
      if (!roomSocial.connected) throw new Error('请先连接房间');
      const files = Array.from(fileList || []);
      if (!files.length || this.sharing) return;
      this.sharing = true;
      this.error = '';
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        for (const file of files) {
          const id = await invoke('room_file_write_begin', {
            name: file.name, size: file.size, mime: file.type || 'application/octet-stream',
          });
          for (let offset = 0; offset < file.size; offset += 1 << 20) {
            const data = await blobBase64(file.slice(offset, Math.min(offset + (1 << 20), file.size)));
            await invoke('room_file_write_append', { id, b64: data });
          }
          const metadata = await invoke('room_file_write_end', { id });
          const entry = {
            ...metadata,
            ownerPeer: roomSocial.localIp,
            ownerName: sm.playerState.playerName || '房员',
            sharedAt: Date.now(),
          };
          this.files = [entry, ...this.files.filter((item) => item.id !== entry.id)];
          await roomSocial.broadcast('file_metadata', { file: entry });
        }
      } catch (error) {
        this.error = String(error?.message || error);
        throw error;
      } finally {
        this.sharing = false;
      }
    }

    _metadata(payload, peer) {
      const file = payload.file;
      if (!file?.id || !file.name || !Number.isFinite(file.size)) return;
      const entry = { ...file, ownerPeer: peer };
      this.files = [entry, ...this.files.filter((item) => item.id !== entry.id || item.ownerPeer !== peer)];
    }

    async download(file) {
      if (this.downloads[file.id]?.status === 'downloading') return;
      this.downloads = { ...this.downloads, [file.id]: { status: 'downloading' } };
      const requestId = id();
      try {
        const response = await new Promise((resolve, reject) => {
          const timer = setTimeout(() => {
            this._pending.delete(requestId);
            reject(new Error('分享端响应超时'));
          }, 30000);
          this._pending.set(requestId, { file, resolve, reject, timer });
          roomSocial._sendOne(file.ownerPeer, 'file_download_request', {
            requestId, fileId: file.id,
          }).catch(reject);
        });
        const { invoke } = await import('@tauri-apps/api/core');
        const saved = await invoke('room_file_download', {
          peer: file.ownerPeer,
          streamPort: response.streamPort,
          streamId: response.streamId,
          fileName: file.name,
        });
        this.downloads = { ...this.downloads, [file.id]: { status: 'done', path: saved.path, size: saved.size } };
      } catch (error) {
        this.downloads = { ...this.downloads, [file.id]: { status: 'error', error: String(error?.message || error) } };
      }
    }

    async _offer(payload, peer) {
      const base = { requestId: payload.requestId };
      const file = this.files.find((item) => item.id === payload.fileId && item.ownerPeer === roomSocial.localIp);
      try {
        if (!file) throw new Error('文件不存在或已取消分享');
        const { invoke } = await import('@tauri-apps/api/core');
        const offer = await invoke('room_file_offer', { fileId: file.id });
        await roomSocial._sendOne(peer, 'file_download_response', { ...base, ...offer });
      } catch (error) {
        await roomSocial._sendOne(peer, 'file_download_response', { ...base, error: String(error?.message || error) }).catch(() => {});
      }
    }

    _response(payload, peer) {
      const pending = this._pending.get(payload.requestId);
      if (!pending || pending.file.ownerPeer !== peer) return;
      this._pending.delete(payload.requestId);
      clearTimeout(pending.timer);
      if (payload.error) pending.reject(new Error(payload.error));
      else pending.resolve(payload);
    }
  }

  export const fileShare = new FileShareState();
</script>
