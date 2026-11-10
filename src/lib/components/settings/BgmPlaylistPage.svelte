<script>
  /**
   * 背景音乐歌单页 —— 软件设置 → 背景音乐 → 打开歌单。
   * 功能：导入本地音频（分块写入 bgm/ 目录）、上下拖拽改顺序、
   * 点击切换播放（点当前曲目 = 播放/暂停）、删除。
   * 播放控制走 BgmPlayer 单例；列表数据在 sm.bgmState.tracks。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { bgmPlayTrack, bgmToggle } from '$lib/services/BgmPlayer.svelte';
  import { isTauri } from '$lib/utils/Platform.svelte';
  import { snackError, snackInfo } from '$lib/components/common/SnackbarStore.svelte';

  const tracks = $derived(sm.bgmState.tracks);
  const currentTrackId = $derived(sm.bgmState.trackId);
  const currentIndex = $derived(
    sm.bgmState.tracks.findIndex((t) => t.trackId === currentTrackId)
  );

  /* ---------- 导入 ---------- */
  const AUDIO_EXTS = ['mp3', 'mp4', 'm4a', 'aac', 'flac', 'wav', 'ogg', 'oga', 'opus', 'weba', 'webm'];
  const ACCEPT = 'audio/*,' + AUDIO_EXTS.map((e) => '.' + e).join(',');
  const BG_CHUNK = 1 << 20; // 1MB 分块（与自定义背景同款 IPC 负载）
  let fileEl = null;
  let importing = $state(false);

  function chooseFiles() {
    fileEl?.click();
  }

  function blobToB64(blob) {
    return new Promise((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => resolve(String(r.result).split(',')[1] || '');
      r.onerror = () => reject(r.error);
      r.readAsDataURL(blob);
    });
  }

  async function onFilesChosen(e) {
    const files = Array.from(e.target.files || []);
    e.target.value = ''; // 允许重复选择同一文件
    if (!files.length || importing) return;
    if (!isTauri()) {
      snackError('导入失败', '浏览器预览无后端，请使用桌面端');
      return;
    }
    importing = true;
    let ok = 0;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      for (const file of files) {
        try {
          const ext = (file.name.split('.').pop() || '').toLowerCase();
          if (!AUDIO_EXTS.includes(ext)) {
            snackError('不支持的格式', `${file.name}（支持 ${AUDIO_EXTS.join(' / ')}）`);
            continue;
          }
          const id = await invoke('bgm_write_begin', { ext });
          for (let off = 0; off < file.size; off += BG_CHUNK) {
            const b64 = await blobToB64(file.slice(off, Math.min(off + BG_CHUNK, file.size)));
            await invoke('bgm_write_append', { id, b64 });
          }
          const info = await invoke('bgm_write_end', { id });
          await sm.addBgmTrack({
            trackId: info.id,
            name: file.name.replace(/\.[^.]+$/, '') || file.name,
            ext: info.ext,
          });
          ok += 1;
        } catch (err) {
          snackError('导入失败', `${file.name}: ${err?.message ?? err}`);
        }
      }
    } finally {
      importing = false;
    }
    if (ok) {
      snackInfo('导入完成', `已导入 ${ok} 个音频文件`);
      // 首次导入自动选中第一首（不自动出声，点击/快捷键播放）
      if (sm.bgmState.trackId == null && sm.bgmState.tracks.length) {
        sm.bgmState.trackId = sm.bgmState.tracks[0].trackId;
        sm.persistBgmTrackId(sm.bgmState.trackId).catch(() => {});
      }
    }
  }

  /* ---------- 拖拽排序（ServersPage 预览语义：拖拽中实时预览，松手落库） ---------- */
  let dragIndex = $state(null);
  let previewOrder = $state(null);
  let dropped = false;

  const displayTracks = $derived(previewOrder ?? tracks);

  function onDragStart(e, index) {
    dragIndex = index;
    dropped = false;
    previewOrder = [...displayTracks];
    e.dataTransfer.effectAllowed = 'move';
    try {
      e.dataTransfer.setData('text/plain', String(index));
    } catch {
      /* 某些环境不允许 setData */
    }
  }

  function applyPreview(insertPos) {
    if (dragIndex == null || insertPos == null) return;
    const cur = [...displayTracks];
    let target = insertPos > dragIndex ? insertPos - 1 : insertPos;
    if (target < 0) target = 0;
    if (target === dragIndex) return;
    const [item] = cur.splice(dragIndex, 1);
    cur.splice(target, 0, item);
    previewOrder = cur;
    dragIndex = target;
  }

  function onListDragOver(e) {
    if (dragIndex == null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    const rows = Array.from(e.currentTarget.querySelectorAll('.bgm-row'));
    let insertPos = rows.length;
    for (let i = 0; i < rows.length; i++) {
      const r = rows[i].getBoundingClientRect();
      if (e.clientY < r.top + r.height / 2) {
        insertPos = i;
        break;
      }
    }
    applyPreview(insertPos);
  }

  function onListDrop(e) {
    if (dragIndex == null) return;
    e.preventDefault();
    dropped = true;
  }

  function onDragEnd() {
    if (dragIndex == null) return;
    if (dropped && previewOrder) sm.reorderBgmTracks(previewOrder);
    dragIndex = null;
    previewOrder = null;
    dropped = false;
  }

  /* ---------- 点击切歌 / 删除 ---------- */
  function onRowClick(index) {
    const t = displayTracks[index];
    if (!t) return;
    if (t.trackId === currentTrackId) {
      bgmToggle(); // 点当前曲目 = 播放/暂停
    } else {
      bgmPlayTrack(index); // 预览顺序与真实顺序在松手前一致
    }
  }

  async function onDelete(e, track) {
    e.stopPropagation();
    await sm.removeBgmTrack(track);
  }
</script>

<SubPage title="背景音乐歌单">
  {#snippet actions()}
    <button class="md-icon-btn" data-tip="导入音频" disabled={importing} onclick={chooseFiles}>
      <Icon name="add" size={24} color="var(--md-on-surface)" />
    </button>
  {/snippet}

  <div class="page-list">
    {#if displayTracks.length === 0}
      <div class="empty">
        <Icon name="music_note" size={64} color="var(--md-outline)" />
        <div class="gap-16"></div>
        <div class="empty-text">歌单还是空的</div>
        <div class="gap-16"></div>
        <button class="md-btn md-elevated-btn" onclick={chooseFiles} disabled={importing}>
          <Icon name="add" size={18} />
          导入音频
        </button>
        <div class="empty-hint">支持 MP3 / MP4 / M4A / FLAC / WAV / OGG 等</div>
      </div>
    {:else}
      <div class="tip-row">
        拖拽调整顺序 · 点击切换播放 · 导入右上角
      </div>
      <div class="track-list" ondragover={onListDragOver} ondrop={onListDrop}>
        {#each displayTracks as t, index (t.trackId)}
          <div
            class="bgm-row {index === dragIndex ? 'dragging' : ''} {t.trackId === currentTrackId ? 'current' : ''}"
            draggable="true"
            role="button"
            tabindex="0"
            ondragstart={(e) => onDragStart(e, index)}
            ondragend={onDragEnd}
            onclick={() => onRowClick(index)}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onRowClick(index);
              }
            }}
          >
            <span class="row-icon">
              {#if t.trackId === currentTrackId && sm.bgmState.playing}
                <span class="ms filled" style="font-size:22px; color:var(--md-primary)">graphic_eq</span>
              {:else if t.trackId === currentTrackId}
                <span class="ms filled" style="font-size:22px; color:var(--md-primary)">pause_circle</span>
              {:else}
                <span class="ms" style="font-size:22px; color:var(--md-on-surface-variant)">music_note</span>
              {/if}
            </span>
            <div class="row-main">
              <div class="row-name ellipsis">{t.name}</div>
              <div class="row-sub">
                {t.ext.toUpperCase()}
                {#if t.trackId === currentTrackId}
                  · {sm.bgmState.playing ? '正在播放' : '已暂停'}
                {/if}
              </div>
            </div>
            <button class="md-icon-btn row-del" data-tip="删除" onclick={(e) => onDelete(e, t)}>
              <Icon name="delete" size={20} />
            </button>
          </div>
        {/each}
      </div>
      <div class="bottom-spacer"></div>
    {/if}
  </div>

  <input type="file" accept={ACCEPT} multiple bind:this={fileEl} onchange={onFilesChosen} hidden />
</SubPage>

<style>
  .page-list {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 16px;
    box-sizing: border-box;
  }
  .tip-row {
    padding: 0 4px 10px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-06);
  }
  .track-list {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .bgm-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 60px;
    padding: 8px 12px;
    margin-bottom: 8px;
    border-radius: 12px;
    background: var(--md-surface-container-low);
    border: 1px solid transparent;
    cursor: pointer;
    user-select: none;
  }
  .bgm-row:hover {
    border-color: var(--md-primary);
  }
  .bgm-row.current {
    border-color: var(--md-primary);
    background: color-mix(in srgb, var(--md-surface-container-low) 88%, var(--md-primary) 12%);
  }
  .bgm-row.dragging {
    opacity: 0.35;
  }
  .row-icon {
    display: inline-flex;
    flex-shrink: 0;
  }
  .row-main {
    flex: 1 1 0;
    min-width: 0;
  }
  .row-name {
    font-size: 15px;
    line-height: 22px;
    font-weight: 500;
    color: var(--md-on-surface);
  }
  .row-sub {
    margin-top: 2px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
  }
  .row-del {
    flex-shrink: 0;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .empty-text {
    font-size: 16px;
    color: var(--md-outline);
  }
  .empty-hint {
    margin-top: 8px;
    font-size: 12px;
    color: var(--md-on-surface-06);
  }
  .gap-16 {
    height: 16px;
  }
  .bottom-spacer {
    height: 16px;
  }
</style>
