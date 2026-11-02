<script>
  /**
   * 瀑布流网格 —— 复刻 flutter_staggered_grid_view 的 StaggeredGrid.count 语义：
   * 按顺序放置，每块放入使「覆盖列最大高度」最小的起始列（并列取最左），
   * 支持 per-item 跨列数（HomePage 卡片 span=2，RoomPage span=1）。
   *
   * items: [{ component, props }]（props 可省略）
   * spanOf: (index) => number 跨列数（默认 1，自动 clamp 到列数）
   */
  let { columns = 2, gap = 8, items = [], spanOf = () => 1, heightFn = null } = $props();

  let containerEl = $state(null);
  let tileEls = $state([]);
  let layout = $state([]); // [{ left, top, width, height }]
  let totalHeight = $state(0);

  function tileSpan(i) {
    return Math.max(1, Math.min(columns, spanOf(i) || 1));
  }

  function relayout() {
    if (!containerEl) return;
    const W = containerEl.clientWidth;
    if (W <= 0) return;
    const cellW = (W - gap * (columns - 1)) / columns;
    const colHeights = new Array(columns).fill(0);
    const next = [];
    for (let i = 0; i < items.length; i++) {
      const el = tileEls[i];
      const span = tileSpan(i);
      const width = cellW * span + gap * (span - 1);
      const h = el ? el.offsetHeight : 0;
      let bestCol = 0;
      let bestTop = Infinity;
      for (let c = 0; c <= columns - span; c++) {
        const top = Math.max(...colHeights.slice(c, c + span));
        if (top < bestTop - 0.5) {
          bestTop = top;
          bestCol = c;
        }
      }
      if (bestTop === Infinity) bestTop = 0;
      for (let c = bestCol; c < bestCol + span; c++) {
        colHeights[c] = bestTop + h + gap;
      }
      next.push({
        left: bestCol * (cellW + gap),
        top: bestTop,
        width,
        height: h,
      });
    }
    layout = next;
    totalHeight = items.length
      ? Math.max(...colHeights) - gap
      : 0;
  }

  /* 尺寸/内容变化时重排 */
  $effect(() => {
    void items.length;
    void columns;
    void gap;
    // 等 DOM 渲染后测量
    const raf = requestAnimationFrame(relayout);
    return () => cancelAnimationFrame(raf);
  });

  $effect(() => {
    if (!containerEl) return;
    const ro = new ResizeObserver(() => relayout());
    ro.observe(containerEl);
    // 观察每个 tile 内容高度变化
    const mos = tileEls.filter(Boolean).map((el) => {
      const m = new ResizeObserver(() => relayout());
      m.observe(el);
      return m;
    });
    return () => {
      ro.disconnect();
      mos.forEach((m) => m.disconnect());
    };
  });
</script>

<div class="masonry" bind:this={containerEl} style="height:{totalHeight}px">
  {#each items as item, i (i)}
    <div
      class="tile"
      bind:this={tileEls[i]}
      style="width:{layout[i] ? layout[i].width : '100%'}px; {layout[i] ? `left:${layout[i].left}px; top:${layout[i].top}px;` : ''}"
    >
      <svelte:component this={item.component} {...(item.props || {})} />
    </div>
  {/each}
</div>

<style>
  .masonry {
    position: relative;
    width: 100%;
    transition: height 150ms ease;
  }
  .tile {
    position: absolute;
    top: 0;
    left: 0;
  }
</style>
