<script>
  /**
   * 进度指示器：Linear（定值/不定）与 Circular（M3）。
   */
  let { type = 'linear', value = null, height = 4, size = 48, strokeWidth = 4, style = '' } = $props();

  const R = $derived((size - strokeWidth) / 2);
  const C = $derived(2 * Math.PI * R);
</script>

{#if type === 'linear'}
  <div class="md-progress-linear {value == null ? 'indeterminate' : ''}" style="height:{height}px; {style}">
    <div class="bar" style={value != null ? `width:${Math.max(0, Math.min(1, value)) * 100}%` : ''}></div>
  </div>
{:else}
  <span class="md-progress-circular" style="width:{size}px; height:{size}px; {style}">
    <svg width={size} height={size}>
      <circle cx={size / 2} cy={size / 2} r={R} stroke-width={strokeWidth} class="track"></circle>
      {#if value == null}
        <!-- 不定态：旋转动画走 scoped class（Svelte 会重写 keyframes 名并同步重写
             同组件 CSS 规则里的引用；若用内联 style 引用，名字对不上且 keyframes
             会被摇树删除 —— 转圈失效的根因） -->
        <circle
          cx={size / 2}
          cy={size / 2}
          r={R}
          stroke-width={strokeWidth}
          stroke-dasharray="{C * 0.25} {C * 0.75}"
          class="spinner"
        ></circle>
      {:else}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={R}
          stroke-width={strokeWidth}
          stroke-dasharray="{C * Math.max(0, Math.min(1, value))} {C}"
        ></circle>
      {/if}
    </svg>
  </span>
{/if}

<style>
  /* 不定态圆弧旋转：transform-box:fill-box + transform-origin:center
     围绕圆自身几何中心旋转（任意 size 通用，无需动态 px） */
  .md-progress-circular .spinner {
    animation: circular-spin 1.4s linear infinite;
    transform-box: fill-box;
    transform-origin: center;
  }
  @keyframes circular-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
