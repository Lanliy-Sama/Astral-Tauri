<script>
  /** 竖屏/窄窗横向液体选择器：点击、横向拖动、速度形变与松手弹簧。 */
  import Icon from '../common/Icon.svelte';
  import { sm } from '../../services/ServiceManager.svelte';

  let { items = [], isSmallWindow = false } = $props();
  const iconSize = $derived(isSmallWindow ? 20 : 24);
  const selectedFontSize = $derived(isSmallWindow ? 10 : 12);
  const unselectedFontSize = $derived(isSmallWindow ? 8 : 10);

  let navEl = null;
  let itemEls = [];
  let pillX = $state(0);
  let pillW = $state(0);
  let pillScaleX = $state(1);
  let pillScaleY = $state(1);
  let dragging = $state(false);
  let hoveredIndex = $state(null);
  let drag = null;
  let currentX = null;
  let springRaf = 0;
  let springTarget = null;
  let suppressClick = false;

  function geometry(index) {
    const el = itemEls[index];
    if (!el) return { x: 0, w: 0, c: 0 };
    return {
      x: el.offsetLeft + 6,
      w: Math.max(0, el.offsetWidth - 12),
      c: el.offsetLeft + el.offsetWidth / 2,
    };
  }

  function nearestIndex(center) {
    let best = 0;
    let distance = Infinity;
    for (let i = 0; i < items.length; i++) {
      const next = Math.abs(geometry(i).c - center);
      if (next < distance) { distance = next; best = i; }
    }
    return best;
  }

  function springTo(target, initialVelocity = 0) {
    cancelAnimationFrame(springRaf);
    springTarget = target;
    let x = currentX ?? target;
    let velocity = initialVelocity;
    let previous = performance.now();
    const step = (time) => {
      const dt = Math.min((time - previous) / 1000, 0.032);
      previous = time;
      const acceleration = -390 * (x - target) - 32 * velocity;
      velocity += acceleration * dt;
      x += velocity * dt;
      currentX = x;
      pillX = x;
      const stretch = Math.min(Math.abs(velocity) / 1600, 0.26);
      pillScaleX = 1 + stretch;
      pillScaleY = 1 / (1 + stretch * 0.42);
      if (Math.abs(x - target) < 0.12 && Math.abs(velocity) < 2) {
        currentX = target;
        pillX = target;
        pillScaleX = 1;
        pillScaleY = 1;
        springTarget = null;
        return;
      }
      springRaf = requestAnimationFrame(step);
    };
    springRaf = requestAnimationFrame(step);
  }

  $effect(() => {
    const index = sm.uiState.selectedIndex;
    const g = geometry(index);
    pillW = g.w;
    if (currentX == null) {
      currentX = g.x;
      pillX = g.x;
    } else if (!dragging && springTarget !== g.x && currentX !== g.x) {
      springTo(g.x);
    }
  });

  $effect(() => {
    const resize = () => {
      cancelAnimationFrame(springRaf);
      springTarget = null;
      const g = geometry(sm.uiState.selectedIndex);
      currentX = g.x;
      pillX = g.x;
      pillW = g.w;
      pillScaleX = 1;
      pillScaleY = 1;
    };
    window.addEventListener('resize', resize);
    return () => window.removeEventListener('resize', resize);
  });

  function tap(index) {
    if (suppressClick) { suppressClick = false; return; }
    sm.uiState.selectedIndex = index;
  }

  function pointerDown(event, index) {
    if (event.button !== 0 || !sm.appSettingsState.liquidGlass) return;
    cancelAnimationFrame(springRaf);
    springTarget = null;
    drag = {
      pointerId: event.pointerId, index, startX: event.clientX, startY: event.clientY,
      lastX: event.clientX, lastT: performance.now(), velocity: 0, moved: false,
    };
    pillScaleX = 0.96;
    pillScaleY = 0.96;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function pointerMove(event) {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const totalX = event.clientX - drag.startX;
    const totalY = event.clientY - drag.startY;
    if (!drag.moved) {
      if (Math.abs(totalX) < 5 && Math.abs(totalY) < 5) return;
      // 竖向手势交还页面；横向手势由液体选择器捕获。
      if (Math.abs(totalY) > Math.abs(totalX)) {
        drag = null;
        pillScaleX = 1;
        pillScaleY = 1;
        return;
      }
      drag.moved = true;
      dragging = true;
      currentX = pillX;
      navEl?.classList.add('lg-dragging');
    }
    event.preventDefault();
    const now = performance.now();
    const dt = Math.max(1, now - drag.lastT) / 1000;
    const dx = event.clientX - drag.lastX;
    drag.velocity = drag.velocity * 0.7 + (dx / dt) * 0.3;
    drag.lastX = event.clientX;
    drag.lastT = now;
    const rect = navEl.getBoundingClientRect();
    const first = geometry(0);
    const last = geometry(items.length - 1);
    const center = Math.min(last.c, Math.max(first.c, event.clientX - rect.left));
    currentX = center - pillW / 2;
    pillX = currentX;
    hoveredIndex = nearestIndex(center);
    const stretch = Math.min(Math.abs(drag.velocity) / 1500, 0.3);
    pillScaleX = 1 + stretch;
    pillScaleY = 1 / (1 + stretch * 0.45);
  }

  function pointerUp(event) {
    if (!drag || (event.pointerId != null && event.pointerId !== drag.pointerId)) return;
    const session = drag;
    drag = null;
    navEl?.classList.remove('lg-dragging');
    hoveredIndex = null;
    if (!session.moved) {
      pillScaleX = 1;
      pillScaleY = 1;
      return;
    }
    dragging = false;
    suppressClick = true;
    const index = nearestIndex((currentX ?? pillX) + pillW / 2);
    const target = geometry(index).x;
    springTo(target, session.velocity * 0.5);
    if (sm.uiState.selectedIndex !== index) sm.uiState.selectedIndex = index;
  }
</script>

<div class="bottom-nav" bind:this={navEl}>
  <div
    class="selection-pill lg-glass lg-spec"
    class:dragging
    style="width:{pillW}px; transform: translateX({pillX}px) scale({pillScaleX}, {pillScaleY})"
  ></div>
  {#each items as item, index (index)}
    <button
      class="bottom-nav-item {sm.uiState.selectedIndex === index ? 'selected' : ''} {hoveredIndex === index ? 'hovered' : ''}"
      bind:this={itemEls[index]}
      onclick={() => tap(index)}
      onpointerdown={(e) => pointerDown(e, index)}
      onpointermove={pointerMove}
      onpointerup={pointerUp}
      onpointercancel={pointerUp}
    >
      <Icon
        name={sm.uiState.selectedIndex === index ? item.activeIcon : item.icon}
        size={iconSize}
        color={sm.uiState.selectedIndex === index ? 'var(--md-primary)' : 'var(--md-on-surface-variant)'}
      />
      {#if sm.uiState.selectedIndex === index || !isSmallWindow}
        <span
          class="bn-label {sm.uiState.selectedIndex === index ? 'sel' : 'unsel'}"
          style="font-size:{sm.uiState.selectedIndex === index ? selectedFontSize : unselectedFontSize}px"
        >{item.label}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .bottom-nav {
    background: var(--md-surface-container-low);
    display: flex;
    border-top: 1px solid var(--md-outline-variant);
    flex-shrink: 0;
    position: relative;
    touch-action: pan-y;
  }
  .bottom-nav-item {
    flex: 1;
    min-width: 0;
    min-height: 56px;
    border: none;
    background: transparent;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 6px 0;
    cursor: pointer;
    position: relative;
    z-index: 1;
    touch-action: pan-y;
    -webkit-tap-highlight-color: transparent;
  }
  .bottom-nav-item.hovered { color: var(--md-primary); }
  .selection-pill {
    display: none;
    position: absolute;
    top: 6px;
    height: 44px;
    left: 0;
    border-radius: 16px;
    pointer-events: none;
    transform-origin: center;
    will-change: transform;
  }
  :global(html.liquid-glass) .selection-pill { display: block; }
  :global(html.liquid-glass) .selection-pill.dragging { cursor: grabbing; }
  .bn-label { line-height: 16px; transition: color 100ms ease; }
  .bn-label.sel { color: var(--md-primary); font-weight: 500; }
  .bn-label.unsel { color: var(--md-on-surface-variant); }
  @media (prefers-reduced-motion: reduce) {
    .bn-label { transition: none; }
  }
</style>
