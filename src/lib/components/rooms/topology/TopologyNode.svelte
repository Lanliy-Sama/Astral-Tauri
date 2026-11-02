<script>
  /**
   * 拓扑节点卡 —— network_topology_nodes.dart 的 buildNode 1:1 移植。
   * 结构：头部 Container(h10v8, 上圆角8, 按类型配色) + 身体 Container(h10v8, 下圆角8,
   * bg surface + border outline) ；两侧 8×8 primary 端口方块（PortTheme size8 color primary，
   * 中心偏移 ±2px）。dense(≥12 节点) 时身体行序为 延迟→IP（无平台行），默认 IP→平台→延迟。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import { latencyColor } from '$lib/utils/display.js';

  let { node, denseMode = false, onMeasure = () => {} } = $props();

  let rootEl = $state(null);

  /* TopologyNodeType → 头部配色 / 图标（headerColor / headerOnColor） */
  const TYPE_STYLE = {
    local: { bg: 'var(--md-primary-container)', fg: 'var(--md-on-primary-container)', icon: 'computer' },
    server: { bg: 'var(--md-tertiary-container)', fg: 'var(--md-on-tertiary-container)', icon: 'cloud' },
    player: { bg: 'var(--md-secondary-container)', fg: 'var(--md-on-secondary-container)', icon: 'person' },
    relay: { bg: 'var(--md-surface-container-highest)', fg: 'var(--md-on-surface-variant)', icon: 'router' },
  };

  const theme = $derived(TYPE_STYLE[node.type] || TYPE_STYLE.player);
  const nodeWidth = $derived(denseMode ? 186 : 220);
  const latColor = $derived(node.latency > 0 ? latencyColor(node.latency) : '');

  /* 上报实测高度：连接线锚点取卡片垂直中心 */
  $effect(() => {
    if (!rootEl) return;
    const id = node.id;
    const report = () => onMeasure(id, rootEl.offsetHeight);
    report();
    const ro = new ResizeObserver(report);
    ro.observe(rootEl);
    return () => ro.disconnect();
  });
</script>

<div
  class="node-card"
  bind:this={rootEl}
  style="left:{node.x}px; top:{node.y}px; width:{nodeWidth}px"
>
  <!-- 头部：图标 18 + 6 + 名(bold 13, ellipsis) -->
  <div class="node-head" style="background:{theme.bg}; color:{theme.fg}">
    <Icon name={theme.icon} size={18} color={theme.fg} />
    <span class="name">{node.displayName}</span>
  </div>
  <!-- 身体：IP / 平台 / 延迟 行 -->
  <div class="node-body">
    {#if denseMode}
      {#if node.latency > 0}
        <div class="row">
          <Icon name="speed" size={13} color="var(--md-on-surface)" />
          <span class="latency" style="color:{latColor}">{node.latency}ms</span>
        </div>
      {/if}
      {#if node.ip != null}
        <div class="row" class:mt={node.latency > 0}>
          <Icon name="language" size={13} color="var(--md-on-surface)" />
          <span class="text ellipsis">{node.ip}</span>
        </div>
      {/if}
    {:else}
      {#if node.ip != null}
        <div class="row">
          <Icon name="language" size={13} color="var(--md-on-surface)" />
          <span class="text">{node.ip}</span>
        </div>
      {/if}
      <div class="row" class:mt={node.ip != null}>
        <Icon name="devices" size={13} color="var(--md-on-surface)" />
        <span class="text ellipsis">{node.platform}</span>
      </div>
      {#if node.latency > 0}
        <div class="row mt">
          <Icon name="speed" size={13} color="var(--md-on-surface)" />
          <span class="latency" style="color:{latColor}">{node.latency}ms</span>
        </div>
      {/if}
    {/if}
  </div>
  <!-- 端口：8×8 primary，中心位于左右边缘外 2px -->
  <span class="port port-in"></span>
  <span class="port port-out"></span>
</div>

<style>
  .node-card {
    position: absolute;
    display: flex;
    flex-direction: column;
  }
  .node-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    border-radius: 8px 8px 0 0;
  }
  .node-head .name {
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 700;
    font-size: 13px;
  }
  .node-body {
    background: var(--md-surface);
    border: 1px solid var(--md-outline);
    border-radius: 0 0 8px 8px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .row.mt {
    margin-top: 4px;
  }
  .text {
    font-size: 11px;
    color: var(--md-on-surface);
    white-space: nowrap;
  }
  .ellipsis {
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .latency {
    font-size: 11px;
    font-weight: 700;
  }
  .port {
    position: absolute;
    top: calc(50% - 4px);
    width: 8px;
    height: 8px;
    background: var(--md-primary);
  }
  .port-in {
    left: -6px;
  }
  .port-out {
    right: -6px;
  }
</style>
