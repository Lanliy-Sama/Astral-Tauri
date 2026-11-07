<script>
  /**
   * 用户信息卡 —— 对照 lib/features/home/widgets/user_ip.dart。
   * 用户名 / 房间选择器 / 虚拟 IP + DHCP 开关；非 idle 时锁定。
   */
  import HomeBox from './HomeBox.svelte';
  import CanvasJump from './CanvasJump.svelte';
  import Icon from '../common/Icon.svelte';
  import TextField from '../common/TextField.svelte';
  import Switch from '../common/Switch.svelte';
  import { sm } from '../../services/ServiceManager.svelte';
  import { CoState } from '../../stores/Enums.svelte';
  import { t } from '../../i18n/I18n.svelte';

  /* 控制器初值只同步一次（对照 initState；此后外部状态变更不回写输入框） */
  let username = $state(sm.playerState.playerName || '');
  let virtualIp = $state(sm.networkConfigState.ipv4 || '');
  let isValidIp = $state(true);
  let canvasOpen = $state(false);

  isValidIp = isValidIPv4(virtualIp);

  /* DHCP 开关本地镜像：非 idle 时点击回弹（对照 Flutter onChanged 内的 idle 守卫） */
  let dhcpLocal = $state(sm.networkConfigState.dhcp);
  $effect(() => {
    dhcpLocal = sm.networkConfigState.dhcp;
  });

  const connectionState = $derived(sm.connectionState.connectionState);
  const isIdle = $derived(connectionState === CoState.idle);
  const isConnected = $derived(connectionState === CoState.connected);
  const dhcp = $derived(sm.networkConfigState.dhcp);
  const roomDisplay = $derived(
    sm.roomState.selectedRoom?.name ?? t('select_room_hint')
  );
  const ipError = $derived(!dhcp && !isValidIp ? t('invalid_ipv4_error') : '');

  /** isValidIPv4WithMask：可选 /mask，4 段 0-255，mask 0-32 */
  function isValidIPv4(ip) {
    const parts = ip.split('/');
    if (parts.length > 2) return false;
    const ipPart = parts[0];
    if (ipPart.length === 0) return false;
    const octets = ipPart.split('.');
    if (octets.length !== 4) return false;
    for (const octet of octets) {
      if (!/^\d+$/.test(octet)) return false;
      const value = Number(octet);
      if (value < 0 || value > 255) return false;
    }
    if (parts.length === 2) {
      const maskPart = parts[1];
      if (maskPart.length === 0) return false;
      if (!/^\d+$/.test(maskPart)) return false;
      const mask = Number(maskPart);
      if (mask < 0 || mask > 32) return false;
    }
    return true;
  }

  function onUsernameChange(value) {
    sm.updatePlayerName(value);
  }

  function onIpChange(value) {
    if (!sm.networkConfigState.dhcp) {
      sm.updateIpv4(value);
      isValidIp = isValidIPv4(value);
    }
  }

  function onDhcpChange(value) {
    if (sm.connectionState.connectionState === CoState.idle) {
      sm.updateDhcp(value);
    } else {
      dhcpLocal = !value;
    }
  }

  function openRoomDialog() {
    if (!isConnected) canvasOpen = true;
  }
</script>

<HomeBox>
  <div class="user-ip-box">
    <div class="title-row">
      <Icon name="person" size={22} color="var(--md-primary)" />
      <span class="title-text">{t('user_info')}</span>
      <div class="title-spacer"></div>
      {#if !isIdle}
        <span class="locked-badge">{t('locked')}</span>
      {/if}
    </div>

    <div class="field-wrap">
      <TextField
        bind:value={username}
        label={t('username')}
        placeholder={t('username_hint')}
        prefixIcon="person"
        floatAlways
        disabled={!isIdle}
        onChange={onUsernameChange}
      />
    </div>

    <div
      class="room-field"
      class:field-disabled={!isIdle}
      class:field-noclick={isConnected}
      onclick={openRoomDialog}
      onkeydown={(e) => {
        if (e.key === 'Enter') openRoomDialog();
      }}
      role="button"
      tabindex={isConnected ? -1 : 0}
    >
      <span class="rf-prefix"><Icon name="apartment" size={24} color="var(--md-primary)" /></span>
      <div class="rf-main">
        <span class="rf-label" class:off={!isIdle}>{t('select_room')}</span>
        <span class="rf-value" class:off={isConnected}>{roomDisplay}</span>
      </div>
      <span class="rf-suffix"><Icon name="menu" size={24} color="var(--md-primary)" /></span>
    </div>

    <div class="ip-row">
      <div class="ip-field">
        <TextField
          bind:value={virtualIp}
          label={t('virtual_network_ip')}
          placeholder={t('virtual_network_ip_hint')}
          prefixIcon="lan"
          floatAlways
          disabled={dhcp || !isIdle}
          errorText={ipError}
          onChange={onIpChange}
        />
      </div>
      <div class="ip-side">
        <Switch bind:value={dhcpLocal} onChange={onDhcpChange} />
        <span class="ip-mode">{dhcp ? t('automatic') : t('manual')}</span>
      </div>
    </div>

    {#if dhcp}
      <div class="dhcp-notice">{t('auto_assign_ip_notice')}</div>
    {:else}
      <div class="tail-spacer"></div>
    {/if}
  </div>

  <CanvasJump
    bind:open={canvasOpen}
    rooms={sm.roomState.rooms}
    onselect={(room) => sm.setRoom(room)}
  />
</HomeBox>

<style>
  .user-ip-box {
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
  .locked-badge {
    padding: 4px 8px;
    background: var(--md-secondary-container);
    border-radius: 12px;
    color: var(--md-on-secondary-container);
    font-size: 12px;
    font-weight: 300;
    line-height: 16px;
  }

  .field-wrap {
    margin-top: 14px;
  }

  /* 输入框：contentPadding v12 h12（对照原版 InputDecoration） */
  .user-ip-box :global(.md-text-field) {
    min-height: 50px;
  }
  .user-ip-box :global(.md-text-field input) {
    padding: 12px 12px;
  }
  .user-ip-box :global(.md-text-field input::placeholder) {
    color: var(--md-on-surface-variant);
    opacity: 1;
  }
  /* 浮动标签（floatingLabelBehavior.always）：卡片底色衬底 */
  .user-ip-box :global(.md-text-field .tf-label.floated) {
    top: 0;
    transform: translateY(-9px);
    height: auto;
    font-size: 12px;
    line-height: 16px;
    background: var(--md-surface-container-low);
    padding: 0 4px;
    margin-left: -4px;
  }
  /* 原版 prefixIcon 显式指定 primary 色（含禁用/错误态） */
  .user-ip-box :global(.md-text-field .tf-prefix .ms) {
    color: var(--md-primary) !important;
  }
  /* 禁用态：透明度对齐 Flutter（填充 4%、文字 38%） */
  .user-ip-box :global(.md-text-field.disabled) {
    background: color-mix(in srgb, var(--md-on-surface) 4%, transparent);
  }
  .user-ip-box :global(.md-text-field.disabled input) {
    color: var(--md-on-surface-038);
  }

  /* 房间选择器（InkWell + InputDecorator） */
  .room-field {
    margin-top: 14px;
    display: flex;
    align-items: center;
    min-height: 50px;
    border: 1px solid var(--md-outline);
    border-radius: 4px;
    cursor: pointer;
    transition: border-color 100ms ease;
  }
  .room-field:hover {
    border-color: var(--md-on-surface);
  }
  .room-field.field-disabled,
  .room-field.field-disabled:hover {
    border-color: var(--md-on-surface-012);
  }
  .room-field.field-noclick {
    cursor: default;
  }
  .rf-prefix,
  .rf-suffix {
    display: inline-flex;
    align-items: center;
    align-self: stretch;
    padding: 0 12px;
    flex-shrink: 0;
  }
  .rf-main {
    flex: 1 1 0;
    min-width: 0;
    position: relative;
    padding: 12px 12px;
  }
  .rf-label {
    position: absolute;
    top: -8px;
    left: 0;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
    background: var(--md-surface-container-low);
    padding: 0 4px;
    margin-left: -4px;
    white-space: nowrap;
  }
  .rf-label.off {
    color: var(--md-on-surface-038);
  }
  .rf-value {
    display: block;
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rf-value.off {
    color: var(--md-on-surface-038);
  }

  .ip-row {
    margin-top: 9px;
    height: 60px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ip-field {
    flex: 1 1 0;
    min-width: 0;
  }
  .ip-side {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .ip-mode {
    font-size: 12px;
    line-height: 16px;
  }

  .dhcp-notice {
    margin-top: 8px;
    font-size: 12px;
    line-height: 16px;
  }
  .tail-spacer {
    height: 12px;
  }
</style>
