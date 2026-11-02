<script>
  /**
   * 网络拓扑视图 —— network_topology.dart + network_topology_build.dart +
   * network_topology_ids.dart 1:1 移植（vyuh_node_flow 编辑器行为以 SVG + absolute 卡片复刻）。
   *
   * 布局算法（严格按 network_topology_build.dart）：
   *   localX=120 targetX=760 baseY=80 rowGap=130；服务器行在上玩家行在下（各按 peerId 升序），
   *   两组都有时隔一行；本地 x=120, y=80+(totalRows-1)*130/2（totalRows<=1 时 y=250）；
   *   非本地 (760, 80+rowIndex*130)；hops 中继链：本地→各跳→目标，
   *   跳 x=120+640/(hopCount+1)*(i+1)，y 与目标同行，displayName=nodeName||'relay_${targetIp}'。
   *
   * 同步节流：结构/指标 1s（reduceUpdates 时 4s）、后台暂停、恢复前台强制同步；
   * 结构变化才重排（已有节点沿用旧坐标 —— existingPositions 语义）。
   */
  import Icon from '$lib/components/common/Icon.svelte';
  import TopologyNode from './TopologyNode.svelte';
  import { sm } from '$lib/services/service-manager.svelte.js';
  import { parsePlatformVersion } from '$lib/utils/display.js';
  import { isServerNode } from '$lib/utils/network.js';

  let { nodes = [], reduceUpdates = false, isInBackground = false } = $props();

  /* ============ 布局常量（NetworkTopologyBuild） ============ */
  const LOCAL_X = 120;
  const TARGET_X = 760;
  const BASE_Y = 80;
  const ROW_GAP = 130;
  const REDUCED_SYNC_INTERVAL = 4000; // network_topology_graph.dart
  const METRICS_SYNC_INTERVAL = 1000;

  /* ============ ID 工具（NetworkTopologyIds） ============ */
  function normalizeNodeName(hostname) {
    return (hostname || '').startsWith('PublicServer_')
      ? hostname.substring('PublicServer_'.length)
      : hostname || '';
  }
  function safeId(raw) {
    return String(raw).replace(/[^A-Za-z0-9_]/g, '_');
  }
  function nodeFallbackKey(nodeInfo) {
    if (nodeInfo.ipv4) return `ip_${nodeInfo.ipv4}`;
    return `name_${normalizeNodeName(nodeInfo.hostname)}`;
  }
  function hopFallbackKey(hop) {
    if (hop.targetIp) return `ip_${hop.targetIp}`;
    return `name_${normalizeNodeName(hop.nodeName)}`;
  }
  function nodeIdForPeerId(peerId, fallbackKey) {
    if (peerId > 0) return `peer_${peerId}`;
    return safeId(fallbackKey);
  }
  function resolveNodeId(peerId, fallbackKey, byPeerId, byFallbackKey) {
    if (peerId > 0) return byPeerId.get(peerId);
    return byFallbackKey.get(fallbackKey);
  }
  function formatLatencyLabel(latencyMs) {
    return `${Math.round(Number(latencyMs) || 0)}ms`;
  }

  /* ============ 图模型构建（NetworkTopologyBuild.buildGraphModel） ============ */
  function buildGraphModel(nodes, localIp, existingPositions, shouldAnimate) {
    const newNodes = [];
    const nodeById = new Map();
    const connMap = new Map();
    const nodeIdsByPeerId = new Map();
    const nodeIdsByFallbackKey = new Map();
    const nodeRowIndex = new Map();

    let localNode = null;
    for (const node of nodes) {
      if (localIp && node.ipv4 === localIp) {
        localNode = node;
        break;
      }
    }

    const nonLocalServers = [];
    const nonLocalPlayers = [];
    for (const node of nodes) {
      if (localNode != null && node.peerId === localNode.peerId) continue;
      if (isServerNode(node)) nonLocalServers.push(node);
      else nonLocalPlayers.push(node);
    }
    nonLocalServers.sort((a, b) => (a.peerId ?? 0) - (b.peerId ?? 0));
    nonLocalPlayers.sort((a, b) => (a.peerId ?? 0) - (b.peerId ?? 0));

    const hasLaneSpacer = nonLocalServers.length > 0 && nonLocalPlayers.length > 0;
    const totalRows =
      nonLocalServers.length + nonLocalPlayers.length + (hasLaneSpacer ? 1 : 0);

    const nonLocalIndexByPeerId = new Map();
    const nonLocalIndexByFallbackKey = new Map();
    for (let i = 0; i < nonLocalServers.length; i++) {
      const server = nonLocalServers[i];
      if (server.peerId > 0) nonLocalIndexByPeerId.set(server.peerId, i);
      nonLocalIndexByFallbackKey.set(nodeFallbackKey(server), i);
    }
    for (let i = 0; i < nonLocalPlayers.length; i++) {
      const player = nonLocalPlayers[i];
      const row = i + nonLocalServers.length + (hasLaneSpacer ? 1 : 0);
      if (player.peerId > 0) nonLocalIndexByPeerId.set(player.peerId, row);
      nonLocalIndexByFallbackKey.set(nodeFallbackKey(player), row);
    }

    let rowIndex = 0;
    for (const nodeInfo of nodes) {
      const isLocal = localNode != null && nodeInfo.peerId === localNode.peerId;
      const isServer = isServerNode(nodeInfo);
      const nodeId = nodeIdForPeerId(nodeInfo.peerId, nodeFallbackKey(nodeInfo));

      if (nodeInfo.peerId > 0) nodeIdsByPeerId.set(nodeInfo.peerId, nodeId);
      nodeIdsByFallbackKey.set(nodeFallbackKey(nodeInfo), nodeId);

      const displayName = normalizeNodeName(nodeInfo.hostname);
      let x;
      let y;

      if (existingPositions.has(nodeId)) {
        const pos = existingPositions.get(nodeId);
        x = pos.x;
        y = pos.y;
      } else if (isLocal) {
        const localY = totalRows <= 1 ? 250 : BASE_Y + ((totalRows - 1) * ROW_GAP) / 2;
        x = LOCAL_X;
        y = localY;
      } else {
        const index =
          nonLocalIndexByPeerId.get(nodeInfo.peerId) ??
          nonLocalIndexByFallbackKey.get(nodeFallbackKey(nodeInfo)) ??
          -1;
        const effectiveIndex = index >= 0 ? index : rowIndex;
        x = TARGET_X;
        y = BASE_Y + effectiveIndex * ROW_GAP;
        nodeRowIndex.set(nodeId, effectiveIndex);
        rowIndex = Math.max(rowIndex, effectiveIndex + 1);
      }

      const nodeObj = {
        id: nodeId,
        type: isLocal ? 'local' : isServer ? 'server' : 'player',
        x,
        y,
        displayName,
        ip: isServer && !isLocal ? null : nodeInfo.ipv4,
        platform: parsePlatformVersion(nodeInfo.version).name,
        latency: isLocal ? 0 : Math.trunc(nodeInfo.latencyMs ?? 0),
      };
      newNodes.push(nodeObj);
      nodeById.set(nodeId, nodeObj);
    }

    const localNodeId =
      localNode == null
        ? null
        : resolveNodeId(
            localNode.peerId,
            nodeFallbackKey(localNode),
            nodeIdsByPeerId,
            nodeIdsByFallbackKey
          );

    if (localNodeId == null) {
      return { nodes: newNodes, connections: [...connMap.values()] };
    }

    for (const nodeInfo of nodes) {
      if (localNode != null && nodeInfo.peerId === localNode.peerId) continue;

      const targetId =
        resolveNodeId(
          nodeInfo.peerId,
          nodeFallbackKey(nodeInfo),
          nodeIdsByPeerId,
          nodeIdsByFallbackKey
        ) ?? nodeIdForPeerId(nodeInfo.peerId, nodeFallbackKey(nodeInfo));

      const hops = nodeInfo.hops || [];
      if (hops.length > 0) {
        let previousNodeId = localNodeId;
        const targetRow = nodeRowIndex.get(targetId) ?? 0;
        const hopCount = hops.length;

        for (let hopIndex = 0; hopIndex < hopCount; hopIndex++) {
          const hop = hops[hopIndex];
          const hopId =
            resolveNodeId(
              hop.peerId,
              hopFallbackKey(hop),
              nodeIdsByPeerId,
              nodeIdsByFallbackKey
            ) ?? nodeIdForPeerId(hop.peerId, hopFallbackKey(hop));

          if (!nodeById.has(hopId)) {
            const displayName = normalizeNodeName(
              hop.nodeName ? hop.nodeName : `relay_${hop.targetIp}`
            );
            const hopX = LOCAL_X + ((TARGET_X - LOCAL_X) / (hopCount + 1)) * (hopIndex + 1);
            const pos =
              existingPositions.get(hopId) ??
              { x: hopX, y: BASE_Y + targetRow * ROW_GAP };
            const hopNode = {
              id: hopId,
              type: 'relay',
              x: pos.x,
              y: pos.y,
              displayName,
              ip: hop.targetIp,
              platform: 'Relay',
              latency: Math.trunc(hop.latencyMs ?? 0),
            };
            newNodes.push(hopNode);
            nodeById.set(hopId, hopNode);
          }

          if (previousNodeId !== hopId) {
            const connId = `conn_${previousNodeId}_to_${hopId}`;
            connMap.set(connId, {
              id: connId,
              sourceId: previousNodeId,
              targetId: hopId,
              label: formatLatencyLabel(hop.latencyMs),
              animate: shouldAnimate,
            });
          }
          previousNodeId = hopId;
        }

        if (previousNodeId !== targetId) {
          const connId = `conn_${previousNodeId}_to_${targetId}`;
          connMap.set(connId, {
            id: connId,
            sourceId: previousNodeId,
            targetId,
            label: formatLatencyLabel(nodeInfo.latencyMs),
            animate: shouldAnimate,
          });
        }
      } else {
        if (localNodeId !== targetId) {
          const connId = `conn_${localNodeId}_to_${targetId}`;
          connMap.set(connId, {
            id: connId,
            sourceId: localNodeId,
            targetId,
            label: formatLatencyLabel(nodeInfo.latencyMs),
            animate: shouldAnimate,
          });
        }
      }
    }

    return { nodes: newNodes, connections: [...connMap.values()] };
  }

  /* ============ 同步节流（_syncGraph/_syncMetrics 语义） ============ */
  let syncedNodes = $state(Array.isArray(nodes) ? [...nodes] : []);
  let lastSyncAt = 0;
  let wasInBackground = false;
  $effect(() => {
    if (isInBackground) {
      wasInBackground = true;
      return;
    }
    const now = Date.now();
    const interval = reduceUpdates ? REDUCED_SYNC_INTERVAL : METRICS_SYNC_INTERVAL;
    if (wasInBackground || now - lastSyncAt >= interval - 500) {
      syncedNodes = Array.isArray(nodes) ? [...nodes] : [];
      lastSyncAt = now;
      wasInBackground = false;
    }
  });

  const shouldAnimate = $derived(!reduceUpdates && !isInBackground);

  /* 坐标持久化：结构变化时已有节点沿用旧坐标（existingPositions） */
  const existingPositions = new Map();
  const model = $derived(
    buildGraphModel(syncedNodes, sm.networkConfigState.ipv4, existingPositions, shouldAnimate)
  );
  $effect(() => {
    existingPositions.clear();
    for (const n of model.nodes) existingPositions.set(n.id, { x: n.x, y: n.y });
  });

  /* 初始视口自适应：首次出现节点时缩放并居中（NodeFlowEditor 初始视图语义） */
  let didFit = false;
  function fitView() {
    if (!rootEl || model.nodes.length === 0) return;
    const vw = rootEl.clientWidth;
    const vh = rootEl.clientHeight;
    const PAD = 48;
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const n of model.nodes) {
      const h = nodeHeights[n.id] ?? 84;
      minX = Math.min(minX, n.x);
      minY = Math.min(minY, n.y);
      maxX = Math.max(maxX, n.x + nodeWidth);
      maxY = Math.max(maxY, n.y + h);
    }
    const w = Math.max(1, maxX - minX);
    const hh = Math.max(1, maxY - minY);
    const scale = Math.min(2.0, Math.max(0.3, Math.min((vw - PAD * 2) / w, (vh - PAD * 2) / hh)));
    zoom = scale;
    panX = (vw - w * scale) / 2 - minX * scale;
    panY = (vh - hh * scale) / 2 - minY * scale;
  }
  $effect(() => {
    if (didFit || !rootEl || model.nodes.length === 0) return;
    didFit = true;
    requestAnimationFrame(fitView);
    /* 子组件完成高度测量后再校正一次 */
    setTimeout(fitView, 400);
  });

  /* ============ 连线几何（端口锚点 = 卡片垂直中心，边缘外 2px） ============ */
  let nodeHeights = $state({});
  function handleMeasure(id, h) {
    if (nodeHeights[id] !== h) nodeHeights[id] = h;
  }

  const denseMode = $derived(Array.isArray(nodes) && nodes.length >= 12);
  const nodeWidth = $derived(denseMode ? 186 : 220);

  const edges = $derived.by(() => {
    const byId = new Map(model.nodes.map((n) => [n.id, n]));
    const list = [];
    for (const c of model.connections) {
      const src = byId.get(c.sourceId);
      const tgt = byId.get(c.targetId);
      if (!src || !tgt) continue;
      const srcH = nodeHeights[c.sourceId] ?? 84;
      const tgtH = nodeHeights[c.targetId] ?? 84;
      const x1 = src.x + nodeWidth + 2;
      const y1 = src.y + srcH / 2;
      const x2 = tgt.x - 2;
      const y2 = tgt.y + tgtH / 2;
      const dx = Math.max(40, (x2 - x1) / 2);
      const c1x = x1 + dx;
      const c2x = x2 - dx;
      list.push({
        ...c,
        path: `M ${x1} ${y1} C ${c1x} ${y1}, ${c2x} ${y2}, ${x2} ${y2}`,
        midX: (x1 + 3 * c1x + 3 * c2x + x2) / 8,
        midY: (y1 + y2) / 2,
      });
    }
    return list;
  });

  /* ============ 图例（buildLegendCard） ============ */
  const legendCounts = $derived.by(() => {
    const localIp = sm.networkConfigState.ipv4;
    let serverCount = 0;
    let playerCount = 0;
    for (const node of nodes || []) {
      if (localIp && node.ipv4 === localIp) continue;
      if (isServerNode(node)) serverCount++;
      else playerCount++;
    }
    return { serverCount, playerCount };
  });

  /* 色块图标前景色：亮色容器 → black87，深色容器 → white（estimateBrightnessForColor） */
  function onColorFor(varName) {
    try {
      const value = getComputedStyle(document.documentElement).getPropertyValue(varName).trim();
      const m = value.match(/^rgba?\(([^)]+)\)$/i);
      if (m) {
        const p = m[1].split(',').map((s) => parseFloat(s));
        const alpha = p.length > 3 ? p[3] : 1;
        if (alpha < 0.5) return '#000000de';
        const lin = (c) => {
          c /= 255;
          return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
        };
        const lum = 0.2126 * lin(p[0]) + 0.7152 * lin(p[1]) + 0.0722 * lin(p[2]);
        return lum < 0.35 ? '#ffffff' : '#000000de';
      }
    } catch {
      /* ignore */
    }
    return '#000000de';
  }

  const legendItems = $derived.by(() => {
    void legendCounts;
    return [
      {
        key: 'local',
        bg: 'var(--md-primary-container)',
        on: onColorFor('--md-primary-container'),
        icon: 'computer',
        label: 'Local',
      },
      {
        key: 'server',
        bg: 'var(--md-tertiary-container)',
        on: onColorFor('--md-tertiary-container'),
        icon: 'cloud',
        label: `Server ${legendCounts.serverCount}`,
      },
      {
        key: 'player',
        bg: 'var(--md-secondary-container)',
        on: onColorFor('--md-secondary-container'),
        icon: 'person',
        label: `Player ${legendCounts.playerCount}`,
      },
    ];
  });

  /* ============ 视口：滚轮缩放 0.3-2.0 + 拖拽平移（NodeFlowConfig） ============ */
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let dragging = $state(false);
  let rootEl = $state(null);
  let dragStart = null;

  function onPointerDown(e) {
    dragStart = { x: e.clientX, y: e.clientY, panX, panY };
    dragging = true;
    rootEl?.setPointerCapture(e.pointerId);
  }
  function onPointerMove(e) {
    if (!dragging || !dragStart) return;
    panX = dragStart.panX + (e.clientX - dragStart.x);
    panY = dragStart.panY + (e.clientY - dragStart.y);
  }
  function onPointerUp() {
    dragging = false;
    dragStart = null;
  }

  /* wheel 缩放（锚定光标），非 passive 以便 preventDefault */
  $effect(() => {
    if (!rootEl) return;
    const onWheel = (e) => {
      e.preventDefault();
      const rect = rootEl.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      const factor = Math.exp(-e.deltaY / 600);
      const next = Math.min(2.0, Math.max(0.3, zoom * factor));
      const k = next / zoom;
      panX = mx - (mx - panX) * k;
      panY = my - (my - panY) * k;
      zoom = next;
    };
    rootEl.addEventListener('wheel', onWheel, { passive: false });
    return () => rootEl.removeEventListener('wheel', onWheel);
  });
</script>

<div
  class="topology-view"
  class:dragging
  bind:this={rootEl}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
>
  <div class="world" style="transform: translate({panX}px, {panY}px) scale({zoom})">
    <!-- 点网格：size 20，色 outlineVariant（GridStyles.dots） -->
    <div class="grid-layer"></div>

    <!-- 连线：bezier，primary@0.6，宽 2.5，label 'XXms'；粒子 stroke-dashoffset 动画（4s） -->
    <svg class="edges-svg" aria-hidden="true">
      {#each edges as e (e.id)}
        <path class="edge-line" d={e.path} />
      {/each}
      {#if shouldAnimate}
        {#each edges as e (e.id)}
          <path class="edge-particles" d={e.path} pathLength="100" />
        {/each}
      {/if}
    </svg>
    {#each edges as e (e.id)}
      <div class="edge-label" style="left:{e.midX}px; top:{e.midY}px">{e.label}</div>
    {/each}

    <!-- 节点卡：宽 dense(≥12) ? 186 : 220 -->
    {#each model.nodes as n (n.id)}
      <TopologyNode node={n} denseMode={denseMode} onMeasure={handleMeasure} />
    {/each}
  </div>

  <!-- 图例卡：top 12 left 12（buildLegendCard） -->
  <div class="legend">
    <div class="legend-title">Topology</div>
    <div class="legend-items">
      {#each legendItems as item (item.key)}
        <span class="legend-item">
          <span class="legend-swatch" style="background:{item.bg}">
            <Icon name={item.icon} size={12} color={item.on} />
          </span>
          <span class="legend-label">{item.label}</span>
        </span>
      {/each}
    </div>
    {#if legendCounts.serverCount > 0 && legendCounts.playerCount > 0}
      <div class="legend-note">Servers are above players.</div>
    {/if}
  </div>
</div>

<style>
  .topology-view {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--md-surface);
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .topology-view.dragging {
    cursor: grabbing;
  }
  .world {
    position: absolute;
    left: 0;
    top: 0;
    width: 0;
    height: 0;
    transform-origin: 0 0;
  }
  .grid-layer {
    position: absolute;
    left: -8000px;
    top: -8000px;
    width: 16000px;
    height: 16000px;
    background-image: radial-gradient(circle, var(--md-outline-variant) 1px, transparent 1px);
    background-size: 20px 20px;
  }
  .edges-svg {
    position: absolute;
    left: 0;
    top: 0;
    width: 0;
    height: 0;
    overflow: visible;
    pointer-events: none;
  }
  .edge-line {
    fill: none;
    stroke: var(--md-primary-06b);
    stroke-width: 2.5;
  }
  .edge-particles {
    fill: none;
    stroke: var(--md-primary);
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-dasharray: 2 8;
    animation: topology-dash 4s linear infinite;
  }
  @keyframes topology-dash {
    from {
      stroke-dashoffset: 0;
    }
    to {
      stroke-dashoffset: -100;
    }
  }
  .edge-label {
    position: absolute;
    transform: translate(-50%, -50%);
    font-size: 11px;
    line-height: 16px;
    color: var(--md-on-surface);
    background: color-mix(in srgb, var(--md-surface) 88%, transparent);
    padding: 0 4px;
    border-radius: 4px;
    white-space: nowrap;
    pointer-events: none;
  }
  .legend {
    position: absolute;
    top: 12px;
    left: 12px;
    background: color-mix(in srgb, var(--md-surface) 92%, transparent);
    border-radius: 12px;
    padding: 8px 10px;
    z-index: 10;
    pointer-events: none;
  }
  .legend-title {
    font-weight: 700;
    color: var(--md-on-surface);
  }
  .legend-items {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 10px;
    margin-top: 6px;
  }
  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .legend-swatch {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 3px;
    border-radius: 4px;
  }
  .legend-label {
    font-size: 11px;
    color: var(--md-on-surface);
  }
  .legend-note {
    margin-top: 6px;
    font-size: 11px;
    color: var(--md-on-surface-variant);
  }
</style>
