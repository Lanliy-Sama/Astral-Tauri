<script>
  /** 左侧导航（桌面端，宽 > 480）—— 80px 宽，选中块动画 200ms（Curves.easeInOutCubic）
   *  对齐策略（2026-11-01 v2，结构性保证）：
   *  - 选中块直接渲染在“被选中的按钮”内部、inset:0 铺满该按钮 —— 静止态与按钮
   *    由 DOM 结构保证完全重合，不做任何坐标换算，不受 DPI/缩放/字体/窗口尺寸影响；
   *  - 切换时的滑动动画采用 FLIP：起点 = 新旧按钮实测位置差（同一父容器，精确），
   *    用 Web Animations API 播放，无论动画是否被中断，最终都收敛于 translateY(0)；
   *  - 悬停块改为按钮自身的 CSS 伪元素，同样结构性对齐。
   */
  import Icon from '../common/Icon.svelte';
  import { sm } from '../../services/service-manager.svelte.js';

  let { items = [] } = $props();

  let itemEls = []; // 各按钮 DOM 引用
  let indicatorEl = null; // 当前选中块（渲染在选中按钮内部）
  let prevIndex = sm.uiState.selectedIndex ?? 0;

  $effect(() => {
    const idx = sm.uiState.selectedIndex;
    if (idx === prevIndex || idx == null) return;
    const fromEl = itemEls[prevIndex];
    const toEl = itemEls[idx];
    prevIndex = idx;
    if (!fromEl || !toEl || !indicatorEl) return; // 兜底：直接就位，不播动画
    // 同一父容器内两按钮的纵向差值，仅作为动画起点
    const dy = fromEl.getBoundingClientRect().top - toEl.getBoundingClientRect().top;
    if (Math.abs(dy) < 0.5) return;
    indicatorEl.getAnimations().forEach((a) => a.cancel());
    // Curves.easeInOutCubic = Cubic(0.655, 0.125, 0.425, 0.855)
    indicatorEl.animate(
      [{ transform: `translateY(${dy}px)` }, { transform: 'translateY(0px)' }],
      { duration: 200, easing: 'cubic-bezier(0.655, 0.125, 0.425, 0.855)' }
    );
  });

  function onTap(index) {
    if (sm.uiState.selectedIndex !== index) {
      sm.uiState.selectedIndex = index;
    }
  }
</script>

<div class="left-nav">
  <div class="nav-items">
    {#each items as item, index (index)}
      <div
        class="nav-item {sm.uiState.selectedIndex === index ? 'selected' : ''} {sm.uiState.hoveredIndex === index && sm.uiState.hoveredIndex !== sm.uiState.selectedIndex ? 'hovered' : ''}"
        bind:this={itemEls[index]}
        onmouseenter={() => (sm.uiState.hoveredIndex = index)}
        onmouseleave={() => (sm.uiState.hoveredIndex = null)}
        onclick={() => onTap(index)}
      >
        {#if sm.uiState.selectedIndex === index}
          <div class="selection-indicator" bind:this={indicatorEl}></div>
        {/if}
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
  }
  .nav-item {
    position: relative; /* 选中块/悬停块的定位父级 = 按钮自身 */
    height: 64px;
    margin: 4px 8px;
    flex-shrink: 0; /* 固定 64px 高（对应 Flutter 固定尺寸 Container），窗口过矮时不压缩 */
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 100ms ease;
  }
  /* InkWell 悬停高亮（M3 默认 hoverColor = onSurface 8%）——对应原版桌面端悬停效果 */
  .nav-item:hover {
    background: var(--md-on-surface-08);
  }
  /* 悬停指示块：surfaceContainerHighest @50%，仅悬停在未选中项上时出现（伪元素结构性对齐） */
  .nav-item.hovered::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 12px;
    background: var(--md-surface-container-highest-05);
  }
  /* 选中块：铺满所在按钮（inset:0），静止态与按钮结构性重合 */
  .selection-indicator {
    position: absolute;
    inset: 0;
    border-radius: 12px;
    background: var(--md-primary-container);
  }
  /* 图标与文字置于指示块之上 */
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
