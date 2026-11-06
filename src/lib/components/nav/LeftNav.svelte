<script>
  /** 左侧导航（桌面端，宽 > 480）—— 80px 宽，选中块为容器级玻璃透镜：
   *  - 选中块渲染在 .nav-items 坐标系内（offsetTop 定位），静止态与按钮结构性重合；
   *  - 点击切换：弹簧滑动（欠阻尼，轻微过冲回落）；
   *  - 按住拖动（液态玻璃「拖一拖滑块」）：选中框脱离吸附、实时跟随指针，
   *    纵向速度映射为液态拉伸（scaleY），指针下的选项即时高亮；
   *    松手后带初速度弹回最近选项 —— 在选项之间拖动选择。
   */
  import Icon from '../common/Icon.svelte';
  import { sm } from '../../services/service-manager.svelte.js';

  let { items = [] } = $props();

  let navItemsEl = null; // .nav-items 容器（选中块的定位父级）
  let itemEls = []; // 各按钮 DOM 引用
  let dragging = $state(false); // 按住拖动中
  let indicatorY = $state(0); // 选中块 translateY（容器坐标）
  let indicatorScaleY = $state(1); // 液态拉伸
  let specX = $state(null); // 镜面高光 X（拖拽跟随；写进模板 style 避免 Svelte 重写抹掉）
  let specY = $state(null); // 镜面高光 Y
  let dragInfo = null; // { v, lastY, lastT }
  let suppressClick = false; // 拖动释放后吞掉一次原生 click（避免选回起始项）
  let springRaf = 0;
  let springTarget = null; // 当前弹簧目标（去重，拖拽提交时避免二次重启）
  let currentY = null; // 选中块当前 y（不进响应式，rAF 内直写样式）

  const ITEM_H = 64; // 与样式一致

  function itemTopAt(index) {
    const el = itemEls[index];
    return el ? el.offsetTop : 0;
  }

  /* 弹簧：欠阻尼积分器（点击切换 / 松手回弹共用），v0 供松手时接续拖拽速度 */
  function springTo(target, v0 = 0) {
    cancelAnimationFrame(springRaf);
    springTarget = target;
    let y = currentY;
    let v = v0;
    const stiffness = 460;
    const damping = 30;
    let lastT = performance.now();
    const step = (t) => {
      const dt = Math.min((t - lastT) / 1000, 0.032);
      lastT = t;
      const a = -stiffness * (y - target) - damping * v;
      v += a * dt;
      y += v * dt;
      currentY = y;
      // 液态拉伸：运动速度 → 纵向伸长（cap 30%）
      const stretch = Math.min(Math.abs(v) / 1500, 0.3);
      indicatorY = y;
      indicatorScaleY = 1 + stretch;
      if (Math.abs(y - target) < 0.12 && Math.abs(v) < 2) {
        currentY = target;
        indicatorY = target;
        indicatorScaleY = 1;
        springTarget = null;
        return;
      }
      springRaf = requestAnimationFrame(step);
    };
    springRaf = requestAnimationFrame(step);
  }

  /* 选中项变化 → 弹簧滑到新位置（拖拽提交时 springTarget 已对齐则跳过） */
  $effect(() => {
    const idx = sm.uiState.selectedIndex;
    const top = itemTopAt(idx);
    if (currentY == null) {
      // 首次定位：直接就位，不播动画
      currentY = top;
      indicatorY = top;
      return;
    }
    if (springTarget === top || dragging) return;
    if (currentY !== top) springTo(top);
    else {
      currentY = top;
      indicatorY = top;
    }
  });

  /* 窗口尺寸变化：条目高度固定，重定位即可 */
  $effect(() => {
    const onResize = () => {
      const top = itemTopAt(sm.uiState.selectedIndex);
      currentY = top;
      indicatorY = top;
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  });

  function onTap(index) {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    if (sm.uiState.selectedIndex !== index) {
      sm.uiState.selectedIndex = index;
    }
  }

  /* ---- 按住拖动 ---- */
  function onItemPointerDown(e, index) {
    if (e.button !== 0) return;
    suppressClick = false;
    dragInfo = { v: 0, lastY: e.clientY, lastT: performance.now(), moved: false, index };
    e.currentTarget.setPointerCapture(e.pointerId);
  }

  function onItemPointerMove(e) {
    if (!dragInfo) return;
    const now = performance.now();
    const dt = Math.max(now - dragInfo.lastT, 1) / 1000;
    const dy = e.clientY - dragInfo.lastY;
    if (!dragInfo.moved && Math.abs(e.clientY - dragInfo.lastY) > 3 && !dragging) {
      dragInfo.moved = true;
      dragging = true;
      cancelAnimationFrame(springRaf);
      springTarget = null;
      navItemsEl.classList.add('lg-dragging');
      currentY = itemTopAt(dragInfo.index);
    }
    if (!dragging) return;
    // 速度跟踪（指数平滑）
    dragInfo.v = dragInfo.v * 0.72 + (dy / dt) * 0.28;
    dragInfo.lastY = e.clientY;
    dragInfo.lastT = now;
    // 跟随：选中块中心对齐指针，钳制在条目范围内
    const rect = navItemsEl.getBoundingClientRect();
    const firstTop = itemTopAt(0);
    const lastTop = itemTopAt(items.length - 1);
    const y = Math.min(lastTop, Math.max(firstTop, e.clientY - rect.top - ITEM_H / 2));
    currentY = y;
    indicatorY = y;
    // 液态拉伸：拖动速度 → scaleY
    const stretch = Math.min(Math.abs(dragInfo.v) / 1500, 0.3);
    indicatorScaleY = 1 + stretch;
    // 指针下选项即时高亮 + 镜面高光跟随（capture 下 MainScreen 委派不命中，这里直写）
    const hovered = nearestIndex(y);
    if (sm.uiState.hoveredIndex !== hovered) sm.uiState.hoveredIndex = hovered;
    specX = e.clientX - rect.left;
    specY = e.clientY - rect.top;
  }

  function nearestIndex(y) {
    let best = 0;
    let bestD = Infinity;
    for (let i = 0; i < items.length; i++) {
      const d = Math.abs(itemTopAt(i) - y);
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    }
    return best;
  }

  function onItemPointerUp() {
    if (!dragInfo) return;
    const wasDragging = dragging;
    const v = dragInfo.v;
    dragInfo = null;
    if (!wasDragging) return; // 未拖动 → 走原生 click
    dragging = false;
    suppressClick = true; // 拖动后的 click 不再触发 onTap
    navItemsEl.classList.remove('lg-dragging');
    if (sm.uiState.hoveredIndex != null) sm.uiState.hoveredIndex = null;
    specX = null; // 高光回落默认位置
    specY = null;
    const targetIdx = nearestIndex(currentY);
    const target = itemTopAt(targetIdx);
    springTo(target, v * 0.55); // 带初速度弹回
    if (sm.uiState.selectedIndex !== targetIdx) {
      sm.uiState.selectedIndex = targetIdx; // effect 内 springTarget 已对齐，不会重启
    }
  }
</script>

<div class="left-nav">
  <div class="nav-items" bind:this={navItemsEl}>
    <!-- 选中块：容器级玻璃透镜（液态玻璃下由 liquid-glass.css 赋材质） -->
    <div
      class="selection-indicator lg-glass lg-spec"
      class:dragging
      style="transform: translateY({indicatorY}px) scaleY({indicatorScaleY}){specX != null ? `; --lg-x: ${specX}px; --lg-y: ${specY}px` : ''}"
    ></div>
    {#each items as item, index (index)}
      <div
        class="nav-item {sm.uiState.selectedIndex === index ? 'selected' : ''} {sm.uiState.hoveredIndex === index && sm.uiState.hoveredIndex !== sm.uiState.selectedIndex ? 'hovered' : ''}"
        bind:this={itemEls[index]}
        onmouseenter={() => (sm.uiState.hoveredIndex = index)}
        onmouseleave={() => (sm.uiState.hoveredIndex = null)}
        onclick={() => onTap(index)}
        onpointerdown={(e) => onItemPointerDown(e, index)}
        onpointermove={onItemPointerMove}
        onpointerup={onItemPointerUp}
        onpointercancel={onItemPointerUp}
      >
        <div class="nav-content">
          <Icon
            name={sm.uiState.selectedIndex === index ? item.activeIcon : item.icon}
            size={24}
            color={sm.uiState.selectedIndex === index ? 'var(--md-primary)' : 'var(--md-on-surface-variant)'}
          />
          <span class="nav-label">{item.label}</span>
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .left-nav {
    width: 80px;
    background: var(--md-surface);
    border-right: 1px solid var(--md-outline);
    padding-top: 14px;
    flex-shrink: 0;
    overflow: hidden;
  }
  .nav-items {
    display: flex;
    flex-direction: column;
    position: relative; /* 选中块的定位父级（两种模式一致） */
  }
  .nav-item {
    position: relative;
    height: 64px;
    margin: 4px 8px;
    flex-shrink: 0;
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 100ms ease;
    touch-action: none; /* 允许按住纵向拖动（触屏） */
  }
  .nav-item:hover {
    background: var(--md-on-surface-08);
  }
  /* 悬停指示块：surfaceContainerHighest @50%（未选中项悬停时） */
  .nav-item.hovered::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 12px;
    background: var(--md-surface-container-highest-05);
  }
  /* 选中块：铺满目标按钮（与按钮 margin 对齐），静止态结构性重合 */
  .selection-indicator {
    position: absolute;
    left: 8px;
    right: 8px;
    height: 64px;
    top: 0;
    border-radius: 12px;
    background: var(--md-primary-container);
    pointer-events: none;
    will-change: transform;
    z-index: 0;
  }
  .nav-content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .nav-label {
    font-size: 14px;
    line-height: 20px;
    font-weight: 700;
    color: var(--md-on-surface-variant);
    transition: color 100ms ease;
    text-align: center;
  }
  .nav-item.selected .nav-label {
    color: var(--md-primary);
  }
</style>
