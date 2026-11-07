<script>
  /**
   * 列表模式用户卡 —— list_user_card.dart 1:1 移植。
   * 单行紧凑项：person 图标 + 名 + 类型徽章 + 延迟/丢包 + IP（右对齐）；
   * hover bg primary@0.06；底边框 outlineVariant@0.5（非最后）；点击复制 IP。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { snackSuccess } from '$lib/components/common/SnackbarStore.svelte';
  import { writeClipboard } from '$lib/rust/Api.svelte';
  import {
    mapConnectionType,
    connectionTypeColor,
    latencyColor,
    lossColor,
  } from '$lib/utils/Display.svelte';

  let { player, localIpv4 = '', showDivider = true } = $props();

  const displayName = $derived(
    (player.hostname || '').startsWith('PublicServer_')
      ? player.hostname.substring('PublicServer_'.length)
      : player.hostname || ''
  );
  const connectionType = $derived(mapConnectionType(player.cost, player.ipv4, localIpv4));
  const typeColor = $derived(connectionTypeColor(connectionType));
  const latColor = $derived(latencyColor(player.latencyMs ?? 0));
  const lossCol = $derived(lossColor(player.lossRate ?? 0));
  const hasIp = $derived(!!player.ipv4 && player.ipv4 !== '0.0.0.0');

  async function copyIp() {
    await writeClipboard(player.ipv4);
    snackSuccess('已复制', `IP地址: ${player.ipv4}`, { duration: 2000 });
  }
</script>

<div
  class="list-user-card"
  class:divider={showDivider}
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
  <Icon name="person" size={18} color="var(--md-primary)" />
  <span class="name">{displayName}</span>
  <span class="badge" style="background:{typeColor}">{connectionType}</span>
  {#if connectionType !== '本机'}
    <span class="latency" style="color:{latColor}">{(player.latencyMs ?? 0).toFixed(0)}ms</span>
    <span class="loss" style="color:{lossCol}">{(player.lossRate ?? 0).toFixed(1)}%</span>
  {/if}
  {#if hasIp}
    <span class="ip">{player.ipv4}</span>
  {/if}
</div>

<style>
  .list-user-card {
    display: flex;
    align-items: center;
    padding: 10px 12px;
    cursor: pointer;
  }
  .list-user-card:hover {
    background: var(--md-primary-06);
  }
  .list-user-card.divider {
    border-bottom: 1px solid color-mix(in srgb, var(--md-outline-variant) 50%, transparent);
  }
  .name {
    margin: 0 8px;
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Dart FontWeight.w600：MiSans 实际渲染 Demibold 档（对应 CSS 500） */
    font-weight: 500;
    font-size: 14px;
    color: var(--md-on-surface);
  }
  .badge {
    flex-shrink: 0;
    padding: 2px 6px;
    border-radius: 6px;
    color: #fff;
    font-size: 11px;
  }
  .latency {
    margin-left: 8px;
    flex-shrink: 0;
    font-weight: 700;
    font-size: 12px;
  }
  .loss {
    margin-left: 6px;
    flex-shrink: 0;
    font-weight: 700;
    font-size: 12px;
  }
  .ip {
    margin-left: 8px;
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
    color: var(--md-secondary);
    font-size: 12px;
  }
</style>
