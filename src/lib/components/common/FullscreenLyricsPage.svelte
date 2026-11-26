<script>
  /** 全屏歌词：旋转胶片 + 进度滚动歌词。 */
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { sm } from '../../services/ServiceManager.svelte';
  import { bgmPosition, bgmPrevious, bgmNext, bgmToggleAny, bgmSeek } from '../../services/BgmPlayer.svelte';
  import { popPage } from '../../router/PageRouter.svelte';
  import { t } from '../../i18n/I18n.svelte';

  let lineIdx = $state(-1);
  let lyricsScrollEl = null;
  const online = $derived(sm.bgmState.onlineTrack);
  const local = $derived(sm.bgmState.tracks.find((track) => track.trackId === sm.bgmState.trackId));
  const title = $derived(online?.title || local?.name || t('no_music_playing'));
  const artist = $derived(online?.artists?.join(' / ') || (local ? t('local_music') : t('select_song_to_play')));
  const cover = $derived(online?.cover || '');
  const modeLabel = $derived(sm.bgmState.playbackMode === 'random' ? t('mode_random') : sm.bgmState.playbackMode === 'loop' ? t('mode_loop') : t('mode_sequence'));
  const modeIcon = $derived(sm.bgmState.playbackMode === 'random' ? 'shuffle' : sm.bgmState.playbackMode === 'loop' ? 'repeat_one' : 'repeat');

  function cyclePlaybackMode() {
    const next = sm.bgmState.playbackMode === 'sequence' ? 'random' : sm.bgmState.playbackMode === 'random' ? 'loop' : 'sequence';
    sm.updatePlaybackMode(next);
  }
  const lines = $derived(sm.bgmState.lyricLines || []);

  function updateLine() {
    if (!lines.length) { lineIdx = -1; return; }
    const position = bgmPosition() * 1000;
    let low = 0;
    let high = lines.length;
    while (low < high) {
      const mid = (low + high) >> 1;
      if (lines[mid].t <= position) low = mid + 1;
      else high = mid;
    }
    lineIdx = low - 1;
  }

  onMount(() => {
    sm.bgmState.lyricsFullScreen = true;
    const timer = setInterval(updateLine, 200);
    updateLine();
    return () => {
      clearInterval(timer);
      sm.bgmState.lyricsFullScreen = false;
    };
  });

  $effect(() => {
    const index = lineIdx;
    if (index >= 0) {
      lyricsScrollEl?.querySelector('.lyric-row.active')?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    }
  });
</script>

<div class="lyrics-page">
  <header class="lyrics-header">
    <button class="md-icon-btn" aria-label={t('back')} onclick={popPage}><Icon name="arrow_back" /></button>
    <div class="header-copy">
      <div class="header-title">{title}</div>
      <div class="header-artist">{artist}</div>
    </div>
    <span class="playing-state">{sm.bgmState.playing ? t('playing') : t('paused')}</span>
  </header>

  <main class="lyrics-main">
    <section class="vinyl-section">
      <div class="vinyl" class:spinning={sm.bgmState.playing}>
        <div class="vinyl-grooves"></div>
        <div class="vinyl-cover">
          {#if cover}
            <img src={cover} alt={`${title}封面`} />
          {:else}
            <Icon name="music_note" size={72} color="var(--md-primary)" />
          {/if}
        </div>
        <div class="vinyl-hole"></div>
      </div>
      <div class="track-copy">
        <div class="track-title">{title}</div>
        <div class="track-artist">{artist}</div>
      </div>
      <div class="fullscreen-controls">
        <button class="md-icon-btn" aria-label={t('previous')} onclick={bgmPrevious}><Icon name="skip_previous" filled /></button>
        <button class="play-toggle" aria-label={sm.bgmState.playing ? t('pause') : t('play')} onclick={bgmToggleAny}>
          <Icon name={sm.bgmState.playing ? 'pause' : 'play_arrow'} size={34} filled />
        </button>
        <button class="md-icon-btn" aria-label={t('next')} onclick={bgmNext}><Icon name="skip_next" filled /></button>
        <button class="mode-toggle" data-tip={modeLabel} aria-label={modeLabel} onclick={cyclePlaybackMode}>
          <Icon name={modeIcon} size={20} filled />
          <span>{modeLabel}</span>
        </button>
      </div>
    </section>

    <section class="lyrics-scroll" aria-label={t('scroll_lyrics')} bind:this={lyricsScrollEl}>
      {#if lines.length}
        <div class="lyrics-spacer"></div>
        {#each lines as line, index (`${line.t}-${index}`)}
          <button
            class="lyric-row"
            class:active={index === lineIdx}
            class:passed={index < lineIdx}
            data-active={index === lineIdx}
            onclick={() => bgmSeek(line.t / 1000)}
          >{line.l}</button>
        {/each}
        <div class="lyrics-spacer"></div>
      {:else}
        <div class="no-lyrics">
          <Icon name="lyrics" size={54} color="var(--md-outline)" />
          <span>{t('no_lyrics')}</span>
          <small>{t('no_lyrics_hint')}</small>
        </div>
      {/if}
    </section>
  </main>
</div>

<style>
  .lyrics-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; background: color-mix(in srgb, var(--md-surface) 88%, transparent); }
  .lyrics-header { min-height: 68px; display: flex; align-items: center; gap: 14px; padding: 10px 22px; border-bottom: 1px solid var(--md-outline-variant); }
  .header-copy { flex: 1; min-width: 0; }
  .header-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 18px; font-weight: 700; }
  .header-artist { margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--md-on-surface-variant); font-size: 12px; }
  .playing-state { color: var(--md-primary); font-size: 12px; }
  .lyrics-main { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(300px, 42%) minmax(0, 1fr); }
  .vinyl-section { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 20px; padding: 28px; border-right: 1px solid var(--md-outline-variant); overflow: hidden; }
  .vinyl { position: relative; width: min(34vw, 340px); aspect-ratio: 1; display: grid; place-items: center; border-radius: 50%; background: radial-gradient(circle at 45% 38%, #35343b, #111116 72%); box-shadow: 0 18px 50px rgb(0 0 0 / 0.36), inset 0 0 0 1px rgb(255 255 255 / 0.08); animation: vinyl-spin 12s linear infinite; animation-play-state: paused; }
  .vinyl.spinning { animation-play-state: running; }
  .vinyl-grooves { position: absolute; inset: 4%; border-radius: 50%; background: repeating-radial-gradient(circle, transparent 0 8px, rgb(255 255 255 / 0.035) 9px 10px); }
  .vinyl-cover { position: relative; width: 58%; aspect-ratio: 1; display: grid; place-items: center; overflow: hidden; border-radius: 50%; background: var(--md-surface-container-high); box-shadow: 0 0 0 2px rgb(255 255 255 / 0.08); }
  .vinyl-cover img { width: 100%; height: 100%; object-fit: cover; }
  .vinyl-hole { position: absolute; width: 7%; aspect-ratio: 1; border-radius: 50%; background: var(--md-surface); box-shadow: inset 0 1px 4px rgb(0 0 0 / 0.4); }
  .track-copy { max-width: 90%; text-align: center; }
  .track-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 20px; font-weight: 700; }
  .track-artist { margin-top: 6px; color: var(--md-on-surface-variant); font-size: 13px; }
  .fullscreen-controls { display: flex; align-items: center; gap: 24px; }
  .play-toggle { width: 62px; height: 62px; display: grid; place-items: center; border: 0; border-radius: 50%; background: var(--md-primary); color: var(--md-on-primary); cursor: pointer; box-shadow: 0 6px 20px var(--md-primary-02); transition: transform 180ms ease, box-shadow 180ms ease; }
  .mode-toggle { height: 38px; display: inline-flex; align-items: center; gap: 6px; padding: 0 12px; border: 1px solid var(--md-outline-variant); border-radius: 999px; background: transparent; color: var(--md-primary); cursor: pointer; font-size: 11px; transition: background 160ms ease, transform 160ms ease; }
  .mode-toggle:hover { background: var(--md-primary-06); }
  .mode-toggle:active { transform: scale(0.95); }
  .play-toggle:hover { transform: scale(1.06); }
  .play-toggle:active { transform: scale(0.94); }
  .lyrics-scroll { min-height: 0; overflow-y: auto; padding: 0 clamp(20px, 5vw, 70px); scroll-behavior: smooth; }
  .lyrics-spacer { height: 42vh; }
  .lyric-row { width: 100%; display: block; padding: 17px 12px; border: 0; border-radius: 14px; background: transparent; color: var(--md-on-surface-variant); text-align: left; font-size: clamp(16px, 2vw, 22px); line-height: 1.65; cursor: pointer; transition: color 280ms ease, transform 280ms ease, background 280ms ease, opacity 280ms ease; }
  .lyric-row:hover { background: var(--md-primary-06); }
  .lyric-row.active { color: var(--md-primary); font-weight: 700; transform: translateX(8px); background: var(--md-primary-06); }
  .lyric-row.passed { opacity: 0.48; }
  .no-lyrics { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; color: var(--md-on-surface-variant); }
  .no-lyrics small { opacity: 0.7; }
  @keyframes vinyl-spin { to { transform: rotate(360deg); } }
  @media (max-width: 720px) {
    .lyrics-main { grid-template-columns: 1fr; grid-template-rows: auto minmax(0, 1fr); }
    .vinyl-section { gap: 12px; padding: 16px; border-right: 0; border-bottom: 1px solid var(--md-outline-variant); }
    .vinyl { width: 168px; }
    .track-title { font-size: 16px; }
    .fullscreen-controls { gap: 18px; }
    .play-toggle { width: 48px; height: 48px; }
    .lyric-row { font-size: 16px; }
    .lyrics-spacer { height: 22vh; }
  }
  @media (prefers-reduced-motion: reduce) {
    .vinyl { animation: none; }
    .lyric-row { transition: none; }
  }
</style>
