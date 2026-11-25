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
  import { sm } from '$lib/services/ServiceManager.svelte';
  import {
    bgmPlayOnlineTrack,
    bgmPlayTrack,
    bgApplyVolume,
    syncBgmSmtc,
  } from '$lib/services/BgmPlayer.svelte';
  import { isTauri } from '$lib/utils/Platform.svelte';
  import { onMount, untrack } from 'svelte';

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
            snackError('不支持的格式', file.name);
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
          snackError('导入失败', `${file.name}：${error?.message || error}`);
        }
      }
    } finally {
      importing = false;
      results = localTracks(query);
    }
    if (imported) snackInfo('导入完成', `已导入 ${imported} 首本地音乐`);
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
      if (!results.length) searchError = '没有找到匹配的歌曲';
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
      snackError('播放失败', `${track.title || '歌曲'}：${message}`);
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

<SubPage title="房间音乐">
  <div class="page-list">
    <!-- 平台切换 -->
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="album" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">音乐平台</div>
          <div class="tile-subtitle">本地音乐与在线平台统一播放；在线平台支持房间账号能力共享</div>
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
            <div class="tile-title">已导入 {sm.bgmState.tracks.length} 首本地音乐</div>
            <div class="tile-subtitle">支持 {AUDIO_EXTS.join(' / ')}，本地文件不会上传给房员</div>
          </div>
          <span class="tile-trailing">
            <button class="md-btn md-tonal-btn" disabled={importing} onclick={() => fileEl?.click()}>
              {importing ? '导入中…' : '导入音乐'}
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
      {/if}
      {#if platform !== 'local' && status.loggedIn}
        <div class="md-divider"></div>
        <div class="md-switch-list-tile no-cursor">
          <div class="tile-content">
            <div class="tile-title">向房间共享此账号的播放能力</div>
            <div class="tile-subtitle">房员仅看到账号名称；搜索与音频流由本机代理，不发送 Cookie 或登录凭证，也不会在本机播放</div>
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
          <div class="tile-title">播放与系统媒体控制</div>
          <div class="tile-subtitle">本地、在线与房员共享音乐共用以下设置</div>
        </div>
      </div>
      <div class="md-divider"></div>
      <div class="md-switch-list-tile no-cursor">
        <div class="tile-content">
          <div class="tile-title">系统媒体控制（SMTC）</div>
          <div class="tile-subtitle">向系统菜单和支持的第三方壁纸/媒体工具同步歌曲、封面、进度与播放控制</div>
        </div>
        <Switch value={sm.bgmState.smtcEnabled} onChange={toggleSmtc} />
      </div>
      <div class="md-switch-list-tile no-cursor">
        <div class="tile-content">
          <div class="tile-title">灵动岛歌词</div>
          <div class="tile-subtitle">单击岛屿展开播放控制；调音量时在同一岛尾部展开音量条</div>
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
          aria-label="音乐音量"
        />
        <span>{sm.bgmState.volume}</span>
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <!-- 搜索 -->
    <SettingsCard>
      {#if platform !== 'local' && roomSocial.connected}
        <div class="account-source">
          <span class="source-label">搜索与播放账号</span>
          <select bind:value={sourcePeer} aria-label="选择共享账号">
            <option value="local">我的账号{status.nickname ? `（${status.nickname}）` : ' / 游客'}</option>
            {#each sharedAccounts as account (account.peer)}
              <option value={account.peer}>{account.accountName} · {account.owner} 共享</option>
            {/each}
          </select>
          <span class="source-status">{sharedAccounts.length ? `${sharedAccounts.length} 个房员账号可用` : '暂无房员共享此平台'}</span>
        </div>
        <div class="md-divider"></div>
      {/if}
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
        <div class="local-empty">还没有本地音乐，点击上方“导入音乐”开始。</div>
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
