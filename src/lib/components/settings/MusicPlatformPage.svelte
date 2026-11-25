<script>
  /** 在线音乐 —— 平台扫码登录（网易云 / QQ / 酷狗）+ 搜索播放。
   *  登录状态与二维码经 Rust 命令轮询；搜索结果点击即加入在线队列播放，
   *  歌词胶囊跟随 audio.currentTime 实时滚动。 */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import MusicTrackCard from '$lib/components/common/MusicTrackCard.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import { snackError, snackInfo } from '$lib/components/common/SnackbarStore.svelte';
  import { roomSocial } from '$lib/services/RoomSocial.svelte';
  import { roomListen } from '$lib/services/RoomListen.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import {
    bgmPlayOnlineTrack,
    bgmPlayTrack,
    bgApplyVolume,
    syncBgmSmtc,
  } from '$lib/services/BgmPlayer.svelte';
  import { isTauri } from '$lib/utils/Platform.svelte';
  import { onMount, untrack } from 'svelte';
  import { t } from '$lib/i18n/I18n.svelte';

  const PLATFORMS = [
    { key: 'local', label: '本地音乐', qr: false },
    { key: 'netease', label: '网易云', qr: true },
    { key: 'qq', label: 'QQ 音乐', qr: true },
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
  let sourcePeer = $state('local');

  const sharedAccounts = $derived(roomSocial.accounts.filter((a) => a.platform === platform));
  const selectedSharedAccount = $derived(sharedAccounts.find((a) => a.peer === sourcePeer) || null);

  const AUDIO_EXTS = ['mp3', 'mp4', 'm4a', 'aac', 'flac', 'wav', 'ogg', 'oga', 'opus', 'weba', 'webm'];
  const AUDIO_ACCEPT = 'audio/*,' + AUDIO_EXTS.map((ext) => `.${ext}`).join(',');
  let fileEl = null;
  let importing = $state(false);

  function localTracks(keyword = '') {
    const q = keyword.trim().toLowerCase();
    return sm.bgmState.tracks
      .filter((track) => !q || track.name.toLowerCase().includes(q))
      .map((track) => ({
        key: `local:${track.trackId}`, id: track.trackId, localTrackId: track.trackId,
        platform: 'local', title: track.name, artists: ['本地音乐'],
        cover: '', durationMs: 0,
      }));
  }

  function blobToB64(blob) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).split(',')[1] || '');
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(blob);
    });
  }

  async function importLocalFiles(event) {
    const files = Array.from(event.target.files || []);
    event.target.value = '';
    if (!files.length || importing || !isTauri()) return;
    importing = true;
    let imported = 0;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      for (const file of files) {
        try {
          const ext = (file.name.split('.').pop() || '').toLowerCase();
          if (!AUDIO_EXTS.includes(ext)) {
            snackError(t('unsupported_format'), file.name);
            continue;
          }
          const id = await invoke('bgm_write_begin', { ext });
          for (let offset = 0; offset < file.size; offset += 1 << 20) {
            const data = await blobToB64(file.slice(offset, Math.min(offset + (1 << 20), file.size)));
            await invoke('bgm_write_append', { id, b64: data });
          }
          const info = await invoke('bgm_write_end', { id });
          await sm.addBgmTrack({
            trackId: info.id,
            name: file.name.replace(/\.[^.]+$/, '') || file.name,
            ext: info.ext,
          });
          imported += 1;
        } catch (error) {
          snackError(t('import_failed'), `${file.name}：${error?.message || error}`);
        }
      }
    } finally {
      importing = false;
      results = localTracks(query);
    }
    if (imported) snackInfo(t('import_complete'), `已导入 ${imported} 首本地音乐`);
  }

  async function deleteLocal(track) {
    const row = sm.bgmState.tracks.find((item) => item.trackId === track.localTrackId);
    if (!row) return;
    await sm.removeBgmTrack(row);
    results = localTracks(query);
  }

  async function toggleShare(value) {
    await sm.updateMusicShare(platform, value);
    await roomSocial.refreshOwnAccounts();
    await roomSocial.broadcastPresence();
  }

  async function toggleSmtc(value) {
    await sm.updateSmtcEnabled(value);
    syncBgmSmtc();
  }

  function updateVolume(value) {
    sm.updateBgmVolume(value).catch(() => {});
    bgApplyVolume(value);
  }

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
      if (platform === 'local') {
        results = localTracks(q);
      } else if (selectedSharedAccount) {
        results = await roomSocial.search(selectedSharedAccount, q);
      } else {
        const { invoke } = await import('@tauri-apps/api/core');
        const res = await invoke('music_search', { platform, query: q });
        results = res.tracks || [];
      }
      sm.bgmState.onlineQueue = results;
      sm.bgmState.onlineIndex = -1;
      if (!results.length) searchError = t('no_results');
    } catch (e) {
      results = [];
      searchError = String(e?.message || e);
    } finally {
      searching = false;
    }
  }

  async function playFrom(list, index) {
    let track = list[index];
    if (!track) return;
    try {
      if (track.platform === 'local') {
        const localIndex = sm.bgmState.tracks.findIndex((item) => item.trackId === track.localTrackId);
        if (localIndex < 0) throw new Error('本地曲目已被删除');
        if (!sm.bgmState.enabled) await sm.updateBgmEnabled(true);
        await bgmPlayTrack(localIndex);
        return;
      }
      if (!sm.bgmState.enabled) await sm.updateBgmEnabled(true);
      sm.bgmState.onlineQueue = list;
      sm.bgmState.onlineIndex = index;
      if (track.sourceAccount && !track.sharedStream) {
        track = await roomSocial.resolveSharedTrack(track);
        list[index] = track;
        sm.bgmState.onlineQueue = [...list];
      }
      await bgmPlayOnlineTrack(track);
      // 歌词拉取由歌词胶囊监听 onlineTrack 变化完成
    } catch (e) {
      console.warn('在线播放失败:', e);
      const message = String(e?.message || e);
      snackError(t('play_failed'), `${track.title || '歌曲'}：${message}`);
      sm.pushLog(`[在线音乐] ${track.title ?? ''} 播放失败：${message}`);
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
      sourcePeer = 'local';
      playlists = [];
      playlistTracks = [];
      selectedPlaylist = null;
      searchError = '';
      if (p === 'local') {
        status = { loggedIn: true, nickname: '本地文件' };
        results = localTracks();
        return;
      }
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

  $effect(() => {
    if (sourcePeer !== 'local' && !selectedSharedAccount) sourcePeer = 'local';
  });

  onMount(() => {
    syncBgmSmtc();
    return () => stopPoll();
  });

  async function startListenTogether() {
    try {
      await roomListen.start();
    } catch (error) {
      snackError(t('listen_start_failed'), String(error?.message || error));
    }
  }

  const stateText = {
    loading: t('qr_loading'),
    waiting: t('qr_waiting'),
    scanned: t('qr_scanned'),
    success: t('login_success'),
    expired: t('qr_expired'),
    error: t('qr_error'),
    idle: '',
  };
</script>

<SubPage title={t('room_music')}>
  <div class="page-list">
    <!-- 平台切换 -->
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="album" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('music_platform')}</div>
          <div class="tile-subtitle">{t('music_platform_desc')}</div>
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

      {#if platform === 'local'}
        <div class="md-list-tile two-line no-cursor">
          <span class="tile-leading"><Icon name="audio_file" size={24} color="var(--md-primary)" /></span>
          <div class="tile-content">
            <div class="tile-title">{t('imported_count', { count: sm.bgmState.tracks.length })}</div>
            <div class="tile-subtitle">{`${AUDIO_EXTS.join(' / ')} · ${t('local_files_private')}`}</div>
          </div>
          <span class="tile-trailing">
            <button class="md-btn md-tonal-btn" disabled={importing} onclick={() => fileEl?.click()}>
              {importing ? t('importing') : t('import_music')}
            </button>
          </span>
        </div>
        <input type="file" accept={AUDIO_ACCEPT} multiple bind:this={fileEl} onchange={importLocalFiles} hidden />
      {:else if loginState.state !== 'idle' && loginState.state !== 'success'}
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
            <div class="tile-title">{status.nickname ? t('logged_in_as', { name: status.nickname }) : t('login_success')}</div>
            <div class="tile-subtitle">{t('music_platform_desc')}</div>
          </div>
          <span class="tile-trailing">
            <button class="md-btn md-text-btn" onclick={logout}>{t('logout')}</button>
          </span>
        </div>
      {:else if PLATFORMS.find((p) => p.key === platform)?.qr}
        <div class="qr-area qr-idle">
          <button class="md-btn md-tonal-btn" onclick={startLogin}>{t('qr_login')}</button>
        </div>
      {/if}
      {#if platform !== 'local' && status.loggedIn}
        <div class="md-divider"></div>
        <div class="md-switch-list-tile no-cursor">
          <div class="tile-content">
            <div class="tile-title">{t('share_account')}</div>
            <div class="tile-subtitle">{t('share_account_desc')}</div>
          </div>
          <Switch
            value={sm.bgmState.roomShare[platform] !== false}
            onChange={toggleShare}
          />
        </div>
      {/if}
    </SettingsCard>

    <div class="gap-16"></div>

    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="settings_remote" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('play_settings')}</div>
          <div class="tile-subtitle">{t('play_settings_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>
      <div class="md-switch-list-tile no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('smtc')}</div>
          <div class="tile-subtitle">{t('smtc_desc')}</div>
        </div>
        <Switch value={sm.bgmState.smtcEnabled} onChange={toggleSmtc} />
      </div>
      <div class="md-switch-list-tile no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('lyric_island')}</div>
          <div class="tile-subtitle">{t('lyric_island_desc')}</div>
        </div>
        <Switch value={sm.bgmState.lyricCapsuleEnabled} onChange={(value) => sm.updateLyricCapsuleEnabled(value)} />
      </div>
      <div class="volume-setting">
        <Icon name="volume_up" size={22} color="var(--md-primary)" />
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          value={sm.bgmState.volume}
          oninput={(e) => updateVolume(Number(e.currentTarget.value))}
          aria-label={t('volume')}
        />
        <span>{sm.bgmState.volume}</span>
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    {#if roomSocial.connected}
      <SettingsCard>
        {#if roomListen.incoming}
          <div class="listen-invite">
            <div class="listen-copy">
              <strong>{t('listen_invite_title', { name: roomListen.incoming.hostName })}</strong>
              <span>{t('listen_invite_desc')}</span>
            </div>
            <div class="listen-actions">
              <button class="md-btn md-filled-btn" onclick={() => roomListen.accept()}>{t('accept')}</button>
              <button class="md-btn md-text-btn" onclick={() => roomListen.decline()}>{t('ignore')}</button>
            </div>
          </div>
        {:else if roomListen.active}
          <div class="listen-invite">
            <div class="listen-copy">
              <strong>{roomListen.isHost ? t('host_listen') : t('listen_with', { name: roomListen.hostName })}</strong>
              <span>{roomListen.isHost ? t('members_joined', { count: roomListen.members.length }) : t('listen_sync_hint')}</span>
            </div>
            <button class="md-btn md-text-btn" onclick={() => roomListen.stop()}>{t('leave_listen')}</button>
          </div>
        {:else}
          <div class="listen-invite">
            <div class="listen-copy">
              <strong>{t('listen_together')}</strong>
              <span>{t('listen_desc')}</span>
            </div>
            <button class="md-btn md-tonal-btn" onclick={startListenTogether}>
              <Icon name="group_add" size={18} />{t('start_listen')}
            </button>
          </div>
        {/if}
        {#if roomListen.error}<div class="search-error">{roomListen.error}</div>{/if}
      </SettingsCard>
      <div class="gap-16"></div>
    {/if}

    <!-- 搜索 -->
    <SettingsCard>
      {#if platform !== 'local' && roomSocial.connected}
        <div class="account-source">
          <span class="source-label">{t('account_source')}</span>
          <select bind:value={sourcePeer} aria-label={t('account_source')}>
            <option value="local">{status.nickname ? t('my_account') + `（${status.nickname}）` : t('guest_account')}</option>
            {#each sharedAccounts as account (account.peer)}
              <option value={account.peer}>{t('shared_by', { name: account.accountName, owner: account.owner })}</option>
            {/each}
          </select>
          <span class="source-status">{sharedAccounts.length ? t('shared_accounts_count', { count: sharedAccounts.length }) : t('no_shared_accounts')}</span>
        </div>
        <div class="md-divider"></div>
      {/if}
      <div class="search-row">
        <input
          class="search-input"
          type="text"
          placeholder={`${PLATFORMS.find((p) => p.key === platform)?.label || ''} · ${t('search_music')}`}
          bind:value={query}
          onkeydown={(e) => {
            if (e.key === 'Enter') doSearch();
          }}
        />
        <button class="md-btn md-tonal-btn" onclick={doSearch} disabled={searching || !query.trim()}>
          {t('search')}
        </button>
      </div>
      {#if searchError}
        <div class="search-error">{searchError}</div>
      {/if}
      {#if results.length}
        <div class="track-grid">
          {#each results as track, index (track.key)}
            <MusicTrackCard
              {track}
              sourceLabel={track.sharedAccountName || track.sourceAccount?.accountName || ''}
              onPlay={() => playFrom(results, index)}
              onDelete={platform === 'local' ? deleteLocal : null}
            />
          {/each}
        </div>
      {:else if platform === 'local'}
        <div class="local-empty">{t('local_empty')}</div>
      {/if}
    </SettingsCard>

    <div class="gap-16"></div>

    <!-- 歌单（网易云登录后） -->
    {#if platform === 'netease' && status.loggedIn && playlists.length}
      <SettingsCard>
        <div class="md-list-tile two-line no-cursor">
          <span class="tile-leading"><Icon name="queue_music" size={24} color="var(--md-on-surface-variant)" /></span>
          <div class="tile-content">
            <div class="tile-title">{t('my_playlists')}</div>
            <div class="tile-subtitle">{t('playlist_hint')}</div>
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
                <div class="track-artist">{t('track_count', { count: p.count })}</div>
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
            <div class="track-grid">
              {#each playlistTracks as track, index (track.key)}
                <MusicTrackCard {track} onPlay={() => playFrom(playlistTracks, index)} />
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
    flex-wrap: wrap;
    padding: 12px 16px 4px;
    gap: 8px;
  }
  .platform-tab {
    flex: 1 1 100px;
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
  .listen-invite { display: flex; align-items: center; justify-content: space-between; gap: 14px; padding: 14px 16px; }
  .listen-copy { min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .listen-copy strong { font-size: 14px; }
  .listen-copy span { color: var(--md-on-surface-variant); font-size: 11px; line-height: 17px; }
  .listen-actions { display: flex; gap: 6px; flex-shrink: 0; }
  @media (max-width: 560px) {
    .listen-invite { align-items: flex-start; flex-direction: column; }
  }
  .account-source {
    display: grid;
    grid-template-columns: auto minmax(180px, 1fr) auto;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
  }
  .source-label { font-size: 13px; font-weight: 600; }
  .account-source select {
    min-width: 0;
    height: 38px;
    padding: 0 34px 0 12px;
    border: 1px solid var(--md-outline-variant);
    border-radius: 12px;
    background: var(--md-surface-container-low);
    color: var(--md-on-surface);
    outline: none;
  }
  .account-source select:focus { border-color: var(--md-primary); }
  .source-status { font-size: 11px; color: var(--md-on-surface-variant); }
  @media (max-width: 640px) {
    .account-source { grid-template-columns: 1fr; gap: 6px; }
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
  .volume-setting {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr) 36px;
    align-items: center;
    gap: 12px;
    padding: 12px 18px 16px;
  }
  .volume-setting input { width: 100%; accent-color: var(--md-primary); }
  .volume-setting span { text-align: right; font-variant-numeric: tabular-nums; }
  .track-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(156px, 1fr));
    gap: 14px;
    padding: 14px;
  }
  .local-empty { padding: 28px 16px; color: var(--md-on-surface-variant); text-align: center; font-size: 13px; }
  @media (max-width: 520px) {
    .track-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; padding: 10px; }
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
