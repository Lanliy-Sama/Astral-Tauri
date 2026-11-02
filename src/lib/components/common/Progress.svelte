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
        <circle
          cx={size / 2}
          cy={size / 2}
          r={R}
          stroke-width={strokeWidth}
          stroke-dasharray="{C * 0.25} {C * 0.75}"
          style="animation: circular-spin 1.4s linear infinite; transform-origin: {size / 2}px {size / 2}px"
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
  @keyframes circular-spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
