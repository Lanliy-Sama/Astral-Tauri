<script>
  /**
   * LogsPage —— 对应 logs_page.dart。
   * 自建 AppBar（SubPage actions：copy_all / clear_all）；FAB 滚动到底部（300ms easeOut）。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { writeClipboard } from '$lib/rust/api.js';
  import { snackInfo, snackSuccess } from '$lib/components/common/snackbars.svelte.js';

  let scrollEl = $state(null);
  let clearOpen = $state(false);

  const logs = $derived(sm.appSettingsState.logs);

  function logKind(log) {
    const lower = log.toLowerCase();
    if (lower.includes('error') || lower.includes('错误')) return 'error';
    if (lower.includes('warning') || lower.includes('警告')) return 'warning';
    return 'info';
  }

  async function copyAllLogs() {
    if (logs.length === 0) {
      snackInfo('提示', '暂无日志可复制');
      return;
    }
    await writeClipboard(logs.join('\n'));
    snackSuccess('复制成功', `已复制 ${logs.length} 条日志到剪贴板`);
  }

  async function copyOneLog(log) {
    await writeClipboard(log);
    snackSuccess('复制成功', '日志已复制到剪贴板', { duration: 1000 });
  }

  async function confirmClear() {
    clearOpen = false;
    sm.clearLogs();
    snackSuccess('已清空', '日志已清空');
  }

  /* animateTo(maxScrollExtent, 300ms, Curves.easeOut) */
  function scrollToBottom() {
    const el = scrollEl;
    if (!el) return;
    const start = el.scrollTop;
    const end = el.scrollHeight - el.clientHeight;
    const startTime = performance.now();
    const duration = 300;
    function step(now) {
      const p = Math.min(1, (now - startTime) / duration);
      const eased = 1 - Math.pow(1 - p, 3);
      el.scrollTop = start + (end - start) * eased;
      if (p < 1) requestAnimationFrame(step);
    }
    requestAnimationFrame(step);
  }
</script>

<SubPage title="日志" scrollable={false}>
  {#snippet actions()}
    <button class="md-icon-btn" data-tip="复制所有日志" onclick={copyAllLogs}>
      <Icon name="copy_all" size={24} color="var(--md-on-surface)" />
    </button>
    <button class="md-icon-btn" data-tip="清空日志" onclick={() => (clearOpen = true)}>
      <Icon name="clear_all" size={24} color="var(--md-on-surface)" />
    </button>
  {/snippet}

  <div class="logs-root">
    {#if logs.length === 0}
      <div class="empty-state">
        <Icon name="article_outlined" size={64} color="var(--flutter-grey)" />
        <div class="gap-16"></div>
        <div class="empty-title">暂无日志</div>
      </div>
    {:else}
      <div class="logs-scroll" bind:this={scrollEl}>
        {#each logs as log, index (index)}
          {@const kind = logKind(log)}
          <div class="md-card log-card">
            <div class="md-list-tile dense no-cursor log-tile">
              <span class="tile-leading">
                <Icon
                  name={kind === 'error' ? 'error_outline' : kind === 'warning' ? 'warning_amber_outlined' : 'info_outline'}
                  size={20}
                  color={kind === 'error' ? 'var(--flutter-red)' : kind === 'warning' ? 'var(--flutter-orange)' : 'var(--flutter-blue)'}
                />
              </span>
              <div class="tile-content">
                <div class="log-title {kind}">{log}</div>
              </div>
              <span class="tile-trailing">
                <button class="md-icon-btn" data-tip="复制此条日志" onclick={() => copyOneLog(log)}>
                  <Icon name="copy" size={16} />
                </button>
              </span>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <button class="md-fab logs-fab" data-tip="滚动到底部" onclick={scrollToBottom}>
      <Icon name="keyboard_arrow_down" size={24} />
    </button>
  </div>
</SubPage>

<Dialog bind:open={clearOpen} title="确认清空">
  <div>确定要清空所有日志吗？</div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (clearOpen = false)}>取消</button>
    <button class="md-btn md-text-btn" onclick={confirmClear}>确定</button>
  {/snippet}
</Dialog>

<style>
  .logs-root {
    height: 100%;
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .logs-scroll {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px;
  }
  /* 空状态：居中 Column */
  .empty-state {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .empty-title {
    font-size: 18px;
    color: var(--flutter-grey);
  }
  .gap-16 {
    height: 16px;
  }
  .log-card {
    margin: 2px 0;
  }
  .log-title {
    font-family: monospace;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface);
    word-break: break-all;
    white-space: pre-wrap;
  }
  .log-title.error {
    color: var(--flutter-red);
  }
  .log-title.warning {
    color: var(--flutter-orange);
  }
  /* FAB：Scaffold 右下角固定 */
  .logs-fab {
    position: absolute;
    right: 16px;
    bottom: 16px;
    z-index: 5;
  }
</style>
