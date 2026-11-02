<script>
  /**
   * SettingsMainPage —— 对应 settings_main_page.dart。
   * IndexedStack 内主页面：ListView padding 16 + 分组头 + 设置卡列表。
   * 子页通过 pushPage() 打开（SubPage AppBar 居中标题 + 返回）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { t } from '$lib/i18n/index.svelte.js';
  import { pushPage } from '$lib/router/page-router.svelte.js';
  import { isAndroid } from '$lib/utils/platform.js';
  import ListenListPage from './ListenListPage.svelte';
  import VpnSegmentPage from './VpnSegmentPage.svelte';
  import NetworkSettingsPage from './NetworkSettingsPage.svelte';
  import StartupPage from './StartupPage.svelte';
  import SoftwareSettingsPage from './SoftwareSettingsPage.svelte';
  import UpdateSettingsPage from './UpdateSettingsPage.svelte';
  import LogsPage from './LogsPage.svelte';
</script>

<div class="page-scroll">
  <!-- 网络设置分组 -->
  <div class="section-header">{t('network_settings')}</div>

  <div class="md-card entry-card">
    <div class="md-list-tile two-line" onclick={() => pushPage(ListenListPage)}>
      <span class="tile-leading">
        <Icon name="list_alt" size={24} color="var(--md-primary)" />
      </span>
      <div class="tile-content">
        <div class="tile-title">{t('listen_list')}</div>
        <div class="tile-subtitle">管理网络监听地址</div>
      </div>
      <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
    </div>
  </div>

  {#if isAndroid()}
    <div class="md-card entry-card">
      <div class="md-list-tile two-line" onclick={() => pushPage(VpnSegmentPage)}>
        <span class="tile-leading">
          <Icon name="vpn_lock" size={24} color="var(--md-primary)" />
        </span>
        <div class="tile-content">
          <div class="tile-title">{t('custom_vpn_segment')}</div>
          <div class="tile-subtitle">配置VPN网段</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>
    </div>
  {/if}

  <div class="md-card entry-card">
    <div class="md-list-tile two-line" onclick={() => pushPage(NetworkSettingsPage)}>
      <span class="tile-leading">
        <Icon name="network_wifi" size={24} color="var(--md-primary)" />
      </span>
      <div class="tile-content">
        <div class="tile-title">高级网络设置</div>
        <div class="tile-subtitle">协议、加密等高级选项</div>
      </div>
      <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
    </div>
  </div>

  <div class="gap-24"></div>

  <!-- 通用设置分组 -->
  <div class="section-header">通用设置</div>

  {#if !isAndroid()}
    <div class="md-card entry-card">
      <div class="md-list-tile two-line" onclick={() => pushPage(StartupPage)}>
        <span class="tile-leading">
          <Icon name="launch" size={24} color="var(--md-primary)" />
        </span>
        <div class="tile-content">
          <div class="tile-title">{t('startup_related')}</div>
          <div class="tile-subtitle">开机启动和自动连接</div>
        </div>
        <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
      </div>
    </div>
  {/if}

  <div class="md-card entry-card">
    <div class="md-list-tile two-line" onclick={() => pushPage(SoftwareSettingsPage)}>
      <span class="tile-leading">
        <Icon name="info" size={24} color="var(--md-primary)" />
      </span>
      <div class="tile-content">
        <div class="tile-title">{t('software_settings')}</div>
        <div class="tile-subtitle">权限和界面设置</div>
      </div>
      <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
    </div>
  </div>

  <div class="md-card entry-card">
    <div class="md-list-tile two-line" onclick={() => pushPage(UpdateSettingsPage)}>
      <span class="tile-leading">
        <Icon name="system_update" size={24} color="var(--md-primary)" />
      </span>
      <div class="tile-content">
        <div class="tile-title">{t('update_settings')}</div>
        <div class="tile-subtitle">自动更新和下载设置</div>
      </div>
      <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
    </div>
  </div>

  <div class="md-card entry-card">
    <div class="md-list-tile two-line" onclick={() => pushPage(LogsPage)}>
      <span class="tile-leading">
        <Icon name="article_outlined" size={24} color="var(--md-primary)" />
      </span>
      <div class="tile-content">
        <div class="tile-title">日志</div>
        <div class="tile-subtitle">查看应用运行日志</div>
      </div>
      <span class="tile-trailing"><Icon name="chevron_right" size={24} /></span>
    </div>
  </div>
</div>

<style>
  .page-scroll {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 16px;
    box-sizing: border-box;
  }
  /* Padding(l16, b8) + SizedBox(8)：titleMedium primary w600 */
  .section-header {
    padding: 0 0 8px 16px;
    margin-bottom: 8px;
    font-size: 16px;
    line-height: 24px;
    font-weight: 600;
    letter-spacing: 0.15px;
    color: var(--md-primary);
  }
  .entry-card {
    margin-bottom: 8px;
  }
  .gap-24 {
    height: 24px;
    flex-shrink: 0;
  }
</style>
