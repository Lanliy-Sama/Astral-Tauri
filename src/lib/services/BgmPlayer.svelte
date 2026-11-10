<script module>
  /**
   * 背景音乐播放器单例 —— <audio> 元素由本模块持有，跨页面持续播放。
   * 音源：astralbgm:// 自定义协议（磁盘 bgm/<uuid>.<ext> 直读）。
   * 状态寄存于 sm.bgmState（enabled/volume/tracks/trackId/playing/
   * volumePulse/volumeDelta），本模块只负责播放控制与音量应用。
   */
  import { sm } from './ServiceManager.svelte';
  import { isTauri } from '../utils/Platform.svelte';

  let audioEl = null;
  let loadedTrackId = null; // audio.src 当前对应的曲目

  function ensureAudio() {
    if (audioEl) return audioEl;
    audioEl = new Audio();
    audioEl.preload = 'auto';
    // 播完自动切下一首（顺序循环）
    audioEl.addEventListener('ended', () => {
      const tracks = sm.bgmState.tracks;
      if (!tracks.length) {
        sm.bgmState.playing = false;
        return;
      }
      const cur = tracks.findIndex((t) => t.trackId === sm.bgmState.trackId);
      bgmPlayTrack((cur + 1) % tracks.length);
    });
    audioEl.addEventListener('pause', () => {
      sm.bgmState.playing = false;
    });
    audioEl.addEventListener('play', () => {
      sm.bgmState.playing = true;
    });
    // 音源加载失败 → 应用日志页可见（诊断 astralbgm 协议问题用）
    audioEl.addEventListener('error', () => {
      const err = audioEl?.error;
      sm.pushLog(
        `[bgm] 音源加载失败 code=${err?.code ?? '?'} ${err?.message ?? ''} src=${audioEl?.src ?? ''}`
      );
    });
    return audioEl;
  }

  async function trackUrl(trackId) {
    if (!isTauri()) return null; // 浏览器预览无后端文件
    try {
      const { convertFileSrc } = await import('@tauri-apps/api/core');
      return convertFileSrc(trackId, 'astralbgm');
    } catch (e) {
      console.warn('背景音乐 URL 生成失败:', e);
      return null;
    }
  }

  export function bgmCurrentIndex() {
    return sm.bgmState.tracks.findIndex((t) => t.trackId === sm.bgmState.trackId);
  }

  /** 播放/切换到指定曲目（歌单下标）。点击当前曲目请用 bgmToggle。 */
  export async function bgmPlayTrack(index) {
    const track = sm.bgmState.tracks[index];
    if (!track) return;
    sm.bgmState.trackId = track.trackId;
    sm.persistBgmTrackId(track.trackId).catch(() => {});
    const url = await trackUrl(track.trackId);
    if (!url) return;
    const a = ensureAudio();
    if (loadedTrackId !== track.trackId) {
      a.src = url;
      loadedTrackId = track.trackId;
    }
    a.volume = sm.bgmState.volume / 100;
    try {
      await a.play();
      sm.bgmState.playing = true;
    } catch (e) {
      sm.bgmState.playing = false;
      const name = e?.name ?? '?';
      const msg = e?.message ?? String(e);
      sm.pushLog(`[bgm] play() 失败 ${name}: ${msg}`);
      console.warn('背景音乐播放失败:', e);
    }
  }

  /** 播放/暂停切换：点击当前曲目、或无选中时从第一首开始。 */
  export function bgmToggle() {
    const tracks = sm.bgmState.tracks;
    if (!tracks.length) return;
    const a = ensureAudio();
    if (sm.bgmState.playing) {
      a.pause();
      return;
    }
    const cur = bgmCurrentIndex();
    if (sm.bgmState.trackId == null || cur < 0) {
      bgmPlayTrack(0);
      return;
    }
    if (loadedTrackId !== sm.bgmState.trackId) {
      bgmPlayTrack(cur);
      return;
    }
    a.volume = sm.bgmState.volume / 100;
    a.play().catch(() => {});
  }

  /** 暂停（关闭背景音乐开关时调用） */
  export function bgmPause() {
    if (audioEl && sm.bgmState.playing) audioEl.pause();
  }

  /** 应用音量到 <audio>（状态与持久化由 sm.updateBgmVolume 负责） */
  export function bgApplyVolume(v) {
    if (audioEl) audioEl.volume = Math.min(1, Math.max(0, v / 100));
  }

  /**
   * Ctrl+↑/↓ 快捷键调音量：更新状态 + 持久化 + 应用到播放器，
   * 并自增 volumePulse 触发主屏灵动岛音量胶囊。
   */
  export function bgmAdjustVolume(delta) {
    const next = Math.min(100, Math.max(0, sm.bgmState.volume + delta));
    sm.bgmState.volumeDelta = delta >= 0 ? 1 : -1;
    sm.bgmState.volumePulse += 1;
    if (next === sm.bgmState.volume) return; // 到顶/到底仍显示胶囊
    sm.updateBgmVolume(next).catch(() => {});
    bgApplyVolume(next);
  }
</script>
