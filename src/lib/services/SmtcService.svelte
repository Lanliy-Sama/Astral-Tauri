<script module>
/** SMTC 服务（Windows 系统媒体会话）：轮询、控制、歌词抓取。
 * 轮询由 MainScreen 启动（1s/次，仅 tauri+Windows）；歌词按 标题|歌手
 * 缓存在内存（≤32 条），曲目切换时自动重抓。状态落在 sm.smtcState，
 * 灵动岛歌词胶囊（SmtcLyricCapsule）与软件设置 SMTC 卡片共同消费。
 *
 * 进度时钟：部分播放器（网易云桌面版）不连续更新 SMTC Position ——
 * 状态为 playing 且两次轮询间 reported 未推进时，从上次基线本地外推
 * （reported + elapsed*rate），歌词不卡死；reported 推进（自然更新/
 * seek/切歌）或 播放↔暂停 切换或 变速 即重置基线。 */
import { sm } from './ServiceManager.svelte';
import { isTauri, isWindows } from '../utils/Platform.svelte';

let started = false;
const lyricCache = new Map(); // 'title|artist' -> [{t,l}]
let baseline = { reported: 0, at: 0, rate: 1, playing: false };

export function smtcStartPolling() {
  if (started || !isTauri() || !isWindows()) return;
  started = true;
  sm.smtcState.supported = true;
  setInterval(() => {
    smtcRefresh().catch(() => {});
  }, 1000);
}

export async function smtcRefresh() {
  if (!started) return;
  const { invoke } = await import('@tauri-apps/api/core');
  const info = await invoke('smtc_get_current');
  const s = sm.smtcState;
  const now = performance.now();
  const prevKey = s.available ? s.title + '|' + s.artist : '';
  s.available = info.hasSession === true;
  s.title = info.title || '';
  s.artist = info.artist || '';
  s.status = info.status || '';
  s.duration = Number(info.durationSecs) || 0;
  s.rate = Number(info.rate) > 0 ? Number(info.rate) : 1;
  const reported = Number(info.positionSecs) || 0;
  const playing = s.status === 'playing';
  const trackChanged = (s.available ? s.title + '|' + s.artist : '') !== prevKey;
  const statusChanged = baseline.playing !== playing;

  /* 基线：reported 推进了 / 切歌 / 播放↔暂停 / 变速 → 重置；
   * playing 且 reported 未推进 → 保留基线继续外推（SMTC 位置停更兜底） */
  const advanced = Math.abs(reported - baseline.reported) >= 0.05;
  if (trackChanged || advanced || statusChanged || !playing || baseline.at === 0) {
    baseline = { reported, at: now, rate: s.rate, playing };
  }
  s.position = reported;
  s.fetchedAt = now;

  const key = s.available ? s.title + '|' + s.artist : '';
  if (key !== prevKey) {
    s.lyricLines = null;
    s.lyricFound = false;
    if (key) void loadLyrics(s.title, s.artist);
  }
}

/** 虚拟播放位置（秒）：SMTC Position 停更时从基线外推；暂停冻结 */
export function smtcVirtualPosition() {
  const s = sm.smtcState;
  if (!s.available) return 0;
  if (s.status !== 'playing') return baseline.reported;
  const elapsed = (performance.now() - baseline.at) / 1000;
  const v = baseline.reported + elapsed * s.rate;
  return s.duration > 0 ? Math.min(v, s.duration) : v;
}

async function loadLyrics(title, artist) {
  const key = title + '|' + artist;
  if (lyricCache.has(key)) {
    const lines = lyricCache.get(key) || [];
    sm.smtcState.lyricLines = lines;
    sm.smtcState.lyricFound = lines.length > 0;
    return;
  }
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const res = await invoke('smtc_fetch_lyrics', { title, artist });
    const lines = Array.isArray(res?.lines) ? res.lines : [];
    if (lyricCache.size > 32) lyricCache.clear();
    lyricCache.set(key, lines);
    const s = sm.smtcState;
    if (s.available && s.title + '|' + s.artist === key) {
      s.lyricLines = lines;
      s.lyricFound = lines.length > 0;
    }
  } catch {
    /* 歌词抓取失败（无匹配/无网络）静默 */
  }
}

export async function smtcControl(action) {
  if (!isTauri() || !isWindows()) return;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('smtc_control', { action });
  } catch (e) {
    console.warn('SMTC 控制失败:', e);
  } finally {
    await smtcRefresh().catch(() => {});
  }
}
</script>
