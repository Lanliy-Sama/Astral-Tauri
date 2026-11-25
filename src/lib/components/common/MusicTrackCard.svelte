<script>
  /** 平铺液态玻璃音乐卡片：大封面、来源、歌手、时长与播放状态。 */
  import Icon from './Icon.svelte';
  import { sm } from '../../services/ServiceManager.svelte';

  let { track, onPlay = () => {}, sourceLabel = '', onDelete = null } = $props();
  let coverFailed = $state(false);
  let coverLoaded = $state(false);
  const artistText = $derived(track.artists?.join(' / ') || track.artist || '未知歌手');
  const isCurrent = $derived(sm.bgmState.onlineTrack?.key === track.key || (!sm.bgmState.onlineTrack && track.localTrackId && sm.bgmState.trackId === track.localTrackId));
  const duration = $derived.by(() => {
    const seconds = Math.floor((track.durationMs || 0) / 1000);
    return seconds ? `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}` : '';
  });
  const platformName = $derived(({ netease: '网易云', qq: 'QQ 音乐', kugou: '酷狗', local: '本地' })[track.platform] || track.platform || '音乐');

  $effect(() => {
    void track.cover;
    coverFailed = false;
    coverLoaded = false;
  });
</script>

<div
  class="music-track-card lg-glass lg-spec {track.platform || 'local'}"
  class:current={isCurrent}
  role="button"
  tabindex="0"
  onclick={onPlay}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onPlay(); }
  }}
>
  <div class="cover-area">
    {#if track.cover && !coverFailed}
      <img
        class="cover"
        class:loaded={coverLoaded}
        src={track.cover}
        alt={`${track.title || '歌曲'}封面`}
        loading="lazy"
        decoding="async"
        referrerpolicy="no-referrer"
        onload={() => (coverLoaded = true)}
        onerror={() => (coverFailed = true)}
      />
    {:else}
      <div class="cover-placeholder">
        <span class="placeholder-ring"></span>
        <Icon name={track.platform === 'local' ? 'audio_file' : 'album'} size={54} color="rgba(255,255,255,.9)" />
      </div>
    {/if}
    <span class="platform-chip">{platformName}</span>
    {#if duration}<span class="duration-chip">{duration}</span>{/if}
    <span class="card-play">
      <Icon name={isCurrent && sm.bgmState.playing ? 'pause' : 'play_arrow'} size={30} color="#fff" filled />
    </span>
    {#if onDelete}
      <button class="delete-btn" data-tip="从本地音乐删除" aria-label="删除本地音乐" onclick={(e) => { e.stopPropagation(); onDelete(track); }}>
        <Icon name="delete" size={19} color="#fff" />
      </button>
    {/if}
  </div>
  <div class="card-copy">
    <div class="card-title" data-tip={track.title}>{track.title || '未知歌曲'}</div>
    <div class="card-artist" data-tip={artistText}>{artistText}</div>
    <div class="card-footer">
      <span class="source">{sourceLabel || (track.sharedAccountName ? `${track.sharedAccountName} 共享` : platformName)}</span>
      {#if isCurrent}<span class="ms filled current-icon">{sm.bgmState.playing ? 'graphic_eq' : 'pause_circle'}</span>{/if}
    </div>
  </div>
</div>

<style>
  .music-track-card {
    min-width: 0;
    overflow: hidden;
    border: 1px solid var(--md-outline-variant);
    border-radius: 20px;
    background: var(--md-surface-container-low);
    cursor: pointer;
    transition: transform 220ms cubic-bezier(0.05, 0.7, 0.1, 1), border-color 180ms ease, box-shadow 220ms ease;
  }
  .music-track-card:hover { transform: translateY(-3px); border-color: var(--md-primary); }
  .music-track-card:active { transform: translateY(0) scale(0.985); }
  .music-track-card.current { border-color: var(--md-primary); }
  .cover-area { position: relative; aspect-ratio: 1; overflow: hidden; background: linear-gradient(145deg, #65558f, #256b8c); }
  .netease .cover-area { background: linear-gradient(145deg, #bd3343, #59324f); }
  .qq .cover-area { background: linear-gradient(145deg, #1d9d74, #275379); }
  .kugou .cover-area { background: linear-gradient(145deg, #2784c7, #4b4a94); }
  .local .cover-area { background: linear-gradient(145deg, #7d5a96, #26717b); }
  .cover { width: 100%; height: 100%; object-fit: cover; opacity: 0; transform: scale(1.025); transition: opacity 240ms ease, transform 500ms cubic-bezier(0.05, 0.7, 0.1, 1); }
  .cover.loaded { opacity: 1; transform: scale(1); }
  .music-track-card:hover .cover.loaded { transform: scale(1.045); }
  .cover-placeholder { position: absolute; inset: 0; display: grid; place-items: center; overflow: hidden; }
  .placeholder-ring { position: absolute; width: 68%; aspect-ratio: 1; border: 1px solid rgb(255 255 255 / 0.18); border-radius: 50%; box-shadow: 0 0 0 28px rgb(255 255 255 / 0.04), 0 0 0 58px rgb(255 255 255 / 0.03); }
  .platform-chip, .duration-chip { position: absolute; top: 10px; padding: 4px 8px; border-radius: 9px; background: rgb(15 15 20 / 0.55); color: #fff; font-size: 10px; backdrop-filter: blur(10px); }
  .platform-chip { left: 10px; }
  .duration-chip { right: 10px; font-variant-numeric: tabular-nums; }
  .card-play { position: absolute; right: 12px; bottom: 12px; width: 48px; height: 48px; display: grid; place-items: center; border: 1px solid rgb(255 255 255 / 0.45); border-radius: 50%; background: rgb(20 20 25 / 0.48); box-shadow: 0 5px 18px rgb(0 0 0 / 0.25), inset 0 1px 1px rgb(255 255 255 / 0.35); backdrop-filter: blur(12px); opacity: 0.9; transform: scale(0.94); transition: transform 180ms ease, opacity 180ms ease; }
  .music-track-card:hover .card-play, .music-track-card.current .card-play { opacity: 1; transform: scale(1); }
  .delete-btn { position: absolute; left: 10px; bottom: 10px; width: 34px; height: 34px; display: grid; place-items: center; border: 1px solid rgb(255 255 255 / 0.25); border-radius: 50%; background: rgb(20 20 25 / 0.5); cursor: pointer; opacity: 0; transition: opacity 160ms ease, background 160ms ease; }
  .music-track-card:hover .delete-btn { opacity: 1; }
  .delete-btn:hover { background: var(--md-error); }
  .card-copy { padding: 12px 13px 11px; }
  .card-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 14px; line-height: 21px; font-weight: 700; }
  .card-artist { margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--md-on-surface-variant); font-size: 12px; line-height: 18px; }
  .card-footer { min-height: 20px; display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 7px; }
  .source { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--md-primary); font-size: 10px; }
  .current-icon { width: 19px; height: 19px; color: var(--md-primary); font-size: 19px; }
  @media (prefers-reduced-motion: reduce) {
    .music-track-card, .cover, .card-play, .delete-btn { transition: none; }
  }
</style>
