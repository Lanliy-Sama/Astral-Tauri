<script module>
/** SMTC 服务（Windows 系统媒体会话）：轮询、控制、歌词抓取。
 * 轮询由 MainScreen 启动（1s/次，仅 tauri+Windows）；歌词按 标题|歌手
 * 缓存在内存（≤32 条），曲目切换时自动重抓。状态落在 sm.smtcState，
 * 灵动岛歌词胶囊（SmtcLyricCapsule）与软件设置 SMTC 卡片共同消费。 */
import { sm } from './ServiceManager.svelte';
import { isTauri, isWindows } from '../utils/Platform.svelte';

let started = false;
const lyricCache = new Map(); // 'title|artist' -> [{t,l}]

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
  const prevKey = s.available ? s.title + '|' + s.artist : '';
  s.available = info.hasSession === true;
  s.title = info.title || '';
  s.artist = info.artist || '';
  s.appId = info.appId || '';
  s.status = info.status || '';
  s.position = Number(info.positionSecs) || 0;
  s.duration = Number(info.durationSecs) || 0;
  s.rate = Number(info.rate) > 0 ? Number(info.rate) : 1;
  s.fetchedAt = performance.now();
  const key = s.available ? s.title + '|' + s.artist : '';
  if (key !== prevKey) {
    s.lyricLines = null;
    s.lyricFound = false;
    if (key) void loadLyrics(s.title, s.artist);
  }
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
