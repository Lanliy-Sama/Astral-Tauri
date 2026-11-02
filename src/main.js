import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { initI18n } from './lib/i18n/index.svelte.js';
import { initPlatform } from './lib/utils/platform.js';
import { initThemeEngine, applyDebugThemeOverride } from './lib/services/theme-engine.svelte.js';

async function main() {
  await initPlatform();
  await initI18n();
  applyDebugThemeOverride();
  initThemeEngine();
  mount(App, { target: document.getElementById('app') });
}

main();
