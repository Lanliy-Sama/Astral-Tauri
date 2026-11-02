<script>
  /**
   * SoftwareSettingsPage —— 对应 software_settings_page.dart（桌面路径）。
   * Android 权限分支（install_mobile / Android 设置卡）按规格桌面忽略。
   */
  import SubPage from '$lib/components/common/SubPage.svelte';
  import SettingsCard from './parts/SettingsCard.svelte';
  import BareDropdown from './parts/BareDropdown.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { t } from '$lib/i18n/index.svelte.js';
  import { isAndroid } from '$lib/utils/platform.js';

  const retryOptions = [1, 2, 3, 5, 10].map((n) => ({ value: n, label: `${n} 次` }));

  /* SwitchListTile 整行可点；点 Switch 本体时避免与 tile 冒泡双触发 */
  function switchTileClick(e, current, update) {
    if (e.target.closest('.md-switch')) return;
    update(!current);
  }
</script>

<SubPage title={t('software_settings')}>
  <div class="page-list">
    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="settings" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">{t('software_settings')}</div>
          <div class="tile-subtitle">{t('software_behavior_desc')}</div>
        </div>
      </div>
      <div class="md-divider"></div>

      {#if !isAndroid()}
        <div
          class="md-switch-list-tile"
          onclick={(e) => switchTileClick(e, sm.windowState.closeMinimize, sm.updateCloseMinimize)}
        >
          <div class="tile-content">
            <div class="tile-title">{t('minimize')}</div>
            <div class="tile-subtitle">{t('minimize_desc')}</div>
          </div>
          <Switch
            value={sm.windowState.closeMinimize}
            onChange={(v) => sm.updateCloseMinimize(v)}
          />
        </div>
      {/if}

      <div class="md-list-tile two-line no-cursor">
        <div class="tile-content">
          <div class="tile-title">{t('player_list_card')}</div>
          <div class="tile-subtitle">{t('player_list_card_desc')}</div>
        </div>
      </div>
      <!-- Padding(16,0,16,12) + SegmentedButton -->
      <div class="segment-wrap">
        <div class="md-segmented">
          <button
            class="segment {sm.displayState.userListStyle === 0 ? 'selected' : ''}"
            onclick={() => sm.setUserListStyle(0)}>简约</button
          >
          <button
            class="segment {sm.displayState.userListStyle === 1 ? 'selected' : ''}"
            onclick={() => sm.setUserListStyle(1)}>详细</button
          >
          <button
            class="segment {sm.displayState.userListStyle === 2 ? 'selected' : ''}"
            onclick={() => sm.setUserListStyle(2)}>列表</button
          >
        </div>
      </div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.appSettingsState.reduceAnimationUpdates, sm.updateReduceAnimationUpdates)}
      >
        <div class="tile-content">
          <div class="tile-title">减少动画更新</div>
          <div class="tile-subtitle">降低拓扑图与连线动画刷新频率，减少 GPU 占用</div>
        </div>
        <Switch
          value={sm.appSettingsState.reduceAnimationUpdates}
          onChange={(v) => sm.updateReduceAnimationUpdates(v)}
        />
      </div>
    </SettingsCard>

    <div class="gap-16"></div>

    <SettingsCard>
      <div class="md-list-tile two-line no-cursor">
        <span class="tile-leading"><Icon name="sync" size={24} color="var(--md-on-surface-variant)" /></span>
        <div class="tile-content">
          <div class="tile-title">连接设置</div>
          <div class="tile-subtitle">配置连接重试行为</div>
        </div>
      </div>
      <div class="md-divider"></div>

      <div
        class="md-switch-list-tile"
        onclick={(e) =>
          switchTileClick(e, sm.appSettingsState.autoRetryOnFailure, sm.updateAutoRetryOnFailure)}
      >
        <div class="tile-content">
          <div class="tile-title">连接失败自动重试</div>
          <div class="tile-subtitle">连接失败时自动重新尝试连接</div>
        </div>
        <Switch
          value={sm.appSettingsState.autoRetryOnFailure}
          onChange={(v) => sm.updateAutoRetryOnFailure(v)}
        />
      </div>

      {#if sm.appSettingsState.autoRetryOnFailure}
        <div class="md-list-tile two-line no-cursor">
          <div class="tile-content">
            <div class="tile-title">最大重试次数</div>
            <div class="tile-subtitle">当前设置为 {sm.appSettingsState.maxRetryCount} 次</div>
          </div>
          <!-- SizedBox(width:100) + DropdownButton(isExpanded) -->
          <span class="tile-trailing retry-dd">
            <BareDropdown
              value={sm.appSettingsState.maxRetryCount}
              options={retryOptions}
              underline={true}
              width={100}
              onChange={(v) => sm.updateMaxRetryCount(v)}
            />
          </span>
        </div>
      {/if}
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
  .segment-wrap {
    padding: 0 16px 12px;
  }
  /* SegmentedButton 段等宽铺满 */
  .segment-wrap .md-segmented {
    display: flex;
    width: 100%;
  }
  .segment-wrap .segment {
    flex: 1 1 0;
  }
</style>
