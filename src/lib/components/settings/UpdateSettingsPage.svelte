<script>
  /**
   * UpdateSettingsPage —— 对应 update_settings_page.dart + update_settings_actions.dart +
   * update_check_ui.dart / update_dialogs.dart（UpdateBusyDialog / UpdateDialog 内联实现）。
   * 'GitHub下载' 在桌面端用 <a> 触发下载（原版 UpdateDownloadUi 的简化，按规格）。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import Progress from '$lib/components/common/Progress.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { t } from '$lib/i18n/I18n.svelte';
  import { pushPage } from '$lib/router/PageRouter.svelte';
  import { openExternal } from '$lib/rust/Api.svelte';
  import { snackError } from '$lib/components/common/SnackbarStore.svelte';
  import { isAndroid, isWindows, isTauri } from '$lib/utils/Platform.svelte';
  import HistoryVersionsPage from './HistoryVersionsPage.svelte';

  /* GitHubProxySelector 常量 */
  const MIRRORS = ['https://gh.xmly.dev/', 'https://gh-proxy.org/', 'https://ghfast.top/'];
  const NET_DISK_URL = 'https://astral.fan/quick-start/download-install/';
  const RELEASES_PAGE = 'https://github.com/ldoubil/astral/releases';

  /* SwitchListTile 整行可点；点 Switch 本体时避免与 tile 冒泡双触发 */
  function switchTileClick(e, current, update) {
    if (e.target.closest('.md-switch')) return;
    update(!current);
  }

  /* ============ 更新检查（UpdateCheckUi.checkAndPresent） ============ */
  let busyOpen = $state(false);
  let resultOpen = $state(false);
  let result = $state(null);

  async function checkForUpdates(forceShowDownload = false) {
    busyOpen = true;
    const r = await sm.checkForUpdates();
    busyOpen = false;

    if (r.kind === 'failed') {
      result = {
        isLatest: false,
        hasRelease: false,
        title: (r.error || '').startsWith('检查更新时发生错误') ? '更新检查失败' : '检查更新失败',
        notes: r.error || '无法获取最新版本信息',
        releasePage: RELEASES_PAGE,
        releaseInfo: null,
      };
    } else if (r.kind === 'upToDate' && !forceShowDownload) {
      result = {
        isLatest: true,
        hasRelease: false,
        title: '当前已是最新版本',
        notes: `当前版本为: ${sm.appVersion}`,
        releasePage: RELEASES_PAGE,
        releaseInfo: null,
      };
    } else {
      /* updateAvailable，或「重新下载」强制显示下载流程 */
      result = {
        isLatest: false,
        hasRelease: r.releaseInfo != null,
        title: `发现新版本: ${r.version}`,
        notes: r.releaseNotes,
        releasePage: r.releasePage,
        releaseInfo: r.releaseInfo,
      };
    }
    resultOpen = true;
  }

  /* ============ 下载（<a> 触发，UpdateDownloader 等价） ============ */
  function platformFileName() {
    if (isAndroid()) return 'astral-arm64-v8a.apk';
    if (isWindows()) return 'astral-windows-x64-setup.exe';
    return '';
  }

  function getDownloadUrlForFile(releaseInfo, fileName) {
    const assets = releaseInfo?.assets;
    if (!Array.isArray(assets)) return null;
    for (const asset of assets) {
      if (asset?.name === fileName) return asset.browser_download_url ?? null;
    }
    return null;
  }

  async function triggerDownload(releaseInfo) {
    const fileName = platformFileName();
    if (!fileName) {
      snackError('下载失败', '当前平台暂不支持内置下载安装');
      return;
    }
    const url = getDownloadUrlForFile(releaseInfo, fileName);
    if (!url) {
      snackError('下载失败', `未找到 ${fileName} 的下载链接`);
      return;
    }
    const accelerated = await sm.resolveAcceleratedUrl(url);
    const a = document.createElement('a');
    a.href = accelerated;
    a.download = fileName;
    a.target = '_blank';
    a.rel = 'noopener';
    document.body.appendChild(a);
    a.click();
    a.remove();
  }

  function netDiskDownload() {
    resultOpen = false;
    openExternal(NET_DISK_URL);
  }

  function githubDownload() {
    resultOpen = false;
    triggerDownload(result?.releaseInfo);
  }

  function manualDownload() {
    resultOpen = false;
    openExternal(result?.releasePage || RELEASES_PAGE);
  }

  /* ============ 版本信息对话框 ============ */
  let versionOpen = $state(false);

  /* ============ 下载加速编辑对话框 ============ */
  let accelOpen = $state(false);
  let accelMode = $state('auto');
  let accelInput = $state('');
  let probing = $state(false);
  let probeResult = $state(null);

  /* downloadAccelerateDescription() 等价 */
  const accelerateDesc = $derived.by(() => {
    const setting = sm.updateState.downloadAccelerate ?? '';
    const trimmed = setting.trim();
    if (trimmed === '') return t('download_acceleration_disabled');
    if (trimmed === 'auto') {
      const resolved = sm.updateState.resolvedDownloadAccelerate;
      if (resolved != null && resolved !== '') {
        return t('download_acceleration_auto_current', { mirror: resolved });
      }
      return t('download_acceleration_auto_pending');
    }
    return setting;
  });

  function normalizePrefix(prefix) {
    const trimmed = prefix.trim();
    if (trimmed === '') return trimmed;
    return trimmed.endsWith('/') ? trimmed : `${trimmed}/`;
  }

  async function probeMirror(prefix) {
    const start = performance.now();
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 6000);
    try {
      await fetch(prefix + 'ldoubil/astral', {
        method: 'HEAD',
        signal: controller.signal,
        mode: 'no-cors',
      });
      clearTimeout(timer);
      return performance.now() - start;
    } catch {
      clearTimeout(timer);
      return Infinity;
    }
  }

  /* GitHubProxySelector.selectFastest(forceRefresh: true) 等价 */
  async function selectFastestMirror() {
    const times = await Promise.all(MIRRORS.map(probeMirror));
    let best = null;
    let bestTime = Infinity;
    for (let i = 0; i < MIRRORS.length; i++) {
      if (times[i] < bestTime) {
        bestTime = times[i];
        best = MIRRORS[i];
      }
    }
    return best;
  }

  async function runProbe() {
    if (probing) return;
    probing = true;
    probeResult = null;
    const prefix = await selectFastestMirror();
    probing = false;
    probeResult = prefix ?? t('download_acceleration_probe_failed');
    if (prefix != null) {
      sm.updateState.resolvedDownloadAccelerate = prefix;
    }
  }

  function openAccelDialog() {
    const current = sm.updateState.downloadAccelerate ?? '';
    const trimmed = current.trim();
    accelMode = trimmed === '' ? 'off' : trimmed === 'auto' ? 'auto' : 'manual';
    accelInput = accelMode === 'manual' ? trimmed : '';
    probing = false;
    probeResult = null;
    accelOpen = true;
  }

  async function saveAccelerate() {
    if (accelMode === 'auto') {
      await sm.updateDownloadAccelerate('auto');
    } else if (accelMode === 'manual') {
      const value = accelInput.trim();
      const normalized = normalizePrefix(value === '' ? MIRRORS[0] : value);
      await sm.updateDownloadAccelerate(normalized);
    } else {
      await sm.updateDownloadAccelerate('');
      sm.updateState.resolvedDownloadAccelerate = null;
    }
    accelOpen = false;
  }
</script>

<SubPage title={t('update_settings')}>
  {#snippet actions()}
    <button
      class="md-icon-btn"
      data-tip={t('check_update')}
      onclick={() => checkForUpdates(false)}
    >
      <Icon name="refresh" size={24} color="var(--md-on-surface)" />
    </button>
  {/snippet}

  <div class="page-list">
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="system_update" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('update_settings')}</div>
          <div class="tile-subtitle">{t('update_behavior_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.updateState.beta, sm.updateBeta)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('join_beta')}</div>
          <div class="tile-subtitle">{t('join_beta_desc')}</div>
        </div>
        <Switch value={sm.updateState.beta} onChange={(v) => sm.updateBeta(v)} />
      </div>

      {#if !sm.updateState.beta}
        <div
          class="md-switch-list-tile"
          onclick={(e) =>
            switchTileClick(e, sm.updateState.autoCheckUpdate, sm.updateAutoCheckUpdate)}
        >
          <div class="tile-content">
            <div class="tile-title">{t('auto_update')}</div>
            <div class="tile-subtitle">{t('auto_update_desc')}</div>
          </div>
          <Switch value={sm.updateState.autoCheckUpdate} onChange={(v) => sm.updateAutoCheckUpdate(v)} />
        </div>
      {/if}

      <div class="md-divider"></div>

      <div class="md-list-tile two-line" onclick={openAccelDialog}>
        <span class="tile-leading"><Icon name="bolt" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('download_acceleration')}</div>
          <div class="tile-subtitle">{accelerateDesc}</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="update" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('update_operations')}</div>
          <div class="tile-subtitle">{t('update_operations_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div class="md-list-tile two-line" onclick={() => checkForUpdates(false)}>
        <span class="tile-leading"><Icon name="refresh" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('check_update')}</div>
          <div class="tile-subtitle">{t('check_update_available')}</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>

      <div class="md-list-tile two-line" onclick={() => (versionOpen = true)}>
        <span class="tile-leading"><Icon name="info_outline" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('version_info')}</div>
          <div class="tile-subtitle">{t('version_info_desc')}</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>

      <div class="md-list-tile two-line" onclick={() => pushPage(HistoryVersionsPage)}>
        <span class="tile-leading"><Icon name="history" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('history_versions')}</div>
          <div class="tile-subtitle">{t('history_versions_desc')}</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>

      <div class="md-list-tile two-line" onclick={() => checkForUpdates(true)}>
        <span class="tile-leading"><Icon name="cloud_download" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">重新下载</div>
          <div class="tile-subtitle">如果出现问题可以尝试重新下载！</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="help" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('update_description')}</div>
          <div class="tile-subtitle">{t('update_description_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="science" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('beta_version')}</div>
          <div class="tile-subtitle">{t('beta_version_desc')}</div>
        </div>
      </div>

      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="auto_awesome" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('auto_update_title')}</div>
          <div class="tile-subtitle">{t('auto_update_info_desc')}</div>
        </div>
      </div>
    </SettingsCard>
  </div>
</SubPage>

<!-- 检查更新忙态对话框（UpdateBusyDialog） -->
<Dialog bind:open={busyOpen} title="检查更新" dismissible={false}>
  <Progress type="linear" />
  <div class="gap-16"></div>
  <div class="busy-message">正在从 GitHub 获取版本信息…</div>
</Dialog>

<!-- 更新结果对话框（UpdateDialog） -->
{#if result}
  <Dialog bind:open={resultOpen} title={result.title} dismissible={false}>
    {#if !result.isLatest}
      <div>更新内容:</div>
      <div class="gap-8"></div>
    {/if}
    <div class="notes-body">{result.notes}</div>
    {#snippet actions()}
      <button class="md-btn md-text-btn" onclick={() => (resultOpen = false)}>稍后再说</button>
      {#if !result.isLatest && result.hasRelease}
        <button class="md-btn md-text-btn" onclick={netDiskDownload}>网盘下载</button>
        <button class="md-btn md-elevated-btn" onclick={githubDownload}>GitHub下载</button>
      {:else if !result.isLatest}
        <button class="md-btn md-elevated-btn" onclick={manualDownload}>手动下载</button>
      {:else}
        <button class="md-btn md-elevated-btn" onclick={() => (resultOpen = false)}>确定</button>
      {/if}
    {/snippet}
  </Dialog>
{/if}

<!-- 版本信息对话框 -->
<Dialog bind:open={versionOpen} title={t('version_info')}>
  <div>{t('current_version')}: {sm.appVersion}</div>
  <div class="gap-8"></div>
  <div>{t('update_channel')}: {sm.updateState.beta ? 'Beta' : 'Stable'}</div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (versionOpen = false)}>{t('close')}</button>
  {/snippet}
</Dialog>

<!-- 下载加速编辑对话框（editDownloadAccelerate） -->
<Dialog bind:open={accelOpen} title={t('download_acceleration_title')}>
  <div>{t('download_acceleration_info_desc')}</div>
  <div class="gap-12"></div>

  <div class="radio-tile" onclick={() => (accelMode = 'auto')} role="radio" aria-checked={accelMode === 'auto'}>
    <span class="md-radio {accelMode === 'auto' ? 'checked' : ''}"></span>
    <div class="radio-content">
      <div class="radio-title">{t('download_acceleration_auto')}</div>
      <div class="radio-subtitle mirrors">{MIRRORS.join('\n')}</div>
    </div>
  </div>

  <div class="radio-tile" onclick={() => (accelMode = 'manual')} role="radio" aria-checked={accelMode === 'manual'}>
    <span class="md-radio {accelMode === 'manual' ? 'checked' : ''}"></span>
    <div class="radio-content">
      <div class="radio-title">{t('download_acceleration_manual')}</div>
    </div>
  </div>

  {#if accelMode === 'manual'}
    <div class="manual-field">
      <TextField bind:value={accelInput} hint={t('download_acceleration_manual_hint')} />
    </div>
  {/if}

  <div class="radio-tile" onclick={() => (accelMode = 'off')} role="radio" aria-checked={accelMode === 'off'}>
    <span class="md-radio {accelMode === 'off' ? 'checked' : ''}"></span>
    <div class="radio-content">
      <div class="radio-title">{t('download_acceleration_disabled')}</div>
    </div>
  </div>

  {#if accelMode === 'auto'}
    <div class="gap-8"></div>
    <button class="md-btn md-outlined-btn probe-btn" disabled={probing} onclick={runProbe}>
      {#if probing}
        <Progress type="circular" size={16} strokeWidth={2} />
      {:else}
        <Icon name="speed" size={24} />
      {/if}
      {probing ? t('download_acceleration_probing') : t('download_acceleration_reprobe')}
    </button>
    {#if probeResult}
      <div class="gap-8"></div>
      <div class="probe-result">{probeResult}</div>
    {/if}
  {/if}

  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (accelOpen = false)}>{t('cancel')}</button>
    <button class="md-btn md-elevated-btn" onclick={saveAccelerate}>{t('save')}</button>
  {/snippet}
</Dialog>

<style>
  .page-list {
    padding: 16px;
  }
  .gap-8 {
    height: 8px;
  }
  .gap-12 {
    height: 12px;
  }
  .gap-16 {
    height: 16px;
  }
  .busy-message {
    text-align: left;
  }
  .notes-body {
    white-space: pre-wrap;
    word-break: break-word;
  }
  /* RadioListTile：AlertDialog 内容区内自带 16px 横向 contentPadding */
  .radio-tile {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 16px;
    cursor: pointer;
    min-height: 56px;
    box-sizing: border-box;
  }
  .radio-content {
    flex: 1 1 0;
    min-width: 0;
  }
  .radio-title {
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
  }
  .radio-subtitle {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
    margin-top: 2px;
  }
  .radio-subtitle.mirrors {
    font-size: 12px;
    line-height: 16px;
    white-space: pre-line;
  }
  .manual-field {
    margin: 0 0 4px 0;
  }
  .probe-result {
    font-size: 13px;
    line-height: 18px;
    color: var(--md-on-surface);
    word-break: break-all;
  }
</style>
