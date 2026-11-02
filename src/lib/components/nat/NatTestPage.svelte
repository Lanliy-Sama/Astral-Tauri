<script>
  /**
   * NAT 类型检测页（多 STUN 标准探测）—— 对应 lib/features/nat_test/pages/nat_test_page.dart。
   * initState postFrame 自动测试；三态卡（检测中 / 结果 / 失败）；STUN 下拉 + 检测按钮。
   */
  import { onMount } from 'svelte';
  import SubPage from '$lib/components/common/SubPage.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import Progress from '$lib/components/common/Progress.svelte';
  import DropdownField from '$lib/components/common/DropdownField.svelte';
  import { testNetworkConnectivity } from '$lib/rust/api.js';
  import { snackError } from '$lib/components/common/snackbars.svelte.js';

  let testResult = $state(null);
  let isTesting = $state(false);
  let stunServer = $state('stun.hot-chilli.net');
  let disposed = false;

  const STUN_SERVERS = ['stun.hot-chilli.net', 'stun.miwifi.com', 'stun.l.google.com'];
  const stunOptions = STUN_SERVERS.map((s) => ({ value: s, label: s }));

  onMount(() => {
    /* 进入页面后自动开始测试 */
    testNetwork();
    return () => {
      disposed = true;
    };
  });

  async function testNetwork() {
    if (isTesting) return;
    isTesting = true;
    testResult = null;
    try {
      const result = await testNetworkConnectivity(stunServer);
      if (disposed) return;
      testResult = result;
      isTesting = false;
    } catch (e) {
      if (disposed) return;
      isTesting = false;
      snackError('NAT 检测失败', e?.message ?? `${e}`, {
        action: { label: '重试', onPressed: testNetwork },
      });
    }
  }

  /* ---- NAT 分类：contains 模糊匹配（中英文同判，与 Dart 逐一对应） ---- */
  function isFullCone(t) {
    return (
      t.includes('完全锥形') ||
      t.includes('Full Cone') ||
      t.includes('开放互联网') ||
      t.includes('Open Internet')
    );
  }
  function isRestricted(t) {
    return (
      t.includes('受限锥形') ||
      t.includes('Restricted Cone') ||
      t.includes('端口受限') ||
      t.includes('Port Restricted')
    );
  }
  function isSymmetric(t) {
    return t.includes('对称型') || t.includes('Symmetric');
  }

  function natColor(natType) {
    const t = natType || '';
    if (isFullCone(t)) return 'var(--flutter-green)';
    if (isRestricted(t)) return 'var(--flutter-blue)';
    if (isSymmetric(t)) return 'var(--flutter-orange)';
    return 'var(--flutter-red)';
  }

  function natIcon(natType) {
    const t = natType || '';
    if (isFullCone(t)) return 'check_circle';
    if (isRestricted(t)) return 'info';
    if (isSymmetric(t)) return 'warning';
    return 'error';
  }

  function natDescription(natType) {
    const t = natType || '';
    if (t.includes('完全锥形') || t.includes('Full Cone')) return '最佳连接。任何主机都可发送数据。';
    if (t.includes('受限锥形') || t.includes('Restricted Cone'))
      return '良好连接。只有联系过的主机可发送。';
    if (t.includes('端口受限') || t.includes('Port Restricted'))
      return '较好连接。常见类型，部分联机可用。';
    if (t.includes('对称型') && !t.includes('防火墙')) return '可能影响P2P连接，需要中继服务器。';
    if (t.includes('防火墙') || t.includes('Firewall')) return 'UDP流量受限。请检查防火墙。';
    if (t.includes('被阻止') || t.includes('Blocked')) return 'UDP被阻止。请检查路由器设置。';
    if (t.includes('开放互联网') || t.includes('Open Internet')) return '理想状态！无NAT，直连互联网。';
    return '无法确定。尝试更换STUN服务器。';
  }
</script>

<SubPage title="NAT 类型检测">
  <div class="nat-body">
    {#snippet compact(protocol, natType, latency)}
      <div class="compact">
        <div class="row1">
          <span class="badge">{protocol}</span>
          <span class="flex1"></span>
          <Icon name="speed" size={16} color="var(--md-primary)" />
          <span class="latency">{latency >= 0 ? `${latency}ms` : '-'}</span>
        </div>
        <div class="row2">
          <Icon name={natIcon(natType)} size={20} color={natColor(natType)} filled />
          <span class="nat-type" style="color:{natColor(natType)}">{natType}</span>
        </div>
        <div class="desc">{natDescription(natType)}</div>
      </div>
    {/snippet}

    {#if isTesting}
      <!-- 正在检测中 -->
      <div class="md-card state-card">
        <Progress type="circular" size={24} strokeWidth={3} />
        <div class="state-text">
          <div class="md-title-small">正在检测中...</div>
          <div class="state-sub">使用多 STUN 标准探测</div>
        </div>
      </div>
    {:else if testResult}
      <!-- 检测结果：IPv4 + Divider + IPv6 -->
      <div class="md-card result-card">
        {@render compact('IPv4', testResult.natTypeV4, testResult.ipv4Latency)}
        <div class="divider-gap"></div>
        <div class="divider-wrap"><div class="md-divider"></div></div>
        <div class="divider-gap"></div>
        {@render compact('IPv6', testResult.natTypeV6, testResult.ipv6Latency)}
      </div>
    {:else}
      <!-- 错误状态 -->
      <div class="md-card state-card">
        <Icon name="error_outline" size={24} color="var(--md-error)" />
        <div class="md-title-small">检测失败</div>
      </div>
    {/if}

    <div class="gap16"></div>

    <!-- STUN 服务器选择 + 重新检测 -->
    <div class="stun-row">
      <div class="stun-field">
        <DropdownField
          bind:value={stunServer}
          options={stunOptions}
          label="STUN 服务器"
          disabled={isTesting}
        />
      </div>
      <button class="md-btn md-filled-btn" disabled={isTesting} onclick={testNetwork}>
        <Icon name="refresh" size={18} />检测
      </button>
    </div>
  </div>
</SubPage>

<style>
  /* SingleChildScrollView padding 16 + Column(stretch) */
  .nat-body {
    padding: 16px;
  }
  .gap16 {
    height: 16px;
  }

  /* 三态卡 */
  .state-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px;
  }
  .state-text {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .state-sub {
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
  }
  .result-card {
    padding: 16px;
  }
  /* SizedBox(16) + Divider(高16，线居中) + SizedBox(16) */
  .divider-gap {
    height: 16px;
  }
  .divider-wrap {
    height: 16px;
    display: flex;
    align-items: center;
  }

  /* _buildCompactResult */
  .compact {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .row1 {
    display: flex;
    align-items: center;
    width: 100%;
  }
  .flex1 {
    flex: 1 1 0;
  }
  .badge {
    padding: 4px 8px;
    background: var(--md-primary-container);
    border-radius: 4px;
    color: var(--md-on-primary-container);
    font-size: 12px;
    line-height: 16px;
    letter-spacing: 0.5px;
    font-weight: 700;
  }
  .latency {
    margin-left: 4px;
    font-size: 12px;
    line-height: 16px;
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .row2 {
    display: flex;
    align-items: center;
    width: 100%;
    margin-top: 8px;
  }
  .nat-type {
    flex: 1 1 0;
    min-width: 0;
    margin-left: 8px;
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
  }
  .desc {
    margin-top: 4px;
    padding-left: 28px;
    font-size: 12px;
    line-height: 16px;
    color: var(--md-on-surface-variant);
  }

  /* STUN 行：Expanded DropdownButtonFormField(OutlineInputBorder r8, contentPadding h12v8) + 12 + FilledButton.icon */
  .stun-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .stun-field {
    flex: 1 1 0;
    min-width: 0;
  }
  .stun-field :global(.md-dropdown) {
    border-radius: 8px;
    min-height: 48px;
  }
  .stun-field :global(.md-dropdown .dd-value) {
    padding: 12px 4px 12px 12px;
  }
</style>
