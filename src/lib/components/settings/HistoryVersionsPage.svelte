<script>
  /**
   * HistoryVersionsPage —— 对应 history_versions_page.dart。
   * GET https://astral.fan/downloads.json → [{version,url,date}]，四态：加载/失败/空/列表。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Progress from '$lib/components/common/Progress.svelte';
  import { t } from '$lib/i18n/index.svelte.js';
  import { openExternal } from '$lib/rust/api.js';
  import { snackError } from '$lib/components/common/snackbars.svelte.js';

  let versions = $state([]);
  let isLoading = $state(true);
  let errorMessage = $state(null);

  $effect(() => {
    loadVersions();
  });

  async function loadVersions() {
    isLoading = true;
    errorMessage = null;
    try {
      const response = await fetch('https://astral.fan/downloads.json');
      if (response.ok) {
        const jsonList = await response.json();
        versions = Array.isArray(jsonList)
          ? jsonList.map((item) => ({
              version: String(item?.version ?? ''),
              url: String(item?.url ?? ''),
              date: String(item?.date ?? ''),
            }))
          : [];
        isLoading = false;
      } else {
        errorMessage = `加载失败: HTTP ${response.status}`;
        isLoading = false;
      }
    } catch (e) {
      errorMessage = `加载失败: ${e}`;
      isLoading = false;
    }
  }

  async function launchUrl(url) {
    const ok = await openExternal(url).catch(() => false);
    if (!ok) {
      snackError('无法打开链接', url);
    }
  }
</script>

<SubPage title={t('history_versions')}>
  {#snippet actions()}
    <button class="md-icon-btn" data-tip="刷新" onclick={loadVersions}>
      <Icon name="refresh" size={24} color="var(--md-on-surface)" />
    </button>
  {/snippet}

  {#if isLoading}
    <div class="center-state">
      <Progress type="circular" />
    </div>
  {:else if errorMessage != null}
    <div class="center-state">
      <Icon name="error_outline" size={64} color="var(--flutter-red)" />
      <div class="gap-16"></div>
      <div class="error-text">{errorMessage}</div>
      <div class="gap-16"></div>
      <button class="md-btn md-elevated-btn" onclick={loadVersions}>
        <Icon name="refresh" size={18} />
        重试
      </button>
    </div>
  {:else if versions.length === 0}
    <div class="center-state">
      <div class="empty-text">暂无历史版本</div>
    </div>
  {:else}
    <div class="page-list">
      {#each versions as version, index (index)}
        <div class="md-card version-card">
          <div class="md-list-tile two-line" onclick={() => launchUrl(version.url)}>
            <span class="tile-leading">
              <span class="avatar">{index + 1}</span>
            </span>
            <div class="tile-content">
              <div class="version-title">{version.version}</div>
              <div class="tile-subtitle">{version.date}</div>
            </div>
            <span class="tile-trailing"><Icon name="download" size={24} /></span>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</SubPage>

<style>
  .center-state {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 16px;
    box-sizing: border-box;
  }
  .gap-16 {
    height: 16px;
  }
  .error-text {
    font-size: 16px;
    color: var(--md-on-surface);
    text-align: center;
    word-break: break-all;
  }
  .empty-text {
    font-size: 16px;
    color: var(--md-on-surface);
  }
  .page-list {
    padding: 16px;
  }
  .version-card {
    margin-bottom: 12px;
  }
  /* CircleAvatar 默认（M3）：40×40 primaryContainer */
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 700;
  }
  .version-title {
    font-size: 16px;
    line-height: 24px;
    font-weight: 600;
    color: var(--md-on-surface);
  }
</style>
