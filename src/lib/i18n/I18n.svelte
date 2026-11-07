<script module>
/** i18n（Svelte 5 模块）—— 对齐 easy_localization（zh/en，回退 zh）。 */
/**
 * i18n —— 对齐 easy_localization 行为（zh/en，回退 zh）。
 * 翻译文件直接沿用原版 assets/translations/{zh,en}.json（170 键，扁平结构）。
 */
import { browserLocale } from '../utils/Platform.svelte';

const dictionaries = { zh: null, en: null };
let current = $state('zh');

export async function initI18n() {
  const [zhRes, enRes] = await Promise.allSettled([
    fetch('/translations/zh.json').then((r) => r.json()),
    fetch('/translations/en.json').then((r) => r.json()),
  ]);
  dictionaries.zh = zhRes.status === 'fulfilled' ? zhRes.value : {};
  dictionaries.en = enRes.status === 'fulfilled' ? enRes.value : {};
  // 语言检测：localStorage → 系统语言
  const saved = localStorage.getItem('astral-locale');
  if (saved === 'zh' || saved === 'en') {
    current = saved;
  } else {
    const loc = (browserLocale() || 'zh').toLowerCase();
    current = loc.startsWith('en') ? 'en' : 'zh';
  }
  document.documentElement.lang = current === 'en' ? 'en' : 'zh-CN';
}

export function getLocale() {
  return current;
}

export function setLocale(locale) {
  if (locale !== 'zh' && locale !== 'en') return;
  current = locale;
  localStorage.setItem('astral-locale', locale);
  document.documentElement.lang = locale === 'en' ? 'en' : 'zh-CN';
}

/**
 * 翻译函数，等价于 easy_localization 的 'key'.tr()。
 * 支持 {param} 插值：t('socks5_address_hint', { port: 1080 })
 */
export function t(key, params) {
  const dict = dictionaries[current] || {};
  let value = dict[key];
  if (value == null) {
    // 回退到 zh（与 easy_localization fallbackLocale: zh 一致）
    value = (dictionaries.zh || {})[key];
  }
  if (value == null) return key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      value = value.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
    }
  }
  return value;
}
</script>
