<script>
  /**
   * 关于卡 —— 对照 lib/features/home/widgets/about_home.dart。
   * 软件版本（有新版时 "当前 -> 最新" + 上箭头）与内核版本（平台图标 + 徽章）。
   */
  import { onMount } from 'svelte';
  import HomeBox from './HomeBox.svelte';
  import Icon from '../common/Icon.svelte';
  import { sm } from '../../services/ServiceManager.svelte';
  import { t } from '../../i18n/I18n.svelte';
  import { parsePlatformVersion, compareVersions, stripVPrefix } from '../../utils/Display.svelte';
  import { easytierVersion } from '../../rust/Api.svelte';

  let kernelVersion = $state('');

  onMount(async () => {
    kernelVersion = await easytierVersion();
  });

  const platformInfo = $derived(parsePlatformVersion(kernelVersion));

  const hasNewVersion = $derived(
    sm.updateState.latestVersion != null &&
      sm.updateState.latestVersion !== '' &&
      compareVersions(sm.appVersion, sm.updateState.latestVersion) < 0
  );

  const versionText = $derived(
    hasNewVersion
      ? `${stripVPrefix(sm.appVersion)} -> ${stripVPrefix(sm.updateState.latestVersion)}`
      : stripVPrefix(sm.appVersion)
  );
</script>

<HomeBox>
  <div class="about-home">
    <div class="title-row">
      <Icon name="info_outline" size={22} color="var(--md-primary)" />
      <span class="title-text">{t('about')}</span>
    </div>

    <div class="ver-row">
      <Icon name="smartphone" size={20} color="var(--md-primary)" />
      <span class="ver-label">{t('software_version')}: </span>
      <span class="ver-current">
        {versionText}
        {#if hasNewVersion}
          <span class="new-arrow">
            <Icon name="arrow_upward" size={16} color="var(--md-primary)" />
          </span>
        {/if}
      </span>
    </div>

    <div class="ver-row kernel">
      <Icon name={platformInfo.icon} size={20} color="var(--md-primary)" />
      <span class="ver-label">{t('kernel_version')}: </span>
      <span class="ver-value">{platformInfo.number}</span>
      {#if platformInfo.name}
        <span class="platform-badge">{platformInfo.name}</span>
      {/if}
    </div>
  </div>
</HomeBox>

<style>
  .about-home {
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

  .ver-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
  }
  .ver-row.kernel {
    margin-top: 4px;
  }

  .ver-label {
    font-weight: 700;
  }
  .ver-current {
    color: var(--md-secondary);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .ver-value {
    color: var(--md-secondary);
    word-break: break-all;
  }
  .new-arrow {
    display: inline-flex;
    align-items: center;
  }

  .platform-badge {
    padding: 2px 6px;
    background: var(--md-primary-container);
    border-radius: 4px;
    color: var(--md-on-primary-container);
    font-size: 12px;
    font-weight: 500;
    line-height: 16px;
  }
</style>
