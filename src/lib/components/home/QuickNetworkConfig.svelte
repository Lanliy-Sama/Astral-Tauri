<script>
  /**
   * 快捷网络配置卡 —— 对照 lib/features/home/widgets/quick_network_config.dart。
   * 强制中转 / 广播转发（仅 Windows）/ 防火墙（仅 Windows）。
   */
  import HomeBox from './HomeBox.svelte';
  import Icon from '../common/Icon.svelte';
  import Switch from '../common/Switch.svelte';
  import { sm } from '../../services/service-manager.svelte.js';
  import { t } from '../../i18n/index.svelte.js';
  import { isWindows } from '../../utils/platform.js';

  /* 广播转发 / 防火墙仅 Windows 显示 */
  const showWindowsOnly = isWindows();

  /* 开关本地镜像：跟随 sm 状态（防火墙失败时 sm 回滚 → 视觉同步回滚） */
  let disableP2p = $state(sm.networkConfigState.disableP2p);
  $effect(() => {
    disableP2p = sm.networkConfigState.disableP2p;
  });

  let broadcastRelay = $state(sm.networkConfigState.enableUdpBroadcastRelay);
  $effect(() => {
    broadcastRelay = sm.networkConfigState.enableUdpBroadcastRelay;
  });

  let firewallOn = $state(sm.firewallState.firewallStatus);
  $effect(() => {
    firewallOn = sm.firewallState.firewallStatus;
  });
</script>

<HomeBox>
  <div class="qnc">
    <div class="title-row">
      <Icon name="tune" size={22} color="var(--md-primary)" />
      <span class="title-text">快捷网络配置</span>
    </div>

    <div class="switch-row">
      <div class="icon-box" class:on={disableP2p}>
        <Icon name="route" size={20} />
      </div>
      <div class="row-text">
        <div class="row-title">强制中转</div>
        <div class="row-sub">禁用P2P直连，所有流量经服务器中转</div>
      </div>
      <Switch bind:value={disableP2p} onChange={(v) => sm.updateDisableP2p(v)} />
    </div>

    {#if showWindowsOnly}
      <div class="switch-row">
        <div class="icon-box" class:on={broadcastRelay}>
          <Icon name="settings_input_antenna" size={20} />
        </div>
        <div class="row-text">
          <div class="row-title">广播转发</div>
          <div class="row-sub">Windows：将局域网 UDP 广播转发到虚拟网（通常需管理员权限）</div>
        </div>
        <Switch bind:value={broadcastRelay} onChange={(v) => sm.updateEnableUdpBroadcastRelay(v)} />
      </div>

      <div class="switch-row">
        <div class="icon-box" class:on={firewallOn}>
          <Icon name="shield" size={20} />
        </div>
        <div class="row-text">
          <div class="row-title">{t('firewall')}</div>
          <div class="row-sub">{firewallOn ? t('firewall_enabled') : t('firewall_disabled')}</div>
        </div>
        <Switch bind:value={firewallOn} onChange={(v) => sm.setFirewall(v).catch(() => {})} />
      </div>
    {/if}
  </div>
</HomeBox>

<style>
  .qnc {
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

  .switch-row {
    display: flex;
    align-items: center;
    padding: 12px;
    margin-top: 14px;
    background: color-mix(in srgb, var(--md-surface-container-highest) 30%, transparent);
    border: 1px solid var(--md-outline-02);
    border-radius: 8px;
  }
  .switch-row + .switch-row {
    margin-top: 8px;
  }

  .icon-box {
    padding: 8px;
    border-radius: 8px;
    background: var(--md-surface-container-highest);
    color: var(--md-on-surface-variant);
    display: inline-flex;
    flex-shrink: 0;
  }
  .icon-box.on {
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
  }

  .row-text {
    flex: 1 1 0;
    min-width: 0;
    margin-left: 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .row-title {
    font-size: 15px;
    font-weight: 500;
    line-height: 22px;
    color: var(--md-on-surface);
  }
  .row-sub {
    margin-top: 2px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-06);
    word-break: break-word;
  }

  .switch-row :global(.md-switch) {
    margin-left: 8px;
    flex-shrink: 0;
  }
</style>
