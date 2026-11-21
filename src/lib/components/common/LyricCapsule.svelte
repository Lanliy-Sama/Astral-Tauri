<script>
  /** 灵动岛歌词胶囊 —— 播放中的曲目（在线平台歌曲 / 本地背景音乐）
   *  顶部常驻显示当前歌词行 + 曲名歌手。
   *  歌词：在线曲目经 music_lyrics 按平台拉取（网易云含翻译合并）；
   *  进度：bgmPosition() = audio.currentTime，天然与播放同步（300ms 步进）。 */
  import { sm } from '../../services/ServiceManager.svelte';
  import { bgmPosition } from '../../services/BgmPlayer.svelte';
  import { isTauri } from '../../utils/Platform.svelte';

  let lineIdx = $state(-1);

  const active = $derived(
    sm.bgmState.lyricCapsuleEnabled && sm.bgmState.playing
  );
  const lines = $derived(sm.bgmState.lyricLines || []);
  const label = $derived.by(() => {
    const t = sm.bgmState.onlineTrack;
    if (t) return t.title + (t.artist ? ' · ' + t.artist : '');
    const local = sm.bgmState.tracks.find((x) => x.trackId === sm.bgmState.trackId);
    return local ? local.name : '';
  });

  /* 在线曲目：按平台拉取歌词（网易云带翻译合并，credit 行已在 Rust 侧过滤） */
  $effect(() => {
    const t = sm.bgmState.onlineTrack;
    if (!t) {
      sm.bgmState.lyricLines = null;
      sm.bgmState.lyricFound = false;
      return;
    }
    if (!isTauri()) return;
    let dead = false;
    (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke('music_lyrics', { platform: t.platform, key: t.key });
        if (dead) return;
        sm.bgmState.lyricLines = res?.lines?.length ? res.lines : null;
        sm.bgmState.lyricFound = !!res?.found;
      } catch {
        if (!dead) {
          sm.bgmState.lyricLines = null;
          sm.bgmState.lyricFound = false;
        }
      }
    })();
    return () => {
      dead = true;
    };
  });

  function tick() {
    if (!lines.length) {
      lineIdx = -1;
      return;
    }
    const pos = bgmPosition() * 1000; // 秒 → 毫秒（LRC 时间轴）
    let idx = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].t <= pos) idx = i;
      else break;
    }
    if (idx !== lineIdx) lineIdx = idx;
  }

  $effect(() => {
    void lines.length;
    tick();
  });

  $effect(() => {
    if (!active) return;
    const timer = setInterval(tick, 300);
    return () => clearInterval(timer);
  });

  const currentLine = $derived(lineIdx >= 0 && lineIdx < lines.length ? lines[lineIdx].l : '');
  const show = $derived(active && (label !== '' || currentLine !== ''));
</script>

{#if show}
  <div class="lyric-capsule">
    <span class="ms filled lyr-note">lyrics</span>
    <div class="lyr-body">
      {#if currentLine}
        <div class="lyr-line">{currentLine}</div>
        <div class="lyr-track">{label}</div>
      {:else}
        <div class="lyr-line">{label || '　'}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .lyric-capsule {
    position: fixed;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(680px, 92vw);
    padding: 10px 18px;
    border-radius: 999px;
    background: rgb(24 24 27 / 0.92);
    color: #ffffff;
    box-shadow:
      0 4px 14px rgb(0 0 0 / 0.28),
      inset 0 0 0 1px rgb(255 255 255 / 0.08);
    backdrop-filter: blur(12px) saturate(1.4);
    z-index: 3490; /* 音量胶囊(3500)弹出时覆盖歌词胶囊 */
    pointer-events: none;
  }
  .lyr-note {
    font-size: 20px;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }
  .lyr-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .lyr-line {
    font-size: 14px;
    font-weight: 600;
    line-height: 18px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lyr-track {
    font-size: 11px;
    line-height: 14px;
    color: rgb(255 255 255 / 0.62);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
