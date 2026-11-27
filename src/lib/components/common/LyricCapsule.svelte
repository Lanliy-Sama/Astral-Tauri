<script>
  /** 统一灵动岛：歌词、音量反馈与播放控制共享同一个居中容器。
   * Ctrl+↑/↓ 只在歌词岛尾部展开音量条；暂停后仍保留岛屿以便恢复播放。 */
  import { onMount } from 'svelte';
  import { sm } from '../../services/ServiceManager.svelte';
  import {
    bgmPosition,
    bgmPrevious,
    bgmNext,
    bgmToggleAny,
  } from '../../services/BgmPlayer.svelte';
  import { isTauri } from '../../utils/Platform.svelte';
  import { voiceService } from '../../services/VoiceService.svelte';
  import FullscreenLyricsPage from './FullscreenLyricsPage.svelte';
  import { pushPage } from '../../router/PageRouter.svelte';
  import { t } from '../../i18n/I18n.svelte';

  let lineIdx = $state(-1);
  let expanded = $state(false);
  let volumeVisible = $state(false);
  let volumeTimer = 0;
  let clickTimer = 0;

  const invitation = $derived(sm.appSettingsState.experimentalFeatures ? voiceService.invites[0] || null : null);
  const hasTrack = $derived(!!sm.bgmState.onlineTrack || !!sm.bgmState.trackId);
  const active = $derived(sm.bgmState.lyricCapsuleEnabled && hasTrack);
  const lines = $derived(sm.bgmState.lyricLines || []);
  const label = $derived.by(() => {
    const track = sm.bgmState.onlineTrack;
    if (track) {
      const artists = track.artists?.join(' / ') || track.artist || '';
      return track.title + (artists ? ' · ' + artists : '');
    }
    const local = sm.bgmState.tracks.find((x) => x.trackId === sm.bgmState.trackId);
    return local ? local.name : '';
  });

  const modeLabel = $derived(sm.bgmState.playbackMode === 'random' ? t('mode_random') : sm.bgmState.playbackMode === 'loop' ? t('mode_loop') : t('mode_sequence'));
  const modeIcon = $derived(sm.bgmState.playbackMode === 'random' ? 'shuffle' : sm.bgmState.playbackMode === 'loop' ? 'repeat_one' : 'repeat');

  function cyclePlaybackMode() {
    const next = sm.bgmState.playbackMode === 'sequence' ? 'random' : sm.bgmState.playbackMode === 'random' ? 'loop' : 'sequence';
    sm.updatePlaybackMode(next);
  }

  $effect(() => {
    const track = sm.bgmState.onlineTrack;
    lineIdx = -1;
    expanded = false;
    if (!track) {
      sm.bgmState.lyricLines = null;
      sm.bgmState.lyricFound = false;
      return;
    }
    if (track.sharedStream) {
      sm.bgmState.lyricLines = track.sharedLyrics?.length ? track.sharedLyrics : null;
      sm.bgmState.lyricFound = !!track.sharedLyrics?.length;
      return;
    }
    if (!isTauri()) return;
    let dead = false;
    (async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke('music_lyrics', {
          platform: track.platform,
          key: track.key,
          track,
        });
        if (dead) return;
        sm.bgmState.lyricLines = res?.lines?.length ? res.lines : null;
        sm.bgmState.lyricFound = !!res?.found;
      } catch (e) {
        if (!dead) {
          sm.bgmState.lyricLines = null;
          sm.bgmState.lyricFound = false;
          sm.pushLog(`[歌词] ${track.title || track.key}: ${e?.message || e}`);
        }
      }
    })();
    return () => {
      dead = true;
    };
  });

  function syncDesktopLyrics() {
    if (!sm.bgmState.desktopLyricsEnabled) return;
    const line = lineIdx >= 0 && lineIdx < lines.length ? lines[lineIdx] : null;
    import('@tauri-apps/api/event')
      .then(({ emit }) => emit('desktop-lyrics-update', {
        title: label,
        playing: sm.bgmState.playing,
        line: line ? { l: line.l, tr: line.tr || '', ro: line.ro || '' } : null,
      }))
      .catch(() => {});
  }

  function tick() {
    if (!lines.length) {
      lineIdx = -1;
      syncDesktopLyrics();
      return;
    }
    const pos = bgmPosition() * 1000;
    let low = 0;
    let high = lines.length;
    while (low < high) {
      const mid = (low + high) >> 1;
      if (lines[mid].t <= pos) low = mid + 1;
      else high = mid;
    }
    const idx = low - 1;
    if (idx !== lineIdx) lineIdx = idx;
    syncDesktopLyrics();
  }

  $effect(() => {
    void lines.length;
    void sm.bgmState.desktopLyricsEnabled;
    tick();
  });

  $effect(() => {
    if (!sm.bgmState.playing) return;
    const timer = setInterval(tick, 250);
    return () => clearInterval(timer);
  });

  $effect(() => {
    const pulse = sm.bgmState.volumePulse;
    if (!pulse) return;
    volumeVisible = true;
    expanded = false;
    clearTimeout(volumeTimer);
    volumeTimer = setTimeout(() => (volumeVisible = false), 1600);
  });

  $effect(() => {
    if (sm.appSettingsState.experimentalFeatures) voiceService.start();
  });

  onMount(() => {
    let unlistenDesktop = null;
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen('desktop-lyrics-close-request', () => sm.updateDesktopLyricsEnabled(false)))
      .then((dispose) => (unlistenDesktop = dispose))
      .catch(() => {});
    return () => {
      clearTimeout(volumeTimer);
      clearTimeout(clickTimer);
      unlistenDesktop?.();
    };
  });

  const currentLine = $derived(lineIdx >= 0 && lineIdx < lines.length ? lines[lineIdx].l : '');
  const show = $derived(!sm.bgmState.lyricsFullScreen && (!!invitation || volumeVisible || (active && label !== '')));

  function toggleExpanded() {
    if (invitation) return;
    if (active && !volumeVisible) expanded = !expanded;
    else if (active) {
      volumeVisible = false;
      expanded = true;
    }
  }

  function onCapsuleClick() {
    if (invitation) return;
    clearTimeout(clickTimer);
    clickTimer = setTimeout(toggleExpanded, 230);
  }

  function openFullscreenLyrics() {
    clearTimeout(clickTimer);
    expanded = false;
    volumeVisible = false;
    pushPage(FullscreenLyricsPage);
  }

  async function acceptCurrentInvite() {
    if (!invitation) return;
    try {
      await voiceService.acceptInvite(invitation);
    } catch {
      // 具体错误保留在 voiceService.error，房间语音面板中可见。
    }
  }
</script>

{#if show}
  <div
    class="lyric-capsule"
    class:expanded
    class:volume-visible={volumeVisible}
    role={active && !invitation ? 'button' : 'status'}
    tabindex={active && !invitation ? 0 : -1}
    aria-expanded={expanded}
    aria-label={invitation ? '语音频道邀请' : active ? '展开音乐控制' : '背景音乐音量'}
    onclick={onCapsuleClick}
    ondblclick={openFullscreenLyrics}
    onkeydown={(e) => {
      if (active && (e.key === 'Enter' || e.key === ' ')) {
        e.preventDefault();
        toggleExpanded();
      }
    }}
  >
    <div class="capsule-main">
      {#if invitation}
        <span class="ms filled lyr-note">podcasts</span>
        <div class="lyr-body invite-body">
          <div class="lyr-line">{invitation.channel.ownerName} 邀请你加入语音</div>
          <div class="lyr-track">{invitation.channel.name} · 5 分钟内有效</div>
        </div>
        <div class="invite-actions" onclick={(e) => e.stopPropagation()}>
          <button class="invite-btn accept" onclick={acceptCurrentInvite}>加入</button>
          <button class="invite-btn" onclick={() => voiceService.declineInvite(invitation.id)}>忽略</button>
        </div>
      {:else}
        {#if active}
          <span class="ms filled lyr-note">{sm.bgmState.playing ? 'graphic_eq' : 'music_note'}</span>
          <div class="lyr-body">
            {#if currentLine}
              <div class="lyr-line">{currentLine}</div>
              <div class="lyr-track">{label}</div>
            {:else}
              <div class="lyr-line">{label || '　'}</div>
              <div class="lyr-track">{sm.bgmState.playing ? '正在播放' : '已暂停 · 单击展开控制'}</div>
            {/if}
          </div>
        {/if}
        <div class="lyr-volume" class:visible={volumeVisible} aria-hidden={!volumeVisible}>
          <span class="ms filled volume-icon">
            {sm.bgmState.volume === 0 ? 'volume_off' : sm.bgmState.volumeDelta >= 0 ? 'volume_up' : 'volume_down'}
          </span>
          <div class="volume-track">
            <div class="volume-fill" style="width:{sm.bgmState.volume}%"></div>
          </div>
          <span class="volume-num">{sm.bgmState.volume}</span>
        </div>
        {#if active && !volumeVisible}
          <span class="ms expand-icon">{expanded ? 'expand_less' : 'expand_more'}</span>
        {/if}
      {/if}
    </div>

    {#if expanded && active && !invitation}
      <div class="controls" onclick={(e) => e.stopPropagation()}>
        <button class="control-btn" data-tip={t('previous')} aria-label={t('previous')} onclick={bgmPrevious}>
          <span class="ms filled">skip_previous</span>
        </button>
        <button class="control-btn primary" aria-label={sm.bgmState.playing ? t('pause') : t('play')} onclick={bgmToggleAny}>
          <span class="ms filled">{sm.bgmState.playing ? 'pause' : 'play_arrow'}</span>
        </button>
        <button class="control-btn" data-tip={t('next')} aria-label={t('next')} onclick={bgmNext}>
          <span class="ms filled">skip_next</span>
        </button>
        <button class="control-btn" data-tip={modeLabel} aria-label={modeLabel} onclick={cyclePlaybackMode}>
          <span class="ms filled">{modeIcon}</span>
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .lyric-capsule {
    position: fixed;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    max-width: min(760px, 94vw);
    padding: 9px 14px;
    border-radius: 999px;
    background: rgb(24 24 27 / 0.94);
    color: #ffffff;
    box-shadow: 0 5px 18px rgb(0 0 0 / 0.3), inset 0 0 0 1px rgb(255 255 255 / 0.09);
    backdrop-filter: blur(14px) saturate(1.35);
    z-index: 3500;
    cursor: pointer;
    transform-origin: top center;
    animation: island-in 260ms cubic-bezier(0.05, 0.7, 0.1, 1);
    transition: border-radius 280ms cubic-bezier(0.05, 0.7, 0.1, 1),
      box-shadow 240ms ease, padding 280ms cubic-bezier(0.05, 0.7, 0.1, 1);
  }
  .lyric-capsule.expanded {
    border-radius: 26px;
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.36), inset 0 0 0 1px rgb(255 255 255 / 0.12);
  }
  .capsule-main {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 38px;
    min-width: 0;
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
    justify-content: center;
    gap: 1px;
    min-width: 0;
    max-width: min(480px, 58vw);
  }
  .lyr-line,
  .lyr-track {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
  }
  .lyr-line { font-size: 14px; font-weight: 600; line-height: 18px; }
  .lyr-track { font-size: 11px; line-height: 14px; color: rgb(255 255 255 / 0.62); }
  .expand-icon { width: 18px; height: 18px; font-size: 18px; opacity: 0.65; flex-shrink: 0; }
  .invite-body { max-width: min(330px, 42vw); text-align: left; }
  .invite-body .lyr-line, .invite-body .lyr-track { text-align: left; }
  .invite-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .invite-btn {
    height: 30px;
    padding: 0 12px;
    border: 1px solid rgb(255 255 255 / 0.18);
    border-radius: 999px;
    background: transparent;
    color: #fff;
    cursor: pointer;
    font-size: 12px;
  }
  .invite-btn.accept { background: #fff; color: #18181b; border-color: #fff; }
  .invite-btn:active { transform: scale(0.94); }
  .lyr-volume {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 0;
    max-width: 0;
    opacity: 0;
    overflow: hidden;
    transition: width 260ms cubic-bezier(0.05, 0.7, 0.1, 1),
      max-width 260ms cubic-bezier(0.05, 0.7, 0.1, 1), opacity 160ms ease;
  }
  .lyr-volume.visible { width: 174px; max-width: 174px; opacity: 1; }
  .volume-icon { width: 20px; height: 20px; font-size: 20px; flex-shrink: 0; }
  .volume-track { width: 104px; height: 5px; border-radius: 999px; background: rgb(255 255 255 / 0.2); overflow: hidden; }
  .volume-fill { height: 100%; border-radius: inherit; background: #fff; transition: width 140ms ease; }
  .volume-num { min-width: 26px; text-align: right; font-size: 12px; font-variant-numeric: tabular-nums; }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 22px;
    padding: 7px 20px 3px;
    overflow: hidden;
    animation: controls-expand 340ms cubic-bezier(0.05, 0.7, 0.1, 1);
  }
  .controls .control-btn {
    animation: control-button-in 300ms cubic-bezier(0.05, 0.7, 0.1, 1) both;
  }
  .controls .control-btn:nth-child(2) { animation-delay: 35ms; }
  .controls .control-btn:nth-child(3) { animation-delay: 70ms; }
  .controls .control-btn:nth-child(4) { animation-delay: 105ms; }
  .control-btn {
    width: 38px;
    height: 38px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 0;
    border-radius: 50%;
    color: #fff;
    background: transparent;
    cursor: pointer;
    transition: background 140ms ease, transform 140ms ease;
  }
  .control-btn:hover { background: rgb(255 255 255 / 0.12); }
  .control-btn:active { transform: scale(0.9); }
  .control-btn.primary { background: #fff; color: #18181b; width: 42px; height: 42px; }
  .control-btn .ms { width: 25px; height: 25px; font-size: 25px; }
  @keyframes island-in { from { opacity: 0; transform: translate(-50%, -12px) scale(0.94); } to { opacity: 1; transform: translate(-50%, 0) scale(1); } }
  @keyframes controls-expand {
    from { max-height: 0; padding-top: 0; padding-bottom: 0; opacity: 0; transform: translateY(-8px) scale(0.96); }
    to { max-height: 76px; opacity: 1; transform: translateY(0) scale(1); }
  }
  @keyframes control-button-in {
    from { opacity: 0; transform: translateY(-6px) scale(0.9); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }
  @media (max-width: 480px) {
    .lyr-body { max-width: 42vw; }
    .lyr-volume.visible { width: 142px; max-width: 142px; }
    .volume-track { width: 76px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .lyric-capsule, .lyr-volume, .volume-fill, .control-btn { animation: none; transition: none; }
  }
</style>
