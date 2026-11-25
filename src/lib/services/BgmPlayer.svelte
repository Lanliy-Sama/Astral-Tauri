<script module>
  /** 背景音乐播放器单例 —— <audio> 元素由本模块持有，跨页面持续播放。
   * 两个音源：本地歌单（astralbgm:// 自定义协议）+ 在线音乐平台
   * （网易云 / QQ / 酷狗，music_play_url 解析直链）。歌词胶囊从
   * bgmPosition() 取真实播放进度（audio.currentTime，天然同步）。 */
  import { sm } from './ServiceManager.svelte';
  import { isTauri } from '../utils/Platform.svelte';

  let audioEl = null;
  let loadedTrackId = null; // audio.src 当前对应的曲目
  let loadedOnlineUrl = null; // audio.src 当前对应的在线直链
  let onlineRequestId = 0; // 丢弃快速切歌时已过期的高延迟平台响应
  let smtcTimer = 0;
  let smtcLastSent = 0;
  let smtcSignature = '';
  let smtcListening = false;

  function handleSmtcEvent(payload) {
    const a = ensureAudio();
    const kind = payload?.kind;
    if (kind === 'play' && !sm.bgmState.playing) bgmToggleAny();
    else if (kind === 'pause' && sm.bgmState.playing) bgmToggleAny();
    else if (kind === 'toggle') bgmToggleAny();
    else if (kind === 'next') bgmNext();
    else if (kind === 'previous') bgmPrevious();
    else if (kind === 'stop') {
      a.pause();
      a.currentTime = 0;
    } else if (kind === 'position' && Number.isFinite(payload.positionMs)) {
      a.currentTime = Math.max(0, payload.positionMs / 1000);
    } else if (kind === 'volume' && Number.isFinite(payload.volume)) {
      const volume = Math.round(Math.min(1, Math.max(0, payload.volume)) * 100);
      sm.updateBgmVolume(volume).catch(() => {});
      bgApplyVolume(volume);
    }
  }

  function initSmtcListener() {
    if (smtcListening || !isTauri()) return;
    smtcListening = true;
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen('smtc-event', (event) => handleSmtcEvent(event.payload)))
      .catch((e) => console.warn('SMTC 事件监听失败:', e));
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen('smtc-error', (event) => sm.pushLog(`[SMTC] ${event.payload}`)))
      .catch(() => {});
  }

  /** 同步系统媒体菜单；第三方壁纸/媒体工具从操作系统读取同一份信息。 */
  export function syncBgmSmtc() {
    if (!isTauri()) return;
    clearTimeout(smtcTimer);
    smtcTimer = setTimeout(async () => {
      const online = sm.bgmState.onlineTrack;
      const local = sm.bgmState.tracks.find((t) => t.trackId === sm.bgmState.trackId);
      const enabled = sm.bgmState.smtcEnabled;
      const playing = sm.bgmState.playing;
      const volume = sm.bgmState.volume;
      const title = online?.title || local?.name || '';
      const signature = `${enabled}|${playing}|${volume}|${online?.key || local?.trackId || ''}`;
      const now = Date.now();
      if (signature === smtcSignature && now - smtcLastSent < 1000) return;
      smtcSignature = signature;
      smtcLastSent = now;
      const a = audioEl;
      const durationMs = Number.isFinite(a?.duration) ? Math.round(a.duration * 1000) : online?.durationMs || 0;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('smtc_update', {
          enabled,
          title,
          artist: online?.artists?.join(' / ') || (local ? '本地音乐' : ''),
          album: online ? online.sharedAccountName || online.platform : local ? '本地音乐' : '',
          cover: online?.cover || '',
          durationMs,
          positionMs: Math.max(0, Math.round((a?.currentTime || 0) * 1000)),
          playing,
          volume: volume / 100,
        });
      } catch (e) {
        console.warn('SMTC 同步失败:', e);
      }
    }, 60);
  }

  function ensureAudio() {
    if (audioEl) return audioEl;
    audioEl = new Audio();
    audioEl.preload = 'auto';
    initSmtcListener();
    for (const event of ['timeupdate', 'durationchange', 'loadedmetadata', 'volumechange', 'seeked']) {
      audioEl.addEventListener(event, syncBgmSmtc);
    }
    // 播完自动切下一首（顺序循环：本地歌单 / 在线队列）
    audioEl.addEventListener('ended', () => {
      if (sm.bgmState.onlineTrack) {
        bgmPlayOnlineNext();
        return;
      }
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
      syncBgmSmtc();
    });
    audioEl.addEventListener('play', () => {
      sm.bgmState.playing = true;
      syncBgmSmtc();
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
    onlineRequestId += 1;
    sm.bgmState.onlineTrack = null;
    sm.bgmState.lyricLines = null;
    sm.bgmState.lyricFound = false;
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
      loadedOnlineUrl = null;
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

  /* ================= 在线音乐（平台直连） ================= */

  /** 真实播放进度（秒）：歌词胶囊的数据源，audio.currentTime 天然同步 */
  export function bgmPosition() {
    return audioEl ? audioEl.currentTime : 0;
  }

  /** 播放在线曲目：music_play_url 解析平台直链 → <audio> 直接播放 */
  export async function bgmPlayOnlineTrack(track) {
    if (!isTauri()) throw new Error('浏览器预览无法调用平台播放接口');
    const requestId = ++onlineRequestId;
    const a = ensureAudio();
    a.pause();
    if (track.sourceAccount && !track.sharedStream) {
      const { roomSocial } = await import('./RoomSocial.svelte');
      track = await roomSocial.resolveSharedTrack(track);
      if (requestId !== onlineRequestId) return;
      const queueIndex = sm.bgmState.onlineQueue.findIndex((t) => t.key === track.key);
      if (queueIndex >= 0) sm.bgmState.onlineQueue[queueIndex] = track;
    }
    let url = track.sharedStream || '';
    if (!url) {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke('music_play_url', {
        platform: track.platform,
        key: track.key,
        track,
      });
      url = res?.url || '';
    }
    if (requestId !== onlineRequestId) return;
    if (!url) throw new Error('没有可用的播放地址');
    sm.bgmState.onlineTrack = track;
    sm.bgmState.trackId = null;
    sm.bgmState.lyricLines = null;
    sm.bgmState.lyricFound = false;
    if (loadedOnlineUrl !== url) {
      a.src = url;
      loadedOnlineUrl = url;
      loadedTrackId = null;
      a.load();
    }
    a.volume = sm.bgmState.volume / 100;
    try {
      await a.play();
      sm.bgmState.playing = true;
    } catch (e) {
      sm.bgmState.playing = false;
      sm.pushLog(`[bgm] 在线播放失败: ${e?.message ?? e}`);
      // 调用方需要显示可见错误；旧实现吞掉异常，点击后看起来毫无反应。
      throw e;
    }
  }

  /** 在线队列下一首（顺序循环） */
  export async function bgmPlayOnlineNext() {
    const queue = sm.bgmState.onlineQueue;
    if (!queue.length) {
      sm.bgmState.playing = false;
      return;
    }
    const next = (sm.bgmState.onlineIndex + 1) % queue.length;
    sm.bgmState.onlineIndex = next;
    await bgmPlayOnlineTrack(queue[next]);
  }

  /** 在线播放/暂停切换 */
  export function bgmOnlineToggle() {
    const a = ensureAudio();
    if (sm.bgmState.playing) {
      a.pause();
      return;
    }
    if (loadedOnlineUrl) {
      a.volume = sm.bgmState.volume / 100;
      a.play().catch(() => {});
    } else if (sm.bgmState.onlineQueue.length) {
      const idx = sm.bgmState.onlineIndex >= 0 ? sm.bgmState.onlineIndex : 0;
      sm.bgmState.onlineIndex = idx;
      bgmPlayOnlineTrack(sm.bgmState.onlineQueue[idx]);
    }
  }

  /** 灵动岛统一控制：在线队列与本地歌单共用上一首/暂停/下一首。 */
  export function bgmToggleAny() {
    if (sm.bgmState.onlineTrack) bgmOnlineToggle();
    else bgmToggle();
  }

  export async function bgmNext() {
    if (sm.bgmState.onlineTrack) {
      await bgmPlayOnlineNext();
      return;
    }
    const tracks = sm.bgmState.tracks;
    if (!tracks.length) return;
    await bgmPlayTrack((bgmCurrentIndex() + 1 + tracks.length) % tracks.length);
  }

  export async function bgmPrevious() {
    if (audioEl && audioEl.currentTime > 4) {
      audioEl.currentTime = 0;
      return;
    }
    if (sm.bgmState.onlineTrack) {
      const queue = sm.bgmState.onlineQueue;
      if (!queue.length) return;
      const previous = (sm.bgmState.onlineIndex - 1 + queue.length) % queue.length;
      sm.bgmState.onlineIndex = previous;
      await bgmPlayOnlineTrack(queue[previous]);
      return;
    }
    const tracks = sm.bgmState.tracks;
    if (!tracks.length) return;
    await bgmPlayTrack((bgmCurrentIndex() - 1 + tracks.length) % tracks.length);
  }

  /** 关闭在线播放（切回本地歌单模式时清状态） */
  export function bgmStopOnline() {
    sm.bgmState.onlineTrack = null;
    sm.bgmState.lyricLines = null;
    sm.bgmState.lyricFound = false;
    sm.bgmState.playing = false;
    if (audioEl) audioEl.pause();
  }
</script>
