<script>
  /**
   * StartupPage —— 对应 startup_page.dart。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { t } from '$lib/i18n/I18n.svelte';

  /* SwitchListTile 整行可点；点 Switch 本体时避免与 tile 冒泡双触发 */
  function switchTileClick(e, current, update) {
    if (e.target.closest('.md-switch')) return;
    update(!current);
  }
</script>

<SubPage title={t('startup_related')}>
  <div class="page-list">
    <SettingsCard>
      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.startupState.startup, sm.updateStartup)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('startup_on_boot')}</div>
          <div class="tile-subtitle">{t('startup_on_boot_desc')}</div>
        </div>
        <Switch value={sm.startupState.startup} onChange={(v) => sm.updateStartup(v)} />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) => switchTileClick(e, sm.startupState.startupMinimize, sm.updateStartupMinimize)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('startup_minimize')}</div>
          <div class="tile-subtitle">{t('startup_minimize_desc')}</div>
        </div>
        <Switch value={sm.startupState.startupMinimize} onChange={(v) => sm.updateStartupMinimize(v)} />
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.startupState.startupAutoConnect, sm.updateStartupAutoConnect)}
      >
        <div class="tile-content">
          <div class="tile-title">{t('startup_auto_connect')}</div>
          <div class="tile-subtitle">{t('startup_auto_connect_desc')}</div>
        </div>
        <Switch value={sm.startupState.startupAutoConnect} onChange={(v) => sm.updateStartupAutoConnect(v)} />
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <SettingsCard header={t('startup_description')}>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="power_settings_new" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('startup_on_boot_title')}</div>
          <div class="tile-subtitle">{t('startup_on_boot_info')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="minimize" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('startup_minimize_title')}</div>
          <div class="tile-subtitle">{t('startup_minimize_info')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="play_arrow" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('startup_auto_connect_title')}</div>
          <div class="tile-subtitle">{t('startup_auto_connect_info')}</div>
        </div>
      </div>
    </SettingsCard>
  </div>
</SubPage>

<style>
  .page-list {
    padding: 16px;
  }
  .gap-16 {
    height: 16px;
  }
</style>
