<script>
  /**
   * 主题色选择器 —— 对应 showThemeColorPicker：
   * BlockPicker（19 种 Material 预设色）+ 自定义颜色（HSV 调色板）。
   */
  import Icon from './Icon.svelte';
  import Dialog from './Dialog.svelte';
  import { sm } from '../../../lib/services/service-manager.svelte.js';

  let { open = $bindable(false) } = $props();

  const PRESET_COLORS = [
    { name: 'red', hex: '#F44336' },
    { name: 'pink', hex: '#E91E63' },
    { name: 'purple', hex: '#9C27B0' },
    { name: 'deepPurple', hex: '#673AB7' },
    { name: 'indigo', hex: '#3F51B5' },
    { name: 'blue', hex: '#2196F3' },
    { name: 'lightBlue', hex: '#03A9F4' },
    { name: 'cyan', hex: '#00BCD4' },
    { name: 'teal', hex: '#009688' },
    { name: 'green', hex: '#4CAF50' },
    { name: 'lightGreen', hex: '#8BC34A' },
    { name: 'lime', hex: '#CDDC39' },
    { name: 'yellow', hex: '#FFEB3B' },
    { name: 'amber', hex: '#FFC107' },
    { name: 'orange', hex: '#FF9800' },
    { name: 'deepOrange', hex: '#FF5722' },
    { name: 'brown', hex: '#795548' },
    { name: 'grey', hex: '#9E9E9E' },
    { name: 'blueGrey', hex: '#607D8B' },
  ];

  let originalColor = $state('');
  let showAdvanced = $state(false);
  let hue = $state(0);
  let sat = $state(1);
  let val = $state(1);

  $effect(() => {
    if (open) {
      originalColor = sm.themeState.themeColor;
      showAdvanced = false;
      const hsv = hexToHsv(sm.themeState.themeColor);
      hue = hsv.h;
      sat = hsv.s;
      val = hsv.v;
    }
  });

  /* ---- HSV / HEX 工具 ---- */
  function hexToHsv(hex) {
    let c = hex.replace('#', '');
    if (c.length === 8) c = c.slice(2);
    const r = parseInt(c.substring(0, 2), 16) / 255;
    const g = parseInt(c.substring(2, 4), 16) / 255;
    const b = parseInt(c.substring(4, 6), 16) / 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const d = max - min;
    let h = 0;
    if (d !== 0) {
      if (max === r) h = ((g - b) / d) % 6;
      else if (max === g) h = (b - r) / d + 2;
      else h = (r - g) / d + 4;
      h *= 60;
      if (h < 0) h += 360;
    }
    return { h, s: max === 0 ? 0 : d / max, v: max };
  }

  function hsvToHex(h, s, v) {
    const c = v * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = v - c;
    let r = 0;
    let g = 0;
    let b = 0;
    if (h < 60) [r, g, b] = [c, x, 0];
    else if (h < 120) [r, g, b] = [x, c, 0];
    else if (h < 180) [r, g, b] = [0, c, x];
    else if (h < 240) [r, g, b] = [0, x, c];
    else if (h < 300) [r, g, b] = [x, 0, c];
    else [r, g, b] = [c, 0, x];
    const toHex = (n) =>
      Math.round((n + m) * 255)
        .toString(16)
        .padStart(2, '0');
    return `#${toHex(r)}${toHex(g)}${toHex(b)}`.toUpperCase();
  }

  const currentCustomHex = $derived(hsvToHex(hue, sat, val));

  async function preview(hex) {
    await sm.updateThemeColor(hex);
  }

  async function cancel() {
    await sm.updateThemeColor(originalColor);
    open = false;
  }

  function confirm() {
    open = false;
  }

  function openAdvanced() {
    open = false;
    showAdvanced = true;
  }

  function onHueInput(e) {
    hue = Number(e.target.value);
    preview(currentCustomHex);
  }
  function onSatVal(e, which) {
    if (which === 's') sat = Number(e.target.value);
    else val = Number(e.target.value);
    preview(currentCustomHex);
  }
</script>

<Dialog bind:open title="选择主题颜色">
  <div class="block-picker">
    {#each PRESET_COLORS as color (color.name)}
      <button
        class="swatch"
        class:selected={sm.themeState.themeColor.toUpperCase() === color.hex.toUpperCase()}
        style="background:{color.hex}"
        onclick={() => preview(color.hex)}
        data-tip={color.name}
      >
        {#if sm.themeState.themeColor.toUpperCase() === color.hex.toUpperCase()}
          <Icon name="check" size={20} color="#fff" />
        {/if}
      </button>
    {/each}
  </div>
  <div style="height:10px"></div>
  <button class="md-btn md-elevated-btn" style="width:100%" onclick={openAdvanced}>
    <Icon name="colorize" size={18} />
    自定义颜色
  </button>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={cancel}>取消</button>
    <button class="md-btn md-filled-btn" onclick={confirm}>确定</button>
  {/snippet}
</Dialog>

<Dialog bind:open={showAdvanced} title="自定义颜色">
  <div class="custom-picker">
    <!-- SV 面积（pickerArea 高度占比 0.8） -->
    <div
      class="sv-area"
      style="background: linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl({hue} 100% 50%))"
    >
      <div class="sv-thumb" style="left:{sat * 100}%; top:{(1 - val) * 100}%"></div>
      <input type="range" min="0" max="1" step="0.001" value={sat} class="sv-range-x" oninput={(e) => onSatVal(e, 's')} />
      <input type="range" min="0" max="1" step="0.001" value={val} class="sv-range-y" oninput={(e) => onSatVal(e, 'v')} />
    </div>
    <!-- 色相滑条 -->
    <input type="range" min="0" max="360" step="1" value={hue} class="hue-range" oninput={onHueInput} />
    <div class="picker-labels">
      <span class="md-label-medium">RGB {parseInt(currentCustomHex.slice(1, 3), 16)}, {parseInt(currentCustomHex.slice(3, 5), 16)}, {parseInt(currentCustomHex.slice(5, 7), 16)}</span>
      <span class="md-label-medium">{currentCustomHex}</span>
    </div>
    <div class="current-swatch" style="background:{currentCustomHex}"></div>
  </div>
  {#snippet actions()}
    <button class="md-btn md-text-btn" onclick={async () => { await sm.updateThemeColor(originalColor); showAdvanced = false; }}>取消</button>
    <button class="md-btn md-filled-btn" onclick={() => (showAdvanced = false)}>确定</button>
  {/snippet}
</Dialog>

<style>
  .block-picker {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
    max-width: 320px;
  }
  .swatch {
    width: 100%;
    aspect-ratio: 1;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.1);
  }
  .swatch.selected {
    box-shadow:
      inset 0 0 0 2px var(--md-on-surface),
      0 0 0 2px var(--md-surface);
  }
  .custom-picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 300px;
  }
  .sv-area {
    position: relative;
    height: 200px;
    border-radius: 10px;
    overflow: hidden;
  }
  .sv-thumb {
    position: absolute;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 3px solid #fff;
    box-shadow: 0 0 4px rgba(0, 0, 0, 0.4);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }
  .sv-range-x,
  .sv-range-y {
    position: absolute;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: crosshair;
    margin: 0;
  }
  .sv-range-y {
    writing-mode: vertical-lr;
    direction: rtl;
  }
  .hue-range {
    width: 100%;
    height: 14px;
    appearance: none;
    border-radius: 7px;
    background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
    cursor: pointer;
  }
  .hue-range::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: transparent;
    border: 3px solid #fff;
    box-shadow: 0 0 4px rgba(0, 0, 0, 0.4);
  }
  .picker-labels {
    display: flex;
    justify-content: space-between;
  }
  .current-swatch {
    height: 36px;
    border-radius: 10px;
    border: 1px solid var(--md-outline-variant);
  }
</style>
