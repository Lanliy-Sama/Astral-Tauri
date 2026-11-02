<script>
  /**
   * BottomSheetMenu —— 对应 magic_wall_page._showCreationMenu 的
   * showModalBottomSheet(SafeArea + Wrap[ListTile, ListTile])。
   * M3 模态底部弹层：贴窗口底边、顶部圆角 28、底部直角、最大宽 640 居中。
   * items: [{ icon, label, disabled, onSelect }]
   */
  import Icon from '$lib/components/common/Icon.svelte';

  let {
    open = $bindable(false),
    items = [],
    onDismiss = () => {},
  } = $props();

  function close() {
    open = false;
    onDismiss?.();
  }

  function select(item) {
    if (item.disabled) return;
    open = false; // Navigator.pop(context) 后再执行动作
    item.onSelect?.();
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape' && open) close();
  }}
/>

{#if open}
  <div
    class="mw-sheet-scrim"
    onclick={(e) => {
      if (e.target === e.currentTarget) close();
    }}
  >
    <div class="mw-sheet" role="dialog" aria-modal="true">
      {#each items as item}
        <div
          class="md-list-tile {item.disabled ? 'disabled' : ''}"
          role="button"
          tabindex="0"
          onclick={() => select(item)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') select(item);
          }}
        >
          <span class="tile-leading"><Icon name={item.icon} size={24} /></span>
          <div class="tile-content">
            <div class="tile-title">{item.label}</div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .mw-sheet-scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.32);
    z-index: 1000;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
  .mw-sheet {
    width: 100%;
    max-width: 640px; /* M3 modal bottom sheet 最大宽 */
    background: var(--md-surface-container-low);
    border-radius: 28px 28px 0 0; /* 顶部圆角 28，底部贴边直角 */
    box-shadow:
      0 3px 3px rgba(0, 0, 0, 0.3),
      0 12px 24px 6px rgba(0, 0, 0, 0.15);
    overflow: hidden;
    animation: mw-sheet-in 200ms ease;
  }
  @keyframes mw-sheet-in {
    from {
      transform: translateY(100%);
    }
    to {
      transform: translateY(0);
    }
  }
</style>
