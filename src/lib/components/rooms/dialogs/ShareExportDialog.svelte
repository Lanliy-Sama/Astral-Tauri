<script module>
  /**
   * 复制房间分享链接 —— 翻译自 room_share_copy_link.dart 的 copyRoomShareLink。
   * linkOnly=true 时仅复制 astral:// 链接（当前房间卡点击 / 分享对话框按钮都走此路径）。
   */
  import { writeClipboard } from '$lib/rust/api.js';
  import { snackError, snackSuccess } from '$lib/components/common/snackbars.svelte.js';
  import {
    encryptRoom,
    validateRoom,
    cleanRoom,
    generateShareLink,
    generateShareText,
  } from '$lib/utils/roomShareCodec.js';

  export async function copyRoomShareLink(room, { linkOnly = false } = {}) {
    try {
      /* RoomShareCodec.generateShareLink：先校验再清洗再加密 */
      const [isValid, errorMessage] = validateRoom(room);
      if (!isValid) throw new Error(`房间数据无效: ${errorMessage}`);
      const cleanedRoom = cleanRoom(room);
      const shareCode = await encryptRoom(cleanedRoom, {
        includeNetworkConfig: (cleanedRoom.networkConfigJson || '').length > 0,
      });
      if (!shareCode) throw new Error('房间加密失败');
      const link = generateShareLink(shareCode);
      const content = linkOnly
        ? link
        : generateShareText(cleanedRoom, link, {
            hasServers: (cleanedRoom.servers || []).length > 0,
            hasNetworkConfig: (cleanedRoom.networkConfigJson || '').length > 0,
          });
      const ok = await writeClipboard(content);
      if (!ok) throw new Error('无法写入剪贴板');
      snackSuccess('复制成功', linkOnly ? '房间链接已复制到剪贴板' : '房间分享信息已复制到剪贴板');
    } catch (e) {
      snackError('复制失败', String(e?.message ?? e));
    }
  }
</script>

<script>
  /**
   * 房间分享导出对话框 —— 翻译自 room_share_export_dialog.dart /
   * room_share_export_body.dart / room_share_advanced_options.dart。
   *
   * 数据流（严格对照原版）：
   * - enabledServerUrls：每启用服务器按 tcp/faketcp/udp/ws/wss/quic/wg/txt/srv/http/https
   *   协议开关生成 URL；
   * - buildRoomToShare：勾选服务器 → Room.servers + customParam=毫秒时间戳；
   *   勾选配置 → networkConfigJson（短键 JSON，仅含勾选项）。
   * - Dialog r28 / bg surface / 整体滚动（maxWidth 560，M3 Dialog 默认约束）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { networkConfigShareFromState } from '$lib/utils/roomShareCodec.js';

  let { open = $bindable(false), room = null } = $props();

  const CONFIG_ITEMS = [
    { key: 'dhcp', label: 'DHCP' },
    { key: 'defaultProtocol', label: '默认协议' },
    { key: 'enableEncryption', label: '加密' },
    { key: 'latencyFirst', label: '延迟优先' },
    { key: 'disableP2p', label: '禁用P2P' },
    { key: 'disableUdpHolePunching', label: '禁用UDP打洞' },
    { key: 'disableTcpHolePunching', label: '禁用TCP打洞' },
    { key: 'disableSymHolePunching', label: '禁用对称打洞' },
    { key: 'dataCompressAlgo', label: '数据压缩' },
    { key: 'enableKcpProxy', label: 'KCP代理' },
    { key: 'bindDevice', label: '绑定设备' },
    { key: 'noTun', label: '禁用TUN' },
  ];
  const PROTOCOL_ORDER = [
    'tcp', 'faketcp', 'udp', 'ws', 'wss', 'quic', 'wg', 'txt', 'srv', 'http', 'https',
  ];

  let selectedServers = $state([]);
  let networkConfigOptions = $state(
    Object.fromEntries(CONFIG_ITEMS.map((item) => [item.key, false]))
  );
  let expanded = $state(false);

  /* 打开时重置（对应 showShareDialog 内的初始状态） */
  $effect(() => {
    if (open) {
      selectedServers = [];
      networkConfigOptions = Object.fromEntries(CONFIG_ITEMS.map((item) => [item.key, false]));
      expanded = false;
    }
  });

  const enabledServerUrls = $derived(
    sm.serverState.servers.flatMap((server) => {
      if (!server.enable) return [];
      return PROTOCOL_ORDER.filter((proto) => server[proto]).map(
        (proto) => `${proto}://${server.url}`
      );
    })
  );

  const configCount = $derived(Object.values(networkConfigOptions).filter(Boolean).length);
  const hasServers = $derived(selectedServers.length > 0);
  const hasNetworkConfig = $derived(configCount > 0);

  /* NetworkConfigShare：仅勾选项进入 JSON（短键，null 字段省略） */
  function buildNetworkConfigJson(options) {
    const current = networkConfigShareFromState(sm.networkConfigState);
    const json = {};
    if (options.dhcp) json.dhcp = current.dhcp;
    if (options.defaultProtocol) json.proto = current.proto;
    if (options.enableEncryption) json.enc = current.enc;
    if (options.latencyFirst) json.lat = current.lat;
    if (options.disableP2p) json.dp2p = current.dp2p;
    if (options.disableUdpHolePunching) json.dudp = current.dudp;
    if (options.disableTcpHolePunching) json.dtcp = current.dtcp;
    if (options.disableSymHolePunching) json.dsym = current.dsym;
    if (options.dataCompressAlgo) json.comp = current.comp;
    if (options.enableKcpProxy) json.kcp = current.kcp;
    if (options.bindDevice) json.bind = current.bind;
    if (options.noTun) json.tun = current.tun;
    return JSON.stringify(json);
  }

  /* buildRoomToShare */
  const roomToShare = $derived.by(() => {
    if (!room) return null;
    return {
      ...room,
      servers: hasServers ? [...selectedServers] : [],
      customParam: hasServers ? String(Date.now()) : '',
      networkConfigJson: hasNetworkConfig ? buildNetworkConfigJson(networkConfigOptions) : '',
    };
  });

  const summaryText = $derived(
    hasServers && hasNetworkConfig
      ? `将分享 ${selectedServers.length} 个服务器 + ${configCount} 项配置`
      : hasServers
        ? `将分享 ${selectedServers.length} 个服务器`
        : `将分享 ${configCount} 项配置`
  );

  function toggleServer(url) {
    selectedServers = selectedServers.includes(url)
      ? selectedServers.filter((u) => u !== url)
      : [...selectedServers, url];
  }

  function toggleConfig(key) {
    networkConfigOptions = { ...networkConfigOptions, [key]: !networkConfigOptions[key] };
  }

  function onCopy() {
    const target = roomToShare;
    open = false;
    copyRoomShareLink(target, { linkOnly: true });
  }

  function onKeydown(e) {
    if (e.key === 'Escape' && open) open = false;
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open && room}
  <div
    class="md-dialog-scrim share-scrim"
    onclick={(e) => {
      if (e.target === e.currentTarget) open = false;
    }}
  >
    <div class="share-dialog" role="dialog">
      <!-- Header：Padding(24,24,24,16) -->
      <div class="share-header">
        <div class="share-icon-box">
          <Icon name="share" size={24} color="var(--md-on-primary-container)" />
        </div>
        <div class="share-title-col">
          <div class="share-title">分享房间</div>
          <div class="share-room-name">{room.name}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <!-- 内容 Padding(24) -->
      <div class="share-content">
        <!-- 房间基本信息卡 -->
        <div class="info-card">
          <div class="info-header">
            <Icon name="info_outline" size={18} color="var(--md-primary)" />
            <span class="info-title">房间基本信息</span>
          </div>
          <div class="info-row">
            <Icon name="tag" size={16} color="var(--md-on-surface-variant)" />
            <div class="info-name-col">
              <div class="info-label">房间名称</div>
              <div class="info-name">{room.name}</div>
            </div>
          </div>
        </div>

        <!-- 高级选项 ExpansionTile（默认收起） -->
        {#if enabledServerUrls.length > 0 || CONFIG_ITEMS.length > 0}
          <div class="advanced-gap"></div>
          <div class="md-expansion share-expansion" class:open={expanded}>
            <div class="expansion-header" onclick={() => (expanded = !expanded)}>
              <Icon name="tune_outlined" size={20} color="var(--md-secondary)" />
              <span class="exp-title">高级选项</span>
              <span class="ms chev">expand_more</span>
            </div>
            {#if expanded}
              <div class="exp-body">
                {#if enabledServerUrls.length > 0}
                  <div class="section-label">🔗 服务器列表</div>
                  <div class="check-container">
                    {#each enabledServerUrls as url, i (i)}
                      <div class="check-tile" onclick={() => toggleServer(url)}>
                        <span
                          class="md-checkbox {selectedServers.includes(url) ? 'checked' : ''}"
                        ><span class="ms">check</span></span>
                        <span class="url-text">{url}</span>
                      </div>
                    {/each}
                  </div>
                  <div class="section-gap"></div>
                {/if}
                <div class="section-label">⚙️ 网络配置</div>
                <div class="check-container">
                  {#each CONFIG_ITEMS as item, i (item.key)}
                    {#if i > 0}<div class="md-divider"></div>{/if}
                    <div class="check-tile" onclick={() => toggleConfig(item.key)}>
                      <span
                        class="md-checkbox {networkConfigOptions[item.key] ? 'checked' : ''}"
                      ><span class="ms">check</span></span>
                      <span class="config-label">{item.label}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}

        <!-- 摘要条 -->
        {#if hasServers || hasNetworkConfig}
          <div class="summary-gap"></div>
          <div class="summary-bar">
            <Icon name="check_circle_outline" size={18} color="var(--md-secondary)" />
            <span class="summary-text ellipsis-2">{summaryText}</span>
          </div>
        {/if}
      </div>

      <div class="md-divider"></div>

      <!-- actions Padding(16) Row(end) -->
      <div class="share-actions">
        <button class="md-btn md-text-btn" onclick={() => (open = false)}>取消</button>
        <button class="md-btn md-filled-btn" onclick={onCopy}>
          <Icon name="copy_outlined" size={18} />
          <span>复制链接</span>
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Dialog 默认 insetPadding：水平 40 垂直 24 */
  .share-scrim {
    padding: 24px 40px;
  }
  .share-dialog {
    width: min(560px, 100%);
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--md-surface);
    border-radius: 28px;
    box-shadow:
      0 3px 3px rgba(0, 0, 0, 0.3),
      0 12px 24px 6px rgba(0, 0, 0, 0.15);
  }

  .share-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 24px 24px 16px;
  }
  .share-icon-box {
    padding: 8px;
    background: var(--md-primary-container);
    border-radius: 12px;
    display: inline-flex;
    flex-shrink: 0;
  }
  .share-title-col {
    flex: 1 1 0;
    min-width: 0;
  }
  .share-title {
    font-size: 24px;
    line-height: 32px;
    font-weight: 600;
    color: var(--md-on-surface);
  }
  .share-room-name {
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .share-content {
    padding: 24px;
  }

  /* 基本信息：p16 / surfaceContainerHighest@0.3 / r16 / border outlineVariant */
  .info-card {
    padding: 16px;
    background: color-mix(in srgb, var(--md-surface-container-highest) 30%, transparent);
    border: 1px solid var(--md-outline-variant);
    border-radius: 16px;
  }
  .info-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .info-title {
    font-size: 12px;
    line-height: 16px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: var(--md-primary);
  }
  .info-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }
  .info-name-col {
    flex: 1 1 0;
    min-width: 0;
  }
  .info-label {
    font-size: 11px;
    line-height: 16px;
    font-weight: 500;
    letter-spacing: 0.5px;
    color: var(--md-on-surface-variant);
  }
  .info-name {
    margin-top: 4px;
    font-size: 14px;
    line-height: 20px;
    font-weight: 500;
    color: var(--md-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .advanced-gap {
    height: 24px;
  }

  /* ExpansionTile：collapsed=surfaceContainerHighest@0.2，expanded=surface */
  .share-expansion {
    background: color-mix(in srgb, var(--md-surface-container-highest) 20%, transparent);
  }
  .share-expansion.open {
    background: var(--md-surface);
  }
  .exp-title {
    flex: 1 1 0;
    font-size: 14px;
    line-height: 20px;
    font-weight: 600;
    letter-spacing: 0.1px;
    color: var(--md-secondary);
  }
  .exp-body {
    padding: 12px 16px;
  }
  .section-label {
    font-size: 12px;
    line-height: 16px;
    font-weight: 600;
    color: var(--md-on-surface);
  }
  .section-gap {
    height: 16px;
  }
  .check-container {
    margin-top: 8px;
    border: 1px solid var(--md-outline-variant);
    border-radius: 8px;
    overflow: hidden;
  }
  /* CheckboxListTile dense：48 高 / h16 */
  .check-tile {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 48px;
    padding: 0 16px;
    cursor: pointer;
  }
  .check-tile:hover {
    background: var(--md-on-surface-08);
  }
  .url-text {
    flex: 1 1 0;
    min-width: 0;
    font-family: monospace;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .config-label {
    flex: 1 1 0;
    min-width: 0;
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface);
  }

  /* 摘要条：p12 / secondaryContainer@0.3 / r12 / border outline */
  .summary-gap {
    height: 16px;
  }
  .summary-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    background: color-mix(in srgb, var(--md-secondary-container) 30%, transparent);
    border: 1px solid var(--md-outline);
    border-radius: 12px;
  }
  .summary-text {
    flex: 1 1 0;
    min-width: 0;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-secondary);
  }

  .share-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px;
  }
</style>
