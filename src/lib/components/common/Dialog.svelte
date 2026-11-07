<script>
  /**
   * M3 AlertDialog 等价组件（showDialog）。
   * 2.4 动画：入场 scrim 淡入 + 面板 emphasized 缩放；退出同样收场（滞后卸载）。
   * 用法：
   *   <Dialog bind:open title="标题" onDismiss>
   *     {snippet children()}
   *     {snippet actions()}
   *   </Dialog>
   */
  let { open = $bindable(false), title = '', width = null, onDismiss = () => {}, dismissible = true, children, actions } = $props();

  /* 滞后卸载：open→false 后先播 200ms 退出动画再移除 DOM（重开时中断退出） */
  let visible = $state(false);
  let cycle = $state(0); // 真开一次自增：重播入场动画
  const closing = $derived(!open && visible);

  $effect(() => {
    if (open) {
      if (!visible) cycle += 1; // 从关到开才重播入场（退出中重开只收 .closing）
      visible = true;
      return;
    }
    if (visible) {
      const t = setTimeout(() => {
        visible = false;
      }, 200);
      return () => clearTimeout(t); // 期间重开 → 取消卸载
    }
  });

  function onKeydown(e) {
    if (e.key === 'Escape' && dismissible && open) {
      open = false;
      onDismiss?.();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if visible}
  {#key cycle}
    <div
      class="md-dialog-scrim"
      class:closing
      onclick={(e) => {
        if (e.target === e.currentTarget && dismissible && open) {
          open = false;
          onDismiss?.();
        }
      }}
    >
      <div class="md-dialog" class:closing style={width ? `width:${width}px; max-width:90vw` : ''} role="dialog">
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
  {/key}
{/if}
