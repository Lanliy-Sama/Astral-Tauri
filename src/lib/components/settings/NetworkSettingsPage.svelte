<script>
  /**
   * NetworkSettingsPage —— 对应 network_settings_page.dart + network_settings_sections.dart。
   * 三卡：基础 / 高级 / 跃点。协议与压缩下拉是 ListTile trailing 的紧凑下拉（隐藏下划线）。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import BareDropdown from './parts/BareDropdown.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Dialog from '$lib/components/common/Dialog.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { t } from '$lib/i18n/I18n.svelte';
  import { getAllInterfacesMetrics } from '$lib/rust/Api.svelte';
  import { snackError } from '$lib/components/common/SnackbarStore.svelte';

  const protocolOptions = [
    { value: 'tcp', label: 'TCP' },
    { value: 'udp', label: 'UDP' },
    { value: 'faketcp', label: 'FakeTCP' },
    { value: 'ws', label: 'WebSocket' },
    { value: 'wss', label: 'WSS' },
    { value: 'quic', label: 'QUIC' },
  ];
  const compressOptions = $derived([
    { value: 1, label: t('no_compression') },
    { value: 2, label: t('high_performance_compression') },
  ]);

  /* 空值显示 'tcp' */
  const protocol = $derived(sm.networkConfigState.defaultProtocol || 'tcp');

  /* SwitchListTile 整行可点；点 Switch 本体时避免与 tile 冒泡双触发 */
  function switchTileClick(e, current, update) {
    if (e.target.closest('.md-switch')) return;
    update(!current);
  }

  /* 跃点开关（存 AllSettings，非响应式状态，页面本地持有） */
  let autoSetMTU = $state(true);
  $effect(() => {
    sm.getAutoSetMTU().then((v) => (autoSetMTU = v));
  });

  async function onAutoSetHop(value) {
    autoSetMTU = value;
    await sm.setAutoSetMTU(value);
  }

  /* SOCKS5 端口（onFieldSubmitted：0 < p <= 65535） */
  function submitPort(value) {
    const port = Number(value);
    if (Number.isFinite(port) && port > 0 && port <= 65535) {
      sm.updateSocks5Port(port);
    }
  }

  /* MTU（回车提交：1280–1500；非法值回落当前值，{#key} 重建输入框） */
  function submitMtu(value) {
    const v = Number(value);
    if (Number.isInteger(v) && v >= 1280 && v <= 1500) {
      sm.updateMtu(v);
    }
  }

  /* 跃点列表对话框 */
  let hopOpen = $state(false);
  let hopList = $state([]);

  async function showHopList() {
    try {
      const result = await getAllInterfacesMetrics();
      hopList = result;
      hopOpen = true;
    } catch {
      snackError('错误', t('get_hop_list_failed'));
    }
  }
</script>

<SubPage title={t('network_settings')}>
  <div class="page-list">
    <!-- CardA 基础 -->
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('p2p_hole_punching')}</div>
          <div class="tile-subtitle">{t('preferred_protocol')}</div>
        </div>
        <span class="tile-trailing">
          <!-- Container(r8) + Padding(h8) + DropdownButton（隐藏 underline） -->
          <span class="dd-wrap">
            <BareDropdown
              value={protocol}
              options={protocolOptions}
              onChange={(v) => sm.updateDefaultProtocol(v)}
            />
          </span>
        </span>
      </div>
      <div class="md-divider"></div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.networkConfigState.enableEncryption, sm.updateEnableEncryption)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('enable_encryption')}</div>
          <!-- 原版副标题复用 auto_set_mtu（怪异映射，保留） -->
          <div class="tile-subtitle">{t('auto_set_mtu')}</div>
        </div>
        <Switch
          value={sm.networkConfigState.enableEncryption}
          onChange={(v) => sm.updateEnableEncryption(v)}
        />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.networkConfigState.latencyFirst, sm.updateLatencyFirst)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('latency_first')}</div>
          <div class="tile-subtitle">{t('latency_first_desc')}</div>
        </div>
        <Switch value={sm.networkConfigState.latencyFirst} onChange={(v) => sm.updateLatencyFirst(v)} />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.networkConfigState.disableP2p, sm.updateDisableP2p)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('disable_p2p')}</div>
          <div class="tile-subtitle">{t('disable_p2p_desc')}</div>
        </div>
        <Switch value={sm.networkConfigState.disableP2p} onChange={(v) => sm.updateDisableP2p(v)} />
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <!-- CardB 高级 -->
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="settings_ethernet" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('advanced_network_settings')}</div>
          <div class="tile-subtitle">{t('advanced_network_settings_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <!-- MTU：数字输入（回车提交，1280–1500） -->
      <div class="md-list-tile two-line no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('mtu')}</div>
          <div class="tile-subtitle">{t('mtu_desc')}</div>
        </div>
        <span class="tile-trailing">
          {#key sm.networkConfigState.mtu}
            <input
              class="port-input"
              type="text"
              inputmode="numeric"
              value={String(sm.networkConfigState.mtu)}
              onkeydown={(e) => {
                if (e.key === 'Enter') submitMtu(e.currentTarget.value);
              }}
            />
          {/key}
        </span>
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(
            e,
            sm.networkConfigState.disableUdpHolePunching,
            sm.updateDisableUdpHolePunching,
          )}
      >
        <div class="tile-content">
          <div class="tile-title">{t('disable_udp_hole_punching')}</div>
          <div class="tile-subtitle">{t('disable_udp_hole_punching_desc')}</div>
        </div>
        <Switch
          value={sm.networkConfigState.disableUdpHolePunching}
          onChange={(v) => sm.updateDisableUdpHolePunching(v)}
        />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(
            e,
            sm.networkConfigState.disableTcpHolePunching,
            sm.updateDisableTcpHolePunching,
          )}
      >
        <div class="tile-content">
          <div class="tile-title">{t('disable_tcp_hole_punching')}</div>
          <div class="tile-subtitle">{t('disable_tcp_hole_punching_desc')}</div>
        </div>
        <Switch
          value={sm.networkConfigState.disableTcpHolePunching}
          onChange={(v) => sm.updateDisableTcpHolePunching(v)}
        />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(
            e,
            sm.networkConfigState.disableSymHolePunching,
            sm.updateDisableSymHolePunching,
          )}
      >
        <div class="tile-content">
          <div class="tile-title">{t('disable_sym_hole_punching')}</div>
          <div class="tile-subtitle">{t('disable_sym_hole_punching_desc')}</div>
        </div>
        <Switch
          value={sm.networkConfigState.disableSymHolePunching}
          onChange={(v) => sm.updateDisableSymHolePunching(v)}
        />
      </div>

      <div class="md-list-tile two-line no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('compression_algorithm')}</div>
          <div class="tile-subtitle">{t('compression_algorithm_desc')}</div>
        </div>
        <span class="tile-trailing">
          <span class="dd-wrap">
            <BareDropdown
              value={sm.networkConfigState.dataCompressAlgo}
              options={compressOptions}
              onChange={(v) => sm.updateDataCompressAlgo(v)}
            />
          </span>
        </span>
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.networkConfigState.enableKcpProxy, sm.updateEnableKcpProxy)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('enable_kcp_proxy')}</div>
          <div class="tile-subtitle">{t('enable_kcp_proxy_desc')}</div>
        </div>
        <Switch value={sm.networkConfigState.enableKcpProxy} onChange={(v) => sm.updateEnableKcpProxy(v)} />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.networkConfigState.bindDevice, sm.updateBindDevice)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('bind_device')}</div>
          <div class="tile-subtitle">{t('bind_device_desc')}</div>
        </div>
        <Switch value={sm.networkConfigState.bindDevice} onChange={(v) => sm.updateBindDevice(v)} />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.networkConfigState.noTun, sm.updateNoTun)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('tun_device')}</div>
          <div class="tile-subtitle">{t('tun_device_desc')}</div>
        </div>
        <Switch value={sm.networkConfigState.noTun} onChange={(v) => sm.updateNoTun(v)} />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.networkConfigState.enableSocks5, sm.updateEnableSocks5)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('enable_socks5')}</div>
          <div class="tile-subtitle">{t('enable_socks5_desc')}</div>
        </div>
        <Switch value={sm.networkConfigState.enableSocks5} onChange={(v) => sm.updateEnableSocks5(v)} />
      </div>

      {#if sm.networkConfigState.enableSocks5}
        <div class="md-list-tile two-line no-cursor">
          <div class="tile-content">
            <div class="tile-title">{t('socks5_port')}</div>
            <div class="tile-subtitle">{t('socks5_port_desc')}</div>
          </div>
          <span class="tile-trailing">
            <!-- ValueKey(端口)：端口变更时重建输入框 -->
            {#key sm.networkConfigState.socks5Port}
              <input
                class="port-input"
                type="text"
                inputmode="numeric"
                value={String(sm.networkConfigState.socks5Port)}
                onkeydown={(e) => {
                  if (e.key === 'Enter') submitPort(e.currentTarget.value);
                }}
              />
            {/key}
          </span>
        </div>

        <div class="md-list-tile no-cursor">
          <div class="tile-content">
            <div class="hint-title">
              {t('socks5_address_hint', { port: sm.networkConfigState.socks5Port })}
            </div>
          </div>
        </div>
      {/if}
    </SettingsCard>

    <div class="gap-16"></div>

    <!-- CardC 跃点 -->
    <SettingsCard>
      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, autoSetMTU, onAutoSetHop)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('auto_set_hop')}</div>
          <div class="tile-subtitle">{t('auto_set_hop_desc')}</div>
        </div>
        <Switch value={autoSetMTU} onChange={(v) => onAutoSetHop(v)} />
      </div>
      <div class="md-divider"></div>

      <div class="md-list-tile" onclick={showHopList}>
        <span class="tile-leading"><Icon name="list" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('view_hop_list')}</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>
    </SettingsCard>
  </div>
</SubPage>

<!-- 网卡跃点列表对话框 -->
<Dialog bind:open={hopOpen} title={t('network_adapter_hop_list')}>
  <div class="hop-list">
    {#each hopList as entry, index (index)}
      <div class="hop-item">{entry[0]}: {entry[1]}</div>
    {/each}
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={() => (hopOpen = false)}>{t('close')}</button>
  {/snippet}
</Dialog>

<style>
  .page-list {
    padding: 16px;
  }
  .gap-16 {
    height: 16px;
  }
  /* Container(borderRadius 8) + Padding(horizontal 8) */
  .dd-wrap {
    display: inline-flex;
    padding: 0 8px;
    border-radius: 8px;
  }
  /* TextFormField(isDense, 数字右对齐, r4, 宽 100) */
  .port-input {
    width: 100px;
    height: 40px;
    border: 1px solid var(--md-outline);
    border-radius: 4px;
    background: transparent;
    text-align: right;
    padding: 0 8px;
    font-size: 16px;
    line-height: 24px;
    color: var(--md-on-surface);
    outline: none;
    box-sizing: border-box;
    font-family: inherit;
  }
  .port-input:hover {
    border-color: var(--md-on-surface);
  }
  .port-input:focus {
    border: 2px solid var(--md-primary);
    padding: 0 7px;
  }
  /* 连接地址提示（primary w500） */
  .hint-title {
    font-size: 16px;
    line-height: 24px;
    font-weight: 500;
    color: var(--md-primary);
  }
  .hop-item {
    padding: 4px 0;
    color: var(--md-on-surface);
  }
</style>
