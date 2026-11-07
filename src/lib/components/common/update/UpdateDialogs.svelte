<script>
  /**
   * 更新对话框宿主 —— 对照 update_check_ui.dart / update_dialogs.dart / update_download_ui.dart。
   * 职责：
   *  1. 监听 MainScreen 启动自动检查派发的 'astral-update-result' 事件（静默模式：
   *     失败不弹窗，成功/最新都弹结果对话框）。
   *  2. 提供 handleDownload()：GitHub 下载流程（加速选路 + 进度对话框 + 完成安装引导）。
   */
  import Dialog from '../Dialog.svelte';
  import Icon from '../Icon.svelte';
  import Progress from '../Progress.svelte';
  import { sm } from '../../../services/ServiceManager.svelte';
  import { openExternal } from '../../../rust/Api.svelte';
  import { isAndroid, isWindows, isTauri } from '../../../utils/Platform.svelte';
  import { snackError } from '../SnackbarStore.svelte';

  /* ---------- 自动检查结果对话框 ---------- */
  let resultOpen = $state(false);
  let result = $state(null);

  function onAutoCheckResult(e) {
    const r = e.detail;
    if (!r || r.kind === 'failed') return; // 静默失败不弹窗
    result = r;
    resultOpen = true;
  }

  $effect(() => {
    window.addEventListener('astral-update-result', onAutoCheckResult);
    return () => window.removeEventListener('astral-update-result', onAutoCheckResult);
  });

  const isLatest = $derived(result?.kind === 'upToDate');

  /* ---------- 下载进度对话框 ---------- */
  let downloadOpen = $state(false);
  let dlProgress = $state(null);
  let dlStatus = $state('正在准备下载…');
  let dlWorking = $state(true);
  let dlCancelled = $state(false);
  let dlDone = $state(false);
  let dlError = $state(null);
  let dlFileName = $state('');

  function platformFileName() {
    if (isAndroid()) return 'astral-arm64-v8a.apk';
    if (isWindows()) return 'astral-windows-x64-setup.exe';
    return '';
  }

  async function handleDownload(releaseInfo) {
    const fileName = platformFileName();
    if (!fileName) {
      snackError('下载失败', '未找到当前平台的安装包');
      return;
    }
    const asset = (releaseInfo?.assets || []).find((a) => a.name === fileName);
    if (!asset?.browser_download_url) {
      snackError('下载失败', `未找到 ${fileName} 的下载链接`);
      return;
    }
    dlFileName = fileName;
    dlProgress = null;
    dlStatus = '正在准备下载…';
    dlWorking = true;
    dlCancelled = false;
    dlDone = false;
    dlError = null;
    downloadOpen = true;

    try {
      /* 1. 选择下载线路 */
      dlStatus = '正在选择下载线路…';
      const url = await sm.resolveAcceleratedUrl(asset.browser_download_url);

      /* 2. 下载（流式进度） */
      dlStatus = '正在下载更新…';
      dlProgress = 0;
      const resp = await fetch(url);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const total = Number(resp.headers.get('content-length')) || null;
      const reader = resp.body.getReader();
      const chunks = [];
      let received = 0;
      for (;;) {
        if (dlCancelled) throw new Error('下载已取消');
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
        received += value.length;
        if (total) dlProgress = received / total;
      }
      if (dlCancelled) throw new Error('下载已取消');

      /* 3. 保存（浏览器：Blob 下载；Tauri：交给系统） */
      const blob = new Blob(chunks);
      const blobUrl = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = blobUrl;
      a.download = fileName;
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(blobUrl), 60000);

      dlWorking = false;
      dlDone = true;
    } catch (e) {
      if (dlCancelled) {
        dlWorking = false;
        dlError = '下载已取消';
        return;
      }
      dlWorking = false;
      dlError = `下载失败: ${e.message || e}`;
    }
  }

  function cancelDownload() {
    dlCancelled = true;
    dlWorking = false;
    dlError = '下载已取消';
  }

  function installFile() {
    downloadOpen = false;
    snackError(
      '无法打开安装文件',
      '请从浏览器下载目录手动运行安装程序\n请确保已开启「允许安装未知来源应用」权限'
    );
  }
</script>

<!-- 启动自动检查结果对话框 -->
<Dialog bind:open={resultOpen} title={isLatest ? (result?.version || '') : `发现新版本: ${result?.version || ''}`} dismissible={false}>
  <div class="update-content">
    {#if !isLatest}
      <div class="md-body-medium" style="margin-bottom:8px">更新内容:</div>
    {/if}
    <div class="md-body-medium update-notes">{result?.releaseNotes || ''}</div>
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (resultOpen = false)}>稍后再说</button>
    {#if !isLatest}
      <button
        class="md-btn md-text-btn"
        onclick={() => {
          resultOpen = false;
          openExternal('https://astral.fan/quick-start/download-install/');
        }}>网盘下载</button>
      <button
        class="md-btn md-filled-btn"
        onclick={() => {
          const info = result?.releaseInfo;
          resultOpen = false;
          if (info) handleDownload(info);
        }}>GitHub下载</button>
    {:else}
      <button class="md-btn md-filled-btn" onclick={() => (resultOpen = false)}>确定</button>
    {/if}
  {/snippet}
</Dialog>

<!-- 下载进度对话框 -->
<Dialog bind:open={downloadOpen} title={dlWorking ? '正在下载更新' : dlError ? '下载失败' : '下载完成'} dismissible={false}>
  <div class="dl-content">
    {#if dlWorking}
      <Progress type="linear" value={dlProgress} />
      <div style="height:16px"></div>
      <div class="md-body-medium">{dlStatus}</div>
      {#if dlProgress != null}
        <div style="height:8px"></div>
        <div class="md-body-medium">下载进度: {(dlProgress * 100).toFixed(1)}%</div>
      {/if}
    {:else if dlError}
      <div class="md-body-medium">{dlError}</div>
    {:else}
      <div class="dl-done">
        <Icon name="check_circle" size={48} color="var(--flutter-green)" />
      </div>
      <div style="height:16px"></div>
      <div class="md-body-medium">文件已下载到: {dlFileName}</div>
      <div style="height:8px"></div>
      <div class="md-body-medium">是否立即安装？</div>
    {/if}
  </div>
  {#snippet actions()}
    {#if dlWorking}
      <button class="md-btn md-text-btn" onclick={cancelDownload}>取消下载</button>
    {:else}
      <button class="md-btn md-text-btn" onclick={() => (downloadOpen = false)}>取消</button>
      {#if dlDone}
        <button class="md-btn md-filled-btn" onclick={installFile}>立即安装</button>
      {:else if dlError}
        <button class="md-btn md-filled-btn" onclick={() => (downloadOpen = false)}>确定</button>
      {/if}
    {/if}
  {/snippet}
</Dialog>

<style>
  .update-content {
    max-height: 50vh;
    overflow-y: auto;
  }
  .update-notes {
    white-space: pre-wrap;
    word-break: break-word;
    max-width: 440px;
  }
  .dl-content {
    min-width: 320px;
  }
  .dl-done {
    display: flex;
    justify-content: center;
  }
</style>
