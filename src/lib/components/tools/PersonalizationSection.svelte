<script>
  /** 工具页内联个性化设置，不创建二级页面。 */
  import SettingsCard from '../settings/parts/SettingsCard.svelte';
  import Switch from '$lib/components/common/Switch.svelte';
  import Icon from '$lib/components/common/Icon.svelte';
  import { sm } from '$lib/services/ServiceManager.svelte';
  import { t } from '$lib/i18n/I18n.svelte';
  import { isTauri } from '$lib/utils/Platform.svelte';
  import { snackError, snackSuccess, snackInfo } from '$lib/components/common/SnackbarStore.svelte';

  const BG_MIME_BY_EXT = {
    jpg: 'image/jpeg', jpeg: 'image/jpeg', jfif: 'image/jpeg', pjpe: 'image/jpeg', pjp: 'image/jpeg',
    png: 'image/png', apng: 'image/apng', gif: 'image/gif', webp: 'image/webp',
    bmp: 'image/bmp', dib: 'image/bmp', svg: 'image/svg+xml',
    ico: 'image/x-icon', cur: 'image/x-icon', tif: 'image/tiff', tiff: 'image/tiff',
    avif: 'image/avif', heic: 'image/heic', heif: 'image/heif', heics: 'image/heic', jxl: 'image/jxl',
  };
  const MIME_TO_EXT = {
    'image/jpeg': 'jpg', 'image/png': 'png', 'image/apng': 'apng', 'image/gif': 'gif',
    'image/webp': 'webp', 'image/bmp': 'bmp', 'image/svg+xml': 'svg',
    'image/x-icon': 'ico', 'image/tiff': 'tiff', 'image/avif': 'avif',
    'image/heic': 'heic', 'image/heif': 'heif', 'image/heic-sequence': 'heic', 'image/jxl': 'jxl',
  };
  const BG_ACCEPT_ATTR = `image/*,${Object.keys(BG_MIME_BY_EXT).map((ext) => `.${ext}`).join(',')}`;
  const BG_CHUNK = 1 << 20;
  let bgFileEl = null;
  let bgBusy = $state(false);

  function resolveImageExt(file) {
    const ext = (file.name.split('.').pop() || '').toLowerCase();
    if (BG_MIME_BY_EXT[ext]) return ext;
    const mime = file.type?.startsWith('image/') ? file.type : null;
    return MIME_TO_EXT[mime] || null;
  }
  function blobToB64(blob) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).split(',')[1] || '');
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(blob);
    });
  }
  function probeImage(url, timeoutMs = 30000) {
    return new Promise((resolve, reject) => {
      const image = new Image();
      const timer = setTimeout(() => { image.src = ''; reject(new Error('timeout')); }, timeoutMs);
      image.onload = () => { clearTimeout(timer); resolve(); };
      image.onerror = () => { clearTimeout(timer); reject(new Error('decode failed')); };
      image.src = url;
    });
  }

  async function onBgFileChosen(event) {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file || bgBusy) return;
    const ext = resolveImageExt(file);
    if (!ext) {
      snackError(t('bg_invalid_type'), t('bg_invalid_type_msg'));
      return;
    }
    bgBusy = true;
    try {
      let url;
      if (isTauri()) {
        if (file.size > 8 * BG_CHUNK) snackInfo(t('bg_processing'), t('bg_processing_msg'));
        const { invoke, convertFileSrc } = await import('@tauri-apps/api/core');
        await invoke('bg_write_begin', { ext });
        for (let offset = 0; offset < file.size; offset += BG_CHUNK) {
          const b64 = await blobToB64(file.slice(offset, Math.min(offset + BG_CHUNK, file.size)));
          await invoke('bg_write_append', { b64 });
        }
        const info = await invoke('bg_write_end');
        url = convertFileSrc('bg', 'astralbg') + '?v=' + info.version;
      } else {
        url = URL.createObjectURL(file);
      }
      try {
        await probeImage(url);
      } catch {
        if (!isTauri()) URL.revokeObjectURL(url);
        snackError(t('bg_decode_failed'), t('bg_decode_failed_msg'));
        return;
      }
      await sm.updateCustomBackground(url);
      snackSuccess(t('bg_applied'), t('bg_applied_msg'));
    } catch (error) {
      snackError(t('bg_read_failed'), String(error?.message || error));
    } finally {
      bgBusy = false;
    }
  }

  async function clearBackground() {
    if (bgBusy) return;
    if (isTauri()) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('clear_custom_background');
      } catch (error) {
        console.warn('清除背景文件失败:', error);
      }
    }
    await sm.updateCustomBackground(null);
    snackInfo(t('bg_cleared'), t('bg_cleared_msg'));
  }
</script>

<SettingsCard>
  <div class="md-list-tile two-line no-cursor">
    <span class="tile-leading"><Icon name="wallpaper" size={24} color="var(--md-primary)" /></span>
    <div class="tile-content">
      <div class="tile-title">{t('custom_background')}</div>
      <div class="tile-subtitle">{t('custom_background_desc')}</div>
    </div>
    {#if sm.appSettingsState.customBackground}
      <span class="tile-trailing bg-preview-wrap">
        <img class="bg-preview" src={sm.appSettingsState.customBackground} alt="当前背景预览" />
      </span>
    {/if}
  </div>
  <div class="md-divider"></div>
  <div class="bg-actions">
    <button class="md-btn md-tonal-btn" onclick={() => bgFileEl?.click()} disabled={bgBusy}>{t('choose_image')}</button>
    <button class="md-btn md-text-btn" onclick={clearBackground} disabled={!sm.appSettingsState.customBackground || bgBusy}>{t('reset_background')}</button>
    {#if bgBusy}<span class="bg-busy-hint">{t('bg_processing')}…</span>{/if}
    <input type="file" accept={BG_ACCEPT_ATTR} bind:this={bgFileEl} onchange={onBgFileChosen} hidden />
  </div>
  <div class="md-divider"></div>
  <div class="md-switch-list-tile no-cursor">
    <div class="tile-content">
      <div class="tile-title">液态玻璃效果</div>
      <div class="tile-subtitle">为卡片、按钮、选择器和音乐封面启用清晰的圆角玻璃材质</div>
    </div>
    <Switch value={sm.appSettingsState.liquidGlass} onChange={(value) => sm.updateLiquidGlass(value)} />
  </div>
</SettingsCard>

<style>
  .bg-actions { display: flex; align-items: center; gap: 8px; padding: 12px 16px 16px; flex-wrap: wrap; }
  .bg-busy-hint { color: var(--md-on-surface-variant); font-size: 13px; }
  .bg-preview-wrap { flex-shrink: 0; }
  .bg-preview { display: block; width: 72px; height: 48px; object-fit: cover; border-radius: 8px; border: 1px solid var(--md-outline-variant); background: var(--md-surface-container-low); }
</style>
