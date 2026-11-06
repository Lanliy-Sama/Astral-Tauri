<script>
  /**
   * M3 Switch（对应 Flutter Switch）+ 液态玻璃增强：
   * - 液态玻璃模式（html.liquid-glass）下轨道与 thumb 均为双层玻璃材质（liquid-glass.css）
   * - 按住 thumb 可拖动：thumb 位置/尺寸实时跟随（液态形变），跨过中点即时预览开关态，
   *   松手吸附最近一侧并弹性回落；未拖动的点按保持原生 toggle 行为
   */
  let { value = $bindable(false), disabled = false, scale = false, onChange = null } = $props();

  let trackEl = null; // 轨道
  let thumbEl = null; // 滑块
  let drag = null; // { p } 拖动进度 0..1
  let dragOn = $state(null); // 拖动中的实时预览态（null = 未拖动）
  let suppressClick = false; // 拖动释放后吞掉一次 click

  /* thumb 中心在轨道上的几何（border-box 比例，scale-08 缩放下同样成立） */
  const OFF_CENTER = 16 / 52; // OFF thumb 中心 = 6 + 8 + 2(边框)
  const ON_CENTER = 36 / 52; // ON  thumb 中心 = 22 + 12 + 2(边框)
  const SPAN = ON_CENTER - OFF_CENTER;

  function toggle() {
    if (disabled) return;
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    value = !value;
    onChange?.(value);
  }

  function onPointerDown(e) {
    if (disabled || e.button !== 0) return;
    suppressClick = false;
    drag = { p: null };
    trackEl.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e) {
    if (!drag) return;
    const rect = trackEl.getBoundingClientRect();
    if (rect.width <= 0) return;
    const p = Math.min(1, Math.max(0, ((e.clientX - rect.left) / rect.width - OFF_CENTER) / SPAN));
    drag.p = p;
    if (dragOn === null) {
      // 离开当前 thumb 位置 4% 轨道宽才视为拖动（避免点按误判）
      const stateP = value ? 1 : 0;
      if (Math.abs(p - stateP) < 0.04) return;
      dragOn = value; // dragging 类由 class:dragging 响应式挂载（不会被 Svelte 类重写抹掉）
    }
    dragOn = p > 0.5; // 跨过中点即时预览开关态
    // 液态形变：尺寸随位置插值（16→24），内联驱动（.dragging 已禁用过渡）
    const w = 16 + 8 * p;
    const left = (OFF_CENTER * rect.width) + SPAN * rect.width * p - w / 2;
    thumbEl.style.width = w + 'px';
    thumbEl.style.height = w + 'px';
    thumbEl.style.left = (left / rect.width) * 100 + '%';
    // 镜面高光跟随（capture 下 MainScreen 的委派同样命中，这里补写指针坐标）
    trackEl.style.setProperty('--lg-x', e.clientX - rect.left + 'px');
    trackEl.style.setProperty('--lg-y', e.clientY - rect.top + 'px');
  }

  function endDrag(commit) {
    if (!drag) return;
    const wasDragging = dragOn !== null;
    const p = drag.p;
    drag = null;
    if (!wasDragging) return; // 未拖动 → 走原生 click
    const finalOn = p != null ? p > 0.5 : value;
    if (commit) suppressClick = true;
    requestAnimationFrame(() => {
      thumbEl.style.width = '';
      thumbEl.style.height = '';
      thumbEl.style.left = ''; // 回落 CSS 类定位 → 弹性吸附动画
    });
    if (commit && finalOn !== value) {
      value = finalOn;
      onChange?.(finalOn);
    }
    dragOn = null; // class:dragging 响应式摘除
  }

  function onPointerUp() {
    endDrag(true);
  }
  function onPointerCancel() {
    endDrag(false);
  }
</script>

<button
  class="md-switch lg-glass lg-spec {(dragOn ?? value) ? 'on' : ''} {scale ? 'scale-08' : ''}"
  class:disabled
  class:dragging={dragOn !== null}
  aria-checked={dragOn ?? value}
  role="switch"
  onclick={toggle}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerCancel}
  tabindex={disabled ? -1 : 0}
  bind:this={trackEl}
  {disabled}
>
  <span class="thumb lg-glass" bind:this={thumbEl}></span>
</button>

<style>
  /* 拖动中 thumb 位置由 JS 内联驱动：禁用位置过渡（液态玻璃/普通模式一致） */
  .md-switch.dragging .thumb {
    transition: none;
  }
</style>
