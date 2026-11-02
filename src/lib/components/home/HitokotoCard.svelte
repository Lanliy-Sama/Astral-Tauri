<script>
  /**
   * 一言卡 —— 对照 lib/features/home/widgets/hitokoto_card.dart。
   * GET https://v1.hitokoto.cn/ ；含 加载/获取失败/网络错误 全部状态文案。
   */
  import { onMount } from 'svelte';
  import HomeBox from './HomeBox.svelte';
  import Icon from '../common/Icon.svelte';
  import Progress from '../common/Progress.svelte';

  let hitokoto = $state('加载中...');
  let hitokotoFrom = $state('');
  let isLoadingHitokoto = $state(true);

  onMount(() => {
    fetchHitokoto();
  });

  async function fetchHitokoto() {
    try {
      const response = await fetch('https://v1.hitokoto.cn/');
      if (response.status === 200) {
        const data = await response.json();
        hitokoto = data['hitokoto'] ?? '暂无内容';
        hitokotoFrom = data['from'] ?? '未知来源';
        isLoadingHitokoto = false;
      } else {
        hitokoto = '获取失败';
        hitokotoFrom = '';
        isLoadingHitokoto = false;
      }
    } catch {
      hitokoto = '网络错误';
      hitokotoFrom = '';
      isLoadingHitokoto = false;
    }
  }

  function refresh() {
    if (isLoadingHitokoto) return;
    isLoadingHitokoto = true;
    hitokoto = '加载中...';
    hitokotoFrom = '';
    fetchHitokoto();
  }
</script>

<HomeBox>
  <div class="hitokoto-card">
    <div class="title-row">
      <Icon name="format_quote" size={22} color="var(--md-primary)" />
      <span class="title-text">一言</span>
      <div class="title-spacer"></div>
      <button
        class="md-icon-btn"
        data-tip="刷新一言"
        disabled={isLoadingHitokoto}
        onclick={refresh}
      >
        <Icon name="refresh" size={18} color="var(--md-primary)" />
      </button>
    </div>

    {#if isLoadingHitokoto}
      <div class="loading-row">
        <Progress type="circular" size={16} strokeWidth={2} />
        <span class="loading-text">加载中...</span>
      </div>
    {:else}
      <div class="hitokoto-content">
        <div class="hitokoto-text">{hitokoto}</div>
        {#if hitokotoFrom}
          <div class="hitokoto-from">—— {hitokotoFrom}</div>
        {/if}
      </div>
    {/if}
  </div>
</HomeBox>

<style>
  .hitokoto-card {
    display: flex;
    flex-direction: column;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title-text {
    font-size: 18px;
    font-weight: 400;
  }
  .title-spacer {
    flex: 1 1 0;
  }

  .loading-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
  }
  .loading-text {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-06);
  }

  .hitokoto-content {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .hitokoto-text {
    font-size: 14px;
    line-height: 1.4;
    color: var(--md-on-surface);
    word-break: break-word;
  }
  .hitokoto-from {
    margin-top: 6px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-06);
    font-style: italic;
  }
</style>
