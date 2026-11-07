<script>
  /**
   * 导入房间（粘贴弹窗）—— 翻译自 room_share_import_dialog.dart。
   * 流程：链接解析 → 长度/解密/校验（错误 SnackBar 全部按原文案）→
   * networkConfigJson → 「检测到网络配置」确认对话框（可取消=不应用继续导入）→
   * addRoom → goToRoom（uiState.goTo(房间Tab) + setRoom）→ 成功 SnackBar。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import TextField from '$lib/components/common/TextField.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { readClipboard } from '$lib/rust/Api.svelte';
  import { snackError, snackInfo, snackSuccess } from '$lib/components/common/SnackbarStore.svelte';
  import {
    decryptRoom,
    validateRoom,
    cleanRoom,
    parseNetworkConfigShare,
    networkConfigShareSummary,
  } from '$lib/utils/RoomShareCodec.svelte';

  let { open = $bindable(false) } = $props();

  let shareCode = $state('');

  /* 「检测到网络配置」确认弹窗（导入流程内部的 showDialog<bool>） */
  let confirmOpen = $state(false);
  let confirmShare = $state(null);
  let applyConfig = $state(true);
  let confirmResolve = null;

  $effect(() => {
    if (open) shareCode = '';
  });

  function askApplyNetworkConfig(share) {
    return new Promise((resolve) => {
      confirmShare = share;
      applyConfig = true;
      confirmResolve = resolve;
      confirmOpen = true;
    });
  }

  function settleConfirm(value) {
    confirmOpen = false;
    const resolve = confirmResolve;
    confirmResolve = null;
    resolve?.(value);
  }

  /* NetworkConfigShare.applyToConfig：逐项调用更新服务 */
  async function applyNetworkConfigShare(share) {
    if (share.dhcp != null) await sm.updateDhcp(share.dhcp);
    if (share.defaultProtocol != null && share.defaultProtocol !== '') {
      await sm.updateDefaultProtocol(share.defaultProtocol);
    }
    if (share.enableEncryption != null) await sm.updateEnableEncryption(share.enableEncryption);
    if (share.latencyFirst != null) await sm.updateLatencyFirst(share.latencyFirst);
    if (share.disableP2p != null) await sm.updateDisableP2p(share.disableP2p);
    if (share.disableUdpHolePunching != null) {
      await sm.updateDisableUdpHolePunching(share.disableUdpHolePunching);
    }
    if (share.disableTcpHolePunching != null) {
      await sm.updateDisableTcpHolePunching(share.disableTcpHolePunching);
    }
    if (share.disableSymHolePunching != null) {
      await sm.updateDisableSymHolePunching(share.disableSymHolePunching);
    }
    if (share.dataCompressAlgo != null) await sm.updateDataCompressAlgo(share.dataCompressAlgo);
    if (share.enableKcpProxy != null) await sm.updateEnableKcpProxy(share.enableKcpProxy);
    if (share.bindDevice != null) await sm.updateBindDevice(share.bindDevice);
    if (share.noTun != null) await sm.updateNoTun(share.noTun);
  }

  /* importFromClipboard：先关弹窗再读剪贴板 */
  async function importFromClipboard() {
    open = false;
    let clipboardText = '';
    try {
      clipboardText = (await readClipboard() || '').trim();
    } catch (e) {
      snackError('读取剪贴板失败', String(e?.message ?? e));
      return;
    }
    if (!clipboardText) {
      snackError('剪贴板为空', '请先复制房间分享码或链接');
      return;
    }
    await importRoom(clipboardText);
  }

  /* importRoom 主流程 */
  async function importRoom(shareText) {
    try {
      let code = (shareText || '').trim();

      /* 深度链接 astral://room?code=xxx */
      if (code.startsWith('astral://')) {
        let hostOk = false;
        let paramCode = '';
        try {
          const url = new URL(code);
          hostOk = url.host === 'room';
          paramCode = url.searchParams.get('code') ?? '';
        } catch {
          hostOk = false;
        }
        if (!hostOk) {
          snackError('链接格式错误', '不是有效的房间分享链接');
          return;
        }
        code = paramCode;
      }

      code = code.replace(/\s+/g, '');

      if (!code) {
        snackError('分享码为空', '请提供有效的房间分享码');
        return;
      }
      if (code.length < 10) {
        snackError('分享码格式错误', '分享码过短或不完整，请检查是否复制完整');
        return;
      }

      const room = await decryptRoom(code);
      if (!room) {
        snackError('分享码解析失败', '无法解析房间信息，可能是分享码已过期或损坏');
        return;
      }

      const [isValid, errorMessage] = validateRoom(room);
      if (!isValid) {
        snackError('房间数据无效', errorMessage || '房间数据不符合要求');
        return;
      }

      const cleanedRoom = cleanRoom(room);

      /* 携带网络配置 → 确认是否应用 */
      let applyNetworkConfig = false;
      if (cleanedRoom.networkConfigJson && cleanedRoom.networkConfigJson.length > 0) {
        try {
          const networkConfig = parseNetworkConfigShare(cleanedRoom.networkConfigJson);
          if (networkConfig) {
            const shouldApply = await askApplyNetworkConfig(networkConfig);
            if (shouldApply === null) return;
            applyNetworkConfig = shouldApply;
            if (applyNetworkConfig) {
              await applyNetworkConfigShare(networkConfig);
              snackInfo('网络配置', '网络配置已应用');
            }
          }
        } catch (e) {
          console.warn('解析或应用网络配置失败:', e);
        }
      }

      const saved = await sm.addRoom(cleanedRoom);

      /* RoomNavigation.goToRoom：切到房间页并选中 */
      sm.uiState.goTo(1);
      await sm.setRoom(saved);

      let serverInfo = '';
      if (cleanedRoom.servers && cleanedRoom.servers.length > 0) {
        serverInfo = ` (已内置 ${cleanedRoom.servers.length} 个服务器)`;
      }
      let networkConfigInfo = '';
      if (applyNetworkConfig) networkConfigInfo = '\n✓ 已应用网络配置';

      snackSuccess(
        '导入成功',
        `已成功添加并选中房间"${cleanedRoom.name}"${serverInfo}${networkConfigInfo}`
      );
    } catch (e) {
      snackError('导入失败', String(e?.message ?? e));
    }
  }

  function onImport() {
    const code = shareCode;
    if (!code) return;
    open = false;
    importRoom(code);
  }
</script>

<!-- 粘贴导入弹窗 -->
<Dialog bind:open={open} title="导入房间">
  <div class="dialog-body">
    <TextField bind:value={shareCode} placeholder="请输入分享码或链接" multiline rows={3} />
    <div class="gap-16"></div>
    <button class="md-btn md-outlined-btn clipboard-btn" onclick={importFromClipboard}>
      <Icon name="paste" size={18} />
      <span>从剪贴板导入</span>
    </button>
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (open = false)}>取消</button>
    <button class="md-btn md-text-btn" disabled={!shareCode} onclick={onImport}>导入</button>
  {/snippet}
</Dialog>

<!-- 检测到网络配置（barrierDismissible: false） -->
<Dialog bind:open={confirmOpen} dismissible={false}>
  <div class="confirm-title">
    <Icon name="settings_suggest" size={24} color="var(--md-primary)" />
    <span>检测到网络配置</span>
  </div>
  <div class="confirm-body">
    <div class="lead">该房间包含以下网络配置：</div>
    <div class="summary-box">
      {#each networkConfigShareSummary(confirmShare || {}) as line (line)}
        <div class="summary-line">{line}</div>
      {/each}
    </div>
    <!-- CheckboxListTile（contentPadding: 0） -->
    <div class="checkbox-tile" onclick={() => (applyConfig = !applyConfig)}>
      <span class="md-checkbox {applyConfig ? 'checked' : ''}"><span class="ms">check</span></span>
      <div class="cb-content">
        <div class="cb-title">应用网络配置</div>
        <div class="cb-subtitle">将上述配置应用到当前设备</div>
      </div>
    </div>
    <div class="hint">提示：如果不应用，仅导入房间信息</div>
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => settleConfirm(false)}>取消</button>
    <button class="md-btn md-elevated-btn" onclick={() => settleConfirm(applyConfig)}>确定</button>
  {/snippet}
</Dialog>

<style>
  .dialog-body {
    display: flex;
    flex-direction: column;
  }
  .gap-16 {
    height: 16px;
    flex-shrink: 0;
  }
  .clipboard-btn {
    width: 100%;
  }

  /* AlertDialog 标题（icon + 24px 标题），补齐 dialog-title 的 24px 顶部间距 */
  .confirm-title {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    font-size: 24px;
    line-height: 32px;
    color: var(--md-on-surface);
  }
  .confirm-body {
    display: flex;
    flex-direction: column;
    margin-top: 12px;
  }
  .lead {
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface);
  }
  /* 摘要容器：p12 / surfaceContainerHighest@0.5 / r8 / border outline@0.3 */
  .summary-box {
    margin-top: 12px;
    padding: 12px;
    background: var(--md-surface-container-highest-05);
    border: 1px solid var(--md-outline-03);
    border-radius: 8px;
  }
  .summary-line {
    padding-top: 2px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface);
  }
  /* CheckboxListTile（两行，contentPadding 0） */
  .checkbox-tile {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 16px;
    min-height: 72px;
    padding: 8px 0;
    cursor: pointer;
  }
  .checkbox-tile:hover {
    background: var(--md-on-surface-08);
  }
  .cb-content {
    flex: 1 1 0;
    min-width: 0;
  }
  .cb-title {
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
  }
  .cb-subtitle {
    margin-top: 2px;
    font-size: 14px;
    line-height: 20px;
    color: var(--md-on-surface-variant);
  }
  .hint {
    margin-top: 8px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-outline);
  }
</style>
