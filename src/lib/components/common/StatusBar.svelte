<script>
  /**
   * 状态栏（AppBar 36px）—— 标题栏拖动区 + 主题/语言菜单 + 窗口控制。
   * 小窗口模式（Android 且 宽<300 或 高<400）使用紧凑变体。
   */
  import Icon from './Icon.svelte';
  import IconMenu from './IconMenu.svelte';
  import WindowControls from './WindowControls.svelte';
  import ThemeSelector from './ThemeSelector.svelte';
  import { sm } from '../../services/service-manager.svelte.js';
  import { setLocale, t } from '../../i18n/index.svelte.js';
  import { isTauri, isDesktopPlatform } from '../../utils/platform.js';

  let { isSmallWindow = false } = $props();

  let colorPickerOpen = $state(false);
  let isChristmas = $state(false);

  $effect(() => {
    const now = new Date();
    isChristmas = now.getMonth() === 11 && now.getDate() >= 22 && now.getDate() <= 28;
  });

  const themeModeItems = $derived.by(() => {
    const modeIcons = { system: 'auto_mode', light: 'wb_sunny', dark: 'nightlight_round' };
    const modeLabels = { system: '跟随系统', light: '亮色模式', dark: '暗色模式' };
    return ['system', 'light', 'dark'].map((m) => ({
      value: m,
      label: modeLabels[m],
      icon: modeIcons[m],
      checkMark: sm.themeState.themeMode === m,
    }));
  });

  const themeModeIcon = $derived.by(() => {
    const modeIcons = { system: 'auto_mode', light: 'wb_sunny', dark: 'nightlight_round' };
    return modeIcons[sm.themeState.themeMode] || 'auto_mode';
  });

  /* 原版 buildThemeModeText：按钮 tooltip 为当前模式文字 */
  const themeModeText = $derived.by(() => {
    const labels = { system: '跟随系统', light: '亮色模式', dark: '暗色模式' };
    return labels[sm.themeState.themeMode] || '跟随系统';
  });

  /* 原版 buildLanguageMenuItems：国旗 + 8px + 文字，无勾选标记 */
  const languageItems = $derived([
    { value: 'zh', flag: '🇨🇳', label: '简体中文' },
    { value: 'en', flag: '🇺🇸', label: 'English' },
  ]);
  function onSelectThemeMode(mode) {
    sm.updateThemeMode(mode);
  }

  function onSelectLanguage(locale) {
    setLocale(locale);
  }

  function startDrag(e) {
    if (!isDesktopPlatform() && isTauri()) return;
    // Tauri 通过 data-tauri-drag-region 属性处理；此处无需 JS
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="status-bar {isSmallWindow ? 'small' : ''}"
  data-tauri-drag-region
  onpointerdown={startDrag}
>
  {#if isSmallWindow}
    <!-- 小窗口：纯文本标题 -->
    <div class="app-title small" data-tauri-drag-region>{sm.appSettingsState.appName}</div>
    <div class="actions" data-tauri-drag-region style="pointer-events:none">
      <div style="pointer-events:auto; display:flex">
        <IconMenu
          icon={themeModeIcon}
          iconSize={16}
          buttonPadding={4}
          tooltip={themeModeText}
          items={themeModeItems}
          onSelect={onSelectThemeMode}
        />
        <IconMenu
          icon="language"
          iconSize={16}
          buttonPadding={4}
          tooltip={t('language')}
          items={languageItems}
          onSelect={onSelectLanguage}
        />
      </div>
    </div>
  {:else}
    <div class="app-title" data-tauri-drag-region>{sm.appSettingsState.appName}</div>
    <div class="actions" data-tauri-drag-region style="pointer-events:none">
      <div style="pointer-events:auto; display:flex; align-items:center">
        {#if isChristmas}
          <!-- 原版：Kevin 与 🎄 无间距，其后 6px；整体左右 padding 8 -->
          <div class="christmas">
            <span class="md-body-medium" style="font-weight:600; color:var(--md-on-primary-container)">爱你们的Kevin</span>
            <span style="font-size:18px">🎄</span>
            <span style="width:6px"></span>
            <span class="md-body-medium" style="font-weight:600; color:var(--md-on-primary-container)">圣诞快乐</span>
            <span style="width:6px"></span>
            <span style="font-size:16px">🎅</span>
          </div>
          <div style="width:8px"></div>
        {/if}
        <IconMenu
          icon={themeModeIcon}
          iconSize={20}
          buttonPadding={8}
          tooltip={themeModeText}
          items={themeModeItems}
          onSelect={onSelectThemeMode}
        />
        <button
          class="md-icon-btn"
          style="padding:4px; color:var(--md-on-primary-container)"
          data-tip="选择主题颜色"
          onclick={() => (colorPickerOpen = true)}
        >
          <Icon name="color_lens" size={20} color="var(--md-on-primary-container)" />
        </button>
        <IconMenu
          icon="language"
          iconSize={20}
          buttonPadding={8}
          tooltip={t('language')}
          items={languageItems}
          onSelect={onSelectLanguage}
        />
        <WindowControls />
      </div>
    </div>
  {/if}
</div>

<ThemeSelector bind:open={colorPickerOpen} />

<style>
  .status-bar {
    height: 36px;
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
    display: flex;
    align-items: center;
    padding: 0 0 0 16px;
    flex-shrink: 0;
  }
  .status-bar.small {
    height: 36px;
  }
  .app-title {
    font-size: 18px;
    font-weight: 700;
    letter-spacing: 1.5px;
    background: linear-gradient(90deg, var(--md-primary), var(--md-tertiary));
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .app-title.small {
    font-size: 14px;
    font-weight: 700;
    letter-spacing: normal;
  }
  .actions {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }
  .christmas {
    display: flex;
    align-items: center;
    padding: 0 8px;
  }
</style>
