<script>
  /**
   * 简约模式用户卡 —— mini_user_card.dart 1:1 移植。
   * Card r8 hover 边框 primary 1；两行：名称/徽章/延迟/丢包 + IP/平台版本/NAT/隧道协议；
   * 点击复制 IP。NAT 走游戏化等级文案（natMiniInfo）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { snackSuccess } from '$lib/components/common/snackbars.svelte.js';
  import { writeClipboard } from '$lib/rust/api.js';
  import {
    mapConnectionType,
    connectionTypeColor,
    latencyColor,
    lossColor,
    natMiniInfo,
    parsePlatformVersion,
    formatTunnelProto,
  } from '$lib/utils/display.js';

  let { player, localIpv4 = '' } = $props();

  const displayName = $derived(
    (player.hostname || '').startsWith('PublicServer_')
      ? player.hostname.substring('PublicServer_'.length)
      : player.hostname || ''
  );
  const connectionType = $derived(mapConnectionType(player.cost, player.ipv4, localIpv4));
  const typeColor = $derived(connectionTypeColor(connectionType));
  const latColor = $derived(latencyColor(player.latencyMs ?? 0));
  const lossCol = $derived(lossColor(player.lossRate ?? 0));
  const nat = $derived(natMiniInfo(player.nat));
  const platform = $derived(parsePlatformVersion(player.version));
  const hasIp = $derived(!!player.ipv4 && player.ipv4 !== '0.0.0.0');
  const showNat = $derived(connectionType !== '本机' && !!player.nat);

  async function copyIp() {
    await writeClipboard(player.ipv4);
    snackSuccess('已复制', `IP地址: ${player.ipv4}`, { duration: 2000 });
  }
</script>

<div
  class="mini-card"
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
  <!-- 第一行：名称 类型 延迟 丢包 -->
  <div class="row row1">
    <Icon name="person" size={18} color="var(--md-primary)" />
    <span class="name" data-tip={displayName}>{displayName}</span>
    <span class="badge" style="background:{typeColor}">{connectionType}</span>
    {#if connectionType !== '本机'}
      <Icon name="timer_outlined" size={16} color={latColor} style="margin-left:10px" />
      <span class="metric" style="color:{latColor}">{(player.latencyMs ?? 0).toFixed(0)}ms</span>
      <Icon name="error_outline" size={16} color={lossCol} style="margin-left:10px" />
      <span class="metric" style="color:{lossCol}">{(player.lossRate ?? 0).toFixed(1)}%</span>
    {/if}
  </div>
  <!-- 第二行：IP地址 ET版本 打洞难易 隧道协议 -->
  <div class="row row2">
    {#if hasIp}
      <!-- Dart Icons.lan_outlined：Outlined 字形（Material Symbols lan + FILL 0） -->
      <Icon
        name="lan"
        size={16}
        color="var(--md-primary)"
        style="font-variation-settings:'FILL' 0"
      />
    {/if}
    <span class="ip" data-tip={hasIp ? player.ipv4 : null}>{hasIp ? player.ipv4 : ''}</span>
    <Icon name={platform.icon} size={16} color="var(--md-primary)" style="margin-left:10px" />
    <span class="text">{platform.number}</span>
    {#if showNat}
      <Icon name={nat.icon === 'help_outline' ? 'help' : nat.icon} size={16} color={nat.color} style="margin-left:10px" />
      <span class="text bold" style="color:{nat.color}">{nat.label}</span>
    {/if}
    {#if player.tunnelProto}
      <Icon name="router" size={16} color="var(--md-primary)" style="margin-left:10px" />
      <span class="text">{formatTunnelProto(player.tunnelProto)}</span>
    {/if}
  </div>
</div>

<style>
  .mini-card {
    background: var(--md-surface-container-low);
    border-radius: 8px;
    border: 1px solid transparent;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.3),
      0 1px 3px 1px rgba(0, 0, 0, 0.15);
    padding: 8px 12px;
    cursor: pointer;
    overflow: hidden;
  }
  .mini-card:hover {
    border-color: var(--md-primary);
  }
  .row {
    display: flex;
    align-items: center;
    white-space: nowrap;
  }
  .row2 {
    margin-top: 4px;
    overflow: hidden;
  }
  .name {
    margin-left: 6px;
    margin-right: 10px;
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 700;
    font-size: 15px;
    color: var(--md-on-surface);
  }
  .badge {
    flex-shrink: 0;
    padding: 2px 6px;
    border-radius: 8px;
    color: #fff;
    font-size: 12px;
  }
  .metric {
    flex-shrink: 0;
    font-weight: 700;
    font-size: 13px;
  }
  .ip {
    margin-left: 4px;
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--md-secondary);
    font-size: 13px;
  }
  .text {
    flex-shrink: 0;
    color: var(--md-secondary);
    font-size: 13px;
  }
  .text.bold {
    font-weight: 700;
  }
</style>
