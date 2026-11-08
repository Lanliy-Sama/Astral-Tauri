<script>
  /**
   * 详细模式用户卡 —— all_user_card.dart + all_user_card_tile.dart +
   * all_user_card_body.dart + player_card_stats.dart 1:1 移植。
   * Card r8 hover 边框 primary 1；头部（名 + 类型徽章 + 延迟/丢包）+ 网络统计
   * （PlayerCardStats，永远只显示 connections[0]）+ 详情行（IP/ET版本/NAT/隧道/连接路径）。
   * 点击整卡复制 IP（注意原版此处 snack 文案为 'IP: '，与列表/简约卡的 'IP地址: ' 不同）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { snackSuccess } from '$lib/components/common/SnackbarStore.svelte';
  import { writeClipboard } from '$lib/rust/Api.svelte';
  import {
    mapConnectionType,
    connectionTypeColor,
    connectionTypeIcon,
    latencyColor,
    lossColor,
    natFullInfo,
    parsePlatformVersion,
    formatTunnelProto,
    formatSpeed,
  } from '$lib/utils/Display.svelte';

  let { player, localIpv4 = '' } = $props();

  const displayName = $derived(
    (player.hostname || '').startsWith('PublicServer_')
      ? player.hostname.substring('PublicServer_'.length)
      : player.hostname || ''
  );
  const connectionType = $derived(mapConnectionType(player.cost, player.ipv4, localIpv4));
  const typeColor = $derived(connectionTypeColor(connectionType));
  const connIcon = $derived(connectionTypeIcon(connectionType));
  const latColor = $derived(latencyColor(player.latencyMs ?? 0));
  const lossCol = $derived(lossColor(player.lossRate ?? 0));
  const nat = $derived(natFullInfo(player.nat));
  const platform = $derived(parsePlatformVersion(player.version));
  const hasIp = $derived(!!player.ipv4 && player.ipv4 !== '0.0.0.0');
  const connection = $derived(player.connections?.[0] ?? null);
  const hops = $derived(player.hops || []);

  async function copyIp() {
    await writeClipboard(player.ipv4);
    snackSuccess('已复制', `IP: ${player.ipv4}`, { duration: 2000 });
  }
  async function copyValue(value) {
    await writeClipboard(value);
    snackSuccess('已复制', `IP地址: ${value}`, { duration: 2000 });
  }

  /* Dart '$double' 插值：整数值渲染带 ".0" 尾缀（如 6400.0） */
  function dartDouble(value) {
    const v = Number(value) || 0;
    return Number.isInteger(v) ? `${v}.0` : String(v);
  }

  /* 统计项（PlayerCardStats.buildStatItem） */
  function statItem(icon, label, value, color) {
    return { icon, label, value, color };
  }
  /* 延迟历史曲线（原版 Flutter 的延迟折线图）：本卡节点的逐秒延迟采样 */
  const latencyHistory = $derived(sm.latencyHistory[String(player.peerId ?? player.hostname)] || []);
  const chartPoints = $derived.by(() => {
    const h = latencyHistory;
    if (!h || h.length < 2) return '';
    const W = 300;
    const Hh = 44;
    const maxMs = Math.max(60, ...h);
    const step = W / (h.length - 1);
    return h
      .map((ms, i) => {
        const y = Hh - 3 - Math.min(ms, maxMs) / maxMs * (Hh - 6);
        return `${(i * step).toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  });

  const leftStats = $derived(
    connection
      ? [
          statItem('upload_rounded', '累计上传', formatSpeed(connection.txBytes), 'var(--md-primary)'),
          statItem('arrow_upward_rounded', '累计发送包', dartDouble(connection.txPackets), 'var(--md-primary)'),
        ]
      : []
  );
  const rightStats = $derived(
    connection
      ? [
          statItem('download_rounded', '累计下载', formatSpeed(connection.rxBytes), 'var(--md-secondary)'),
          statItem('arrow_downward_rounded', '累计接收包', dartDouble(connection.rxPackets), 'var(--md-secondary)'),
        ]
      : []
  );
</script>

<div
  class="all-card"
  role="button"
  tabindex="0"
  onclick={copyIp}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      copyIp();
    }
  }}
>
  <div class="tile">
    <!-- 头部：名称 + 类型徽章 + 延迟/丢包 -->
    <div class="header">
      <div class="title">
        <Icon name="person" size={22} color="var(--md-primary)" />
        <span class="name" data-tip={displayName}>{displayName}</span>
      </div>
      <div class="badges">
        <span class="type-badge" style="background:{typeColor}">
          <Icon name={connIcon} size={14} color="#fff" />
          <span>{connectionType}</span>
        </span>
        {#if connectionType !== '本机'}
          <span class="stat" data-tip="延迟">
            <Icon name="timer_outlined" size={18} color={latColor} />
            <span style="color:{latColor}">{(player.latencyMs ?? 0).toFixed(0)} ms</span>
          </span>
          <span class="stat" data-tip="丢包率">
            <Icon name="error_outline" size={18} color={lossCol} />
            <span style="color:{lossCol}">{(player.lossRate ?? 0).toFixed(1)}%</span>
          </span>
        {/if}
      </div>
    </div>

    <div class="md-divider d1"></div>

    <!-- 延迟历史曲线 -->
    {#if chartPoints}
      <svg class="lat-chart" viewBox="0 0 300 44" preserveAspectRatio="none">
        <polyline points={chartPoints} fill="none" stroke={latColor} stroke-width="1.5"
          stroke-linejoin="round" stroke-linecap="round" />
      </svg>
    {:else if connection}
      <div class="lat-chart-hint">正在收集延迟数据…</div>
    {/if}

    <!-- 网络数据（无连接数据时居中提示） -->
    {#if !connection}
      <div class="stats-section">
        <div class="no-conn">无连接数据</div>
      </div>
    {:else}
      <div class="stats-section">
        <div class="stats-header">
          <Icon name="wifi" size={20} color="var(--md-primary)" />
          <span class="stats-title">网络数据:</span>
        </div>
        <!-- SizedBox(h90) + PlayerCardStats -->
        <div class="stats">
          <div class="stats-col">
            {#each leftStats as item (item.label)}
              <div class="stat-item">
                <Icon name={item.icon} size={18} color={item.color} />
                <div class="stat-text">
                  <div class="stat-label">{item.label}</div>
                  <div class="stat-value" style="color:{item.color}">{item.value}</div>
                </div>
              </div>
            {/each}
          </div>
          <div class="stats-col">
            {#each rightStats as item (item.label)}
              <div class="stat-item">
                <Icon name={item.icon} size={18} color={item.color} />
                <div class="stat-text">
                  <div class="stat-label">{item.label}</div>
                  <div class="stat-value" style="color:{item.color}">{item.value}</div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}

    <div class="md-divider d2"></div>

    <!-- 详情行（PlayerCardStats.buildInfoRow） -->
    <div class="details">
      {#if hasIp}
        <div class="info-row">
          <Icon
            name="lan"
            size={20}
            color="var(--md-primary)"
            style="font-variation-settings:'FILL' 0"
          />
          <span class="info-label">IP地址:</span>
          <button
            class="copy-btn"
            data-tip="复制IP地址"
            onclick={(e) => {
              e.stopPropagation();
              copyValue(player.ipv4);
            }}
          >
            <Icon name="copy" size={18} />
          </button>
          <span class="info-value">{player.ipv4}</span>
        </div>
      {/if}
      <div class="info-row">
        <Icon name={platform.icon} size={20} color="var(--md-primary)" />
        <span class="info-label">ET版本:</span>
        <span class="info-value">{platform.number}</span>
      </div>
      <div class="info-row">
        <Icon
          name={nat.icon === 'help_outline' ? 'help' : nat.icon}
          size={20}
          color="var(--md-primary)"
        />
        <span class="info-label">NAT类型:</span>
        <span class="info-value" style="color:{nat.color}">{nat.label}</span>
      </div>
      {#if player.tunnelProto}
        <div class="info-row">
          <Icon name="router" size={20} color="var(--md-primary)" />
          <span class="info-label">隧道类型:</span>
          <span class="info-value">{formatTunnelProto(player.tunnelProto)}</span>
        </div>
      {/if}
      {#if hops.length > 0}
        <!-- 连接路径（AllUserCardBody.buildHopsInfo） -->
        <div class="hops">
          <Icon name="route" size={20} color="var(--md-primary)" />
          <div class="hops-body">
            <div class="hops-title">连接路径:</div>
            <div class="hops-list">
              {#each hops as hop, i (i)}
                <span class="hop-chip">
                  {hop.nodeName} ({(hop.latencyMs ?? 0).toFixed(0)}ms,{' '}
                  {(hop.packetLoss ?? 0).toFixed(1)}%)
                </span>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .all-card {
    background: var(--md-surface-container-low);
    border-radius: 8px;
    border: 1px solid transparent;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15);
    cursor: pointer;
    overflow: hidden;
  }
  .all-card:hover {
    border-color: var(--md-primary);
  }
  /* InkWell highlightColor primary@0.1 */
  .all-card:active {
    background: color-mix(in srgb, var(--md-surface-container-low) 90%, var(--md-primary) 10%);
  }
  .tile {
    padding: 12px;
  }

  /* 头部 */
  .header {
    display: flex;
    align-items: flex-start;
  }
  .title {
    display: flex;
    align-items: center;
    flex: 1 1 0;
    min-width: 0;
  }
  .name {
    margin-left: 8px;
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 18px;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    align-items: center;
    gap: 4px 12px;
    margin-left: 16px;
  }
  .type-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: 12px;
    color: #fff;
    font-size: 12px;
    font-weight: 700;
  }
  .stat {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-weight: 700;
    font-size: 13px;
  }

  .d1 {
    margin-top: 8px;
  }
  .d2 {
    margin-top: 8px;
  }

  /* 网络统计区 */
  .lat-chart {
    display: block;
    width: 100%;
    height: 44px;
    padding: 2px 8px 0;
    box-sizing: border-box;
  }
  .lat-chart-hint {
    padding: 4px 16px 6px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
    opacity: 0.7;
  }

  .stats-section {
    margin-top: 12px;
  }
  .no-conn {
    display: flex;
    justify-content: center;
    color: var(--md-on-surface-06);
  }
  .stats-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .stats-title {
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .stats {
    display: flex;
    gap: 16px;
    margin-top: 12px;
    min-height: 90px;
    align-items: flex-start;
  }
  .stats-col {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .stat-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .stat-item + .stat-item {
    margin-top: 10px;
  }
  .stat-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .stat-label {
    font-size: 12px;
    color: var(--md-on-surface-07);
  }
  .stat-value {
    font-weight: 700;
    font-size: 14px;
  }

  /* 详情行 */
  .details {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
  }
  .details > * + * {
    margin-top: 8px;
  }
  .info-row {
    display: flex;
    align-items: center;
  }
  .info-label {
    margin-left: 12px;
    flex-shrink: 0;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .copy-btn {
    margin-left: 8px;
    width: 18px;
    height: 18px;
    min-width: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--md-on-surface-variant);
    cursor: pointer;
  }
  .copy-btn:hover {
    background: var(--md-on-surface-08);
  }
  .info-value {
    margin-left: 8px;
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
    color: var(--md-secondary);
  }

  /* 连接路径 */
  .hops {
    display: flex;
    align-items: flex-start;
  }
  .hops-body {
    margin-left: 12px;
    flex: 1 1 0;
    min-width: 0;
  }
  .hops-title {
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .hops-list {
    margin-top: 4px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .hop-chip + .hop-chip {
    margin-top: 4px;
  }
  .hop-chip {
    padding: 2px 6px;
    background: var(--md-primary-container);
    border-radius: 4px;
    color: var(--md-on-primary-container);
    font-size: 12px;
  }
</style>
