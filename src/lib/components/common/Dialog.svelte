<script>
  /**
   * M3 AlertDialog 等价组件（showDialog）。
   * 用法：
   *   <Dialog bind:open title="标题" onDismiss>
   *     {snippet children()}
   *     {snippet actions()}
   *   </Dialog>
   */
  let { open = $bindable(false), title = '', width = null, onDismiss = () => {}, dismissible = true, children, actions } = $props();

  function onKeydown(e) {
    if (e.key === 'Escape' && dismissible) {
      open = false;
      onDismiss?.();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div
    class="md-dialog-scrim"
    onclick={(e) => {
      if (e.target === e.currentTarget && dismissible) {
        open = false;
        onDismiss?.();
      }
    }}
  >
    <div class="md-dialog" style={width ? `width:${width}px; max-width:90vw` : ''} role="dialog">
      {#if title}
        <div class="dialog-title">{title}</div>
      {/if}
      <div class="dialog-content">
        {@render children?.()}
      </div>
      {#if actions}
        <div class="dialog-actions">
          {@render actions?.()}
        </div>
      {/if}
    </div>
  </div>
{/if}
