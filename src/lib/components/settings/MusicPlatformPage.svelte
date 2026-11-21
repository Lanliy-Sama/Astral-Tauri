<script>
  /** 在线音乐 —— 平台扫码登录（网易云 / QQ / 酷狗）+ 搜索播放。
   *  登录状态与二维码经 Rust 命令轮询；搜索结果点击即加入在线队列播放，
   *  歌词胶囊跟随 audio.currentTime 实时滚动。 */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { bgmPlayOnlineTrack } from '$lib/services/BgmPlayer.svelte';
  import { isTauri } from '$lib/utils/Platform.svelte';
  import { onMount, untrack } from 'svelte';

  const PLATFORMS = [
    { key: 'netease', label: '网易云', qr: true },
    { key: 'qq', label: 'QQ 音乐', qr: false },
    { key: 'kugou', label: '酷狗', qr: true },
  ];

  let platform = $state('netease');
  let loginState = $state({ state: 'idle', qrSvg: '', qrPng: '', message: '' });
  let status = $state({ loggedIn: false, nickname: '' });
  let query = $state('');
  let searching = $state(false);
  let results = $state([]);
  let searchError = $state('');
  let playlists = $state([]);
  let playlistTracks = $state([]);
  let selectedPlaylist = $state(null);
  let loadingTracks = $state(false);

  let pollTimer = 0;
  let lastQrVersion = '';

  function stopPoll() {
    clearInterval(pollTimer);
    pollTimer = 0;
  }

  async function refreshStatus(p = platform) {
    if (!isTauri()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      status = await invoke('music_status', { platform: p });
    } catch {
      status = { loggedIn: false };
    }
  }

  async function startLogin(p = platform) {
    if (!isTauri()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('music_login_start', { platform: p });
      loginState = await invoke('music_login_state', {});
      stopPoll();
      pollTimer = setInterval(async () => {
        try {
          const st = await invoke('music_login_state', {});
          if (p !== platform) {
            stopPoll();
            return;
          }
          loginState = st;
          if (st.state === 'success') {
            stopPoll();
            await refreshStatus(p);
            if (p === 'netease') await loadPlaylists();
          } else if (st.state === 'expired' || st.state === 'error') {
            stopPoll();
          }
        } catch {
          stopPoll();
        }
      }, 1500);
    } catch (e) {
      loginState = { state: 'error', message: String(e?.message || e) };
    }
  }

  async function logout() {
    if (!isTauri()) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('music_logout', { platform });
    } catch {
      /* 忽略 */
    }
    await refreshStatus();
    playlists = [];
    playlistTracks = [];
    selectedPlaylist = null;
    /* 退出后回到扫码态 */
    if (PLATFORMS.find((x) => x.key === platform)?.qr && isTauri()) {
      startLogin();
    }
  }

  async function loadPlaylists() {
    if (!isTauri() || platform !== 'netease' || !status.loggedIn) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke('music_playlists', { platform });
      playlists = res.playlists || [];
    } catch (e) {
      console.warn('歌单读取失败:', e);
    }
  }

  async function openPlaylist(p) {
    if (!isTauri()) return;
    selectedPlaylist = p;
    loadingTracks = true;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke('music_playlist_tracks', { platform, id: p.id });
      playlistTracks = res.tracks || [];
      sm.bgmState.onlineQueue = playlistTracks;
      sm.bgmState.onlineIndex = -1;
    } catch (e) {
      console.warn('歌单曲目读取失败:', e);
    } finally {
      loadingTracks = false;
    }
  }

  async function doSearch() {
    const q = query.trim();
    if (!q || !isTauri()) return;
    searching = true;
    searchError = '';
    selectedPlaylist = null;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke('music_search', { platform, query: q });
      results = res.tracks || [];
      sm.bgmState.onlineQueue = results;
      sm.bgmState.onlineIndex = -1;
      if (!results.length) searchError = '没有找到匹配的歌曲';
    } catch (e) {
      results = [];
      searchError = String(e?.message || e);
    } finally {
      searching = false;
    }
  }

  async function playFrom(list, index) {
    const track = list[index];
    if (!track) return;
    try {
      sm.bgmState.onlineQueue = list;
      sm.bgmState.onlineIndex = index;
      await bgmPlayOnlineTrack(track);
      // 歌词拉取由歌词胶囊监听 onlineTrack 变化完成
    } catch (e) {
      console.warn('在线播放失败:', e);
    }
  }

  /* 只跟随 platform 变化：切平台清状态 → 刷新登录态 → 未登录且支持扫码
     则自动出二维码。status 的读取用 untrack 包裹，避免刷新登录态反过来
     触发本 effect 重跑（否则会反复重新出二维码）。 */
  $effect(() => {
    const p = platform;
    untrack(() => {
      stopPoll();
      loginState = { state: 'idle', qrSvg: '', qrPng: '', message: '' };
      results = [];
      playlists = [];
      playlistTracks = [];
      selectedPlaylist = null;
      searchError = '';
      void (async () => {
        await refreshStatus(p);
        if (p !== platform) return; // 已切走，丢弃过期结果
        if (status.loggedIn) {
          if (p === 'netease') await loadPlaylists();
          return;
        }
        if (PLATFORMS.find((x) => x.key === p)?.qr && isTauri()) {
          startLogin(p);
        }
      })();
    });
  });

  onMount(() => () => stopPoll());

  const stateText = {
    loading: '二维码生成中…',
    waiting: '请用手机 App 扫码登录',
    scanned: '已扫码，请在手机上确认',
    success: '登录成功',
    expired: '二维码已过期，请刷新',
    error: '登录失败，请刷新重试',
    idle: '',
  };
</script>

<SubPage title="在线音乐">
  <div class="page-list">
    <!-- 平台切换 -->
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="album" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">音乐平台</div>
          <div class="tile-subtitle">扫码登录后播放账号有权限的歌曲（网易云 / QQ / 酷狗）</div>
        </div>
      </div>
      <div class="md-divider"></div>
      <div class="platform-tabs">
        {#each PLATFORMS as p (p.key)}
          <button
            class="platform-tab {platform === p.key ? 'selected' : ''}"
            onclick={() => (platform = p.key)}
          >
            {p.label}
          </button>
        {/each}
      </div>

      {#if loginState.state !== 'idle' && loginState.state !== 'success'}
        <div class="qr-area">
          {#if loginState.qrPng}
            <img class="qr-img" src={loginState.qrPng} alt="登录二维码" />
          {:else if loginState.qrSvg}
            <div class="qr-svg">{@html loginState.qrSvg}</div>
          {:else if loginState.state === 'loading'}
            <div class="qr-loading">二维码生成中…</div>
          {/if}
          <div class="qr-state">{stateText[loginState.state] || loginState.message || loginState.state}</div>
        </div>
      {:else if status.loggedIn}
        <div class="md-list-tile two-line no-cursor">
          <span class="tile-leading"><Icon name="person" size={24} color="var(--md-primary)" /></span>
          <div class="tile-content">
            <div class="tile-title">已登录{status.nickname ? `：${status.nickname}` : ''}</div>
            <div class="tile-subtitle">可以播放账号有权限的歌曲了</div>
          </div>
          <span class="tile-trailing">
            <button class="md-btn md-text-btn" onclick={logout}>退出登录</button>
          </span>
        </div>
      {:else if PLATFORMS.find((p) => p.key === platform)?.qr}
        <div class="qr-area qr-idle">
          <button class="md-btn md-tonal-btn" onclick={startLogin}>获取登录二维码</button>
        </div>
      {:else if platform === 'qq'}
        <div class="md-list-tile two-line no-cursor">
          <div class="tile-content">
            <div class="tile-subtitle">QQ 音乐扫码登录暂未开放；当前可直接搜索播放免费歌曲</div>
          </div>
        </div>
      {/if}
    </SettingsCard>

    <div class="gap-16"></div>

    <!-- 搜索 -->
    <SettingsCard>
      <div class="search-row">
        <input
          class="search-input"
          type="text"
          placeholder={`在 ${PLATFORMS.find((p) => p.key === platform)?.label || ''} 搜索歌曲`}
          bind:value={query}
          onkeydown={(e) => {
            if (e.key === 'Enter') doSearch();
          }}
        />
        <button class="md-btn md-tonal-btn" onclick={doSearch} disabled={searching || !query.trim()}>
          搜索
        </button>
      </div>
      {#if searchError}
        <div class="search-error">{searchError}</div>
      {/if}
      {#if results.length}
        <div class="track-list">
          {#each results as track, index (track.key)}
            <div
              class="track-row"
              role="button"
              tabindex="0"
              onclick={() => playFrom(results, index)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') playFrom(results, index);
              }}
            >
              <span class="track-play"><Icon name="play_circle" size={22} color="var(--md-primary)" /></span>
              <div class="track-meta">
                <div class="track-title">{track.title}</div>
                <div class="track-artist">{track.artists.join(' / ')}</div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </SettingsCard>

    <div class="gap-16"></div>

    <!-- 歌单（网易云登录后） -->
    {#if platform === 'netease' && status.loggedIn && playlists.length}
      <SettingsCard>
        <div class="md-list-tile two-line no-cursor">
          <span class="tile-leading"><Icon name="queue_music" size={24} color="var(--md-on-surface-variant)" /></span>
          <div class="tile-content">
            <div class="tile-title">我的歌单</div>
            <div class="tile-subtitle">点开播放账号歌单里的歌曲</div>
          </div>
        </div>
        <div class="md-divider"></div>
        <div class="playlist-list">
          {#each playlists as p (p.id)}
            <div
              class="playlist-row"
              role="button"
              tabindex="0"
              onclick={() => openPlaylist(p)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') openPlaylist(p);
              }}
            >
              <div class="track-meta">
                <div class="track-title">{p.name}</div>
                <div class="track-artist">{p.count} 首</div>
              </div>
              <span class="tile-trailing"><Icon name="chevron_right" size={22} /></span>
            </div>
          {/each}
        </div>
        {#if selectedPlaylist}
          <div class="md-divider"></div>
          <div class="playlist-head">
            {selectedPlaylist.name}{loadingTracks ? ' · 加载中…' : ` · ${playlistTracks.length} 首`}
          </div>
          {#if !loadingTracks}
            <div class="track-list">
              {#each playlistTracks as track, index (track.key)}
                <div
                  class="track-row"
                  role="button"
                  tabindex="0"
                  onclick={() => playFrom(playlistTracks, index)}
                  onkeydown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') playFrom(playlistTracks, index);
                  }}
                >
                  <span class="track-play"><Icon name="play_circle" size={22} color="var(--md-primary)" /></span>
                  <div class="track-meta">
                    <div class="track-title">{track.title}</div>
                    <div class="track-artist">{track.artists.join(' / ')}</div>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </SettingsCard>
    {/if}
  </div>
</SubPage>

<style>
  .page-list {
    padding: 16px;
  }
  .gap-16 {
    height: 16px;
  }
  .platform-tabs {
    display: flex;
    padding: 12px 16px 4px;
    gap: 8px;
  }
  .platform-tab {
    flex: 1 1 0;
    padding: 10px 0;
    border-radius: 999px;
    border: 1px solid var(--md-outline-variant);
    background: transparent;
    color: var(--md-on-surface);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
  }
  .platform-tab.selected {
    background: var(--md-secondary-container);
    border-color: transparent;
    color: var(--md-on-secondary-container);
  }
  .qr-area {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 16px;
  }
  .qr-area.qr-idle {
    padding: 20px 16px;
  }
  .qr-img,
  .qr-svg :global(svg) {
    width: 200px;
    height: 200px;
    border-radius: 12px;
    background: #ffffff;
    padding: 8px;
  }
  .qr-loading {
    width: 200px;
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--md-on-surface-variant);
    border-radius: 12px;
    background: var(--md-surface-container-low);
  }
  .qr-state {
    font-size: 14px;
    color: var(--md-on-surface-variant);
  }
  .search-row {
    display: flex;
    gap: 8px;
    padding: 12px 16px;
  }
  .search-input {
    flex: 1 1 0;
    padding: 10px 14px;
    border-radius: 10px;
    border: 1px solid var(--md-outline-variant);
    background: var(--md-surface-container-low);
    color: var(--md-on-surface);
    font-size: 14px;
    outline: none;
  }
  .search-input:focus {
    border-color: var(--md-primary);
  }
  .search-error {
    padding: 0 16px 10px;
    color: var(--md-error);
    font-size: 13px;
  }
  .track-list {
    max-height: 420px;
    overflow-y: auto;
    padding: 4px 8px 8px;
  }
  .track-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 10px;
    cursor: pointer;
  }
  .track-row:hover {
    background: var(--md-primary-06);
  }
  .track-play {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }
  .track-meta {
    min-width: 0;
    flex: 1 1 0;
  }
  .track-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--md-on-surface);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .track-artist {
    font-size: 12px;
    color: var(--md-on-surface-variant);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .playlist-list {
    max-height: 300px;
    overflow-y: auto;
    padding: 4px 8px 8px;
  }
  .playlist-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 10px;
    cursor: pointer;
  }
  .playlist-row:hover {
    background: var(--md-primary-06);
  }
  .playlist-head {
    padding: 10px 16px;
    font-size: 13px;
    color: var(--md-on-surface-variant);
  }
</style>
