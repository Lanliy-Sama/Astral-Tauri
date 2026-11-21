<script>
  /** 灵动岛歌词胶囊 —— 开关（软件设置 → 系统音乐控制）启用后常驻顶部，
   *  实时显示 SMTC 系统播放（第三方播放器）的当前歌词行 + 歌名歌手。
   *  进度插值：smtcState.position + rate * 自取数起经过时间，300ms 步进。 */
  import { sm } from '../../services/ServiceManager.svelte';
  import { isTauri, isWindows } from '../../utils/Platform.svelte';

  let lineIdx = $state(-1);

  const active = $derived(
    isTauri() &&
    isWindows() &&
    sm.appSettingsState.smtcLyricEnabled &&
    sm.smtcState.available
  );
  const lines = $derived(sm.smtcState.lyricLines || []);
  const trackLabel = $derived(
    sm.smtcState.title + (sm.smtcState.artist ? ' · ' + sm.smtcState.artist : '')
  );

  function tick() {
    const s = sm.smtcState;
    if (!s.available || !lines.length) {
      lineIdx = -1;
      return;
    }
    const pos = s.position + ((performance.now() - s.fetchedAt) / 1000) * s.rate;
    let idx = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].t <= pos) idx = i;
      else break;
    }
    if (idx !== lineIdx) lineIdx = idx;
  }

  $effect(() => {
    void lines.length;
    void sm.smtcState.fetchedAt;
    tick();
  });

  $effect(() => {
    if (!active) return;
    const timer = setInterval(tick, 300);
    return () => clearInterval(timer);
  });

  const currentLine = $derived(lineIdx >= 0 && lineIdx < lines.length ? lines[lineIdx].l : '');
</script>

{#if active}
  <div class="lyric-capsule">
    <span class="ms filled lyr-note">lyrics</span>
    <div class="lyr-body">
      {#if currentLine}
        <div class="lyr-line">{currentLine}</div>
        <div class="lyr-track">{trackLabel}</div>
      {:else}
        <div class="lyr-line">{trackLabel || '　'}</div>
        {#if sm.smtcState.lyricFound === false && sm.smtcState.lyricLines !== null}
          <div class="lyr-track">暂无匹配歌词</div>
        {/if}
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
