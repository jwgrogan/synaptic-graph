<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { GalaxyEngine } from "./renderer/engine";
  import { createSimulation, snapshotGraph, type SimNode } from "./renderer/layout";
  import { renderNodes, updateNodePositions } from "./renderer/nodes";
  import { renderConnections } from "./renderer/connections";
  import { nodes, edges, impulses, connections, selectedNodeId, nodeCount, edgeCount } from "./stores";
  import { getAllImpulses, getAllConnections } from "./api";
  import { showToast } from "./toastStore";
  import type { Simulation } from "d3-force";
  import type { GraphEdge } from "./types";

  let canvasEl: HTMLCanvasElement;
  let engine: GalaxyEngine;
  let simulation: Simulation<SimNode, any> | null = null;
  let animFrame: number;
  let frameCount = 0;
  let simNodes: SimNode[] = [];
  let simLinks: any[] = [];
  let nodeClickHandler: ((id: string) => void) | undefined;
  let dragNode: SimNode | null = null;
  let isEmpty = true;

  // Connection tooltip state
  let connectionTooltip: { x: number; y: number; relationship: string; weight: number } | null = null;

  function saveScreenshot() {
    if (!engine) return;
    const canvas = engine.app.canvas as HTMLCanvasElement;
    const link = document.createElement("a");
    link.download = "synaptic-graph.png";
    link.href = canvas.toDataURL("image/png");
    link.click();
  }

  // Selected node visual state
  $: if (engine && $selectedNodeId) {
    for (const child of engine.nodeLayer.children) {
      if (child.label === $selectedNodeId) {
        child.scale.set(1.3);
        child.alpha = 1.0;
      } else {
        child.alpha = 0.4;
      }
    }
  } else if (engine) {
    for (const child of engine.nodeLayer.children) {
      child.scale.set(1.0);
      child.alpha = 1.0;
    }
  }

  function handleNavigateToNode(e: Event) {
    const detail = (e as CustomEvent).detail;
    const nodeId = detail.id;
    selectedNodeId.set(nodeId);
    const node = $nodes.find(n => n.impulse.id === nodeId);
    if (node && engine) {
      const w = canvasEl.parentElement?.clientWidth || 1400;
      const h = canvasEl.parentElement?.clientHeight || 900;
      engine.zoomToNode(node.x, node.y, w, h);
    }
  }

  // Helper: distance from point to line segment
  function pointToLineDistance(px: number, py: number, x1: number, y1: number, x2: number, y2: number): number {
    const dx = x2 - x1, dy = y2 - y1;
    const lenSq = dx * dx + dy * dy;
    if (lenSq === 0) return Math.sqrt((px - x1) ** 2 + (py - y1) ** 2);
    let t = ((px - x1) * dx + (py - y1) * dy) / lenSq;
    t = Math.max(0, Math.min(1, t));
    const projX = x1 + t * dx, projY = y1 + t * dy;
    return Math.sqrt((px - projX) ** 2 + (py - projY) ** 2);
  }

  // Convert screen mouse position to world coordinates
  function screenToWorld(clientX: number, clientY: number): { wx: number; wy: number } {
    if (!canvasEl || !engine) return { wx: 0, wy: 0 };
    const rect = canvasEl.getBoundingClientRect();
    const scaleX = canvasEl.width / rect.width;
    const scaleY = canvasEl.height / rect.height;
    const mouseX = (clientX - rect.left) * scaleX;
    const mouseY = (clientY - rect.top) * scaleY;
    const wx = (mouseX - engine.camera.x * window.devicePixelRatio) / (engine.camera.zoom * window.devicePixelRatio);
    const wy = (mouseY - engine.camera.y * window.devicePixelRatio) / (engine.camera.zoom * window.devicePixelRatio);
    return { wx, wy };
  }

  function handleCanvasMousemove(e: MouseEvent) {
    if (!engine || $edges.length === 0) {
      connectionTooltip = null;
      return;
    }

    const { wx, wy } = screenToWorld(e.clientX, e.clientY);

    // Threshold in world coordinates: 5px / zoom so it feels consistent
    const threshold = 5 / engine.camera.zoom;

    let closest: GraphEdge | null = null;
    let closestDist = Infinity;
    for (const edge of $edges) {
      const dist = pointToLineDistance(wx, wy, edge.source.x, edge.source.y, edge.target.x, edge.target.y);
      if (dist < threshold && dist < closestDist) {
        closest = edge;
        closestDist = dist;
      }
    }

    if (closest) {
      const rect = canvasEl.getBoundingClientRect();
      connectionTooltip = {
        x: e.clientX - rect.left + 12,
        y: e.clientY - rect.top - 8,
        relationship: closest.connection.relationship,
        weight: closest.connection.weight,
      };
    } else {
      connectionTooltip = null;
    }
  }

  function handleCanvasMouseleave() {
    connectionTooltip = null;
  }

  async function loadAndRender() {
    let impulseData: import("./types").Impulse[] = [];
    let connectionData: import("./types").Connection[] = [];
    try {
      impulseData = await getAllImpulses();
      connectionData = await getAllConnections();
      console.log(`[synaptic-graph] Loaded ${impulseData.length} impulses, ${connectionData.length} connections`);
    } catch (err) {
      showToast("Failed to load graph data: " + (err instanceof Error ? err.message : String(err)));
    }
    impulses.set(impulseData);
    connections.set(connectionData);

    if (impulseData.length === 0) {
      console.log("[synaptic-graph] No impulses found");
      isEmpty = true;
      return;
    }

    isEmpty = false;

    // Wait a tick for the canvas element to render after isEmpty becomes false
    await new Promise((r) => setTimeout(r, 0));

    if (!engine) {
      engine = new GalaxyEngine();
      await engine.init(canvasEl);
      setupDrag(canvasEl);
    }

    // Stop any existing simulation
    if (simulation) {
      simulation.stop();
      cancelAnimationFrame(animFrame);
    }

    // Create live simulation
    const w = canvasEl.parentElement?.clientWidth || 1400;
    const h = canvasEl.parentElement?.clientHeight || 900;
    const sim = createSimulation(impulseData, connectionData, w, h);
    simulation = sim.simulation;
    simNodes = sim.simNodes;
    simLinks = sim.simLinks;

    // Click handler
    nodeClickHandler = (id: string) => {
      console.log(`[synaptic-graph] Node clicked: ${id}`);
      selectedNodeId.set(id);
    };

    // Initial render
    const snap = snapshotGraph(simNodes, simLinks);
    nodes.set(snap.nodes);
    edges.set(snap.edges);
    renderNodes(engine.nodeLayer, snap.nodes, nodeClickHandler);
    renderConnections(engine.connectionLayer, snap.edges);

    // Animation loop — re-render every frame while simulation is active
    function tick() {
      if (!simulation) return;
      frameCount++;

      const snap = snapshotGraph(simNodes, simLinks);
      nodes.set(snap.nodes);
      edges.set(snap.edges);

      // Update positions efficiently
      updateNodePositions(engine.nodeLayer, snap.nodes);

      // Only re-render connections every 3 frames (they're expensive)
      if (frameCount % 3 === 0) {
        renderConnections(engine.connectionLayer, snap.edges);
      }

      // Zoom-dependent label visibility
      const showAllLabels = engine.camera.zoom > 1.5;
      for (const child of engine.nodeLayer.children) {
        const lbl = child.children?.find((c: any) => c.label === "nodelabel");
        if (lbl && showAllLabels) {
          lbl.alpha = 1;
        }
        // Don't hide labels here — hover handler manages that when zoomed out
      }

      // Keep requesting frames while simulation has energy
      if (simulation.alpha() > 0.001) {
        animFrame = requestAnimationFrame(tick);
      }
    }

    // Listen for simulation ticks
    simulation.on("tick", () => {
      // Just request a render frame
      cancelAnimationFrame(animFrame);
      animFrame = requestAnimationFrame(tick);
    });

    // Initial render frame
    animFrame = requestAnimationFrame(tick);
  }

  async function refreshGraph() {
    try {
      await loadAndRender();
      showToast("Graph refreshed", "success");
    } catch (err) {
      showToast("Failed to refresh graph: " + (err instanceof Error ? err.message : String(err)));
    }
  }

  onMount(async () => {
    window.addEventListener("navigate-to-node", handleNavigateToNode);
    await loadAndRender();
  });

  function setupDrag(canvas: HTMLCanvasElement) {
    let isDraggingNode = false;

    canvas.addEventListener("mousedown", (e) => {
      if (!simulation) return;

      // Find if we clicked a node
      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width / rect.width;
      const scaleY = canvas.height / rect.height;
      const mouseX = (e.clientX - rect.left) * scaleX;
      const mouseY = (e.clientY - rect.top) * scaleY;

      // Transform to world coordinates
      const worldX = (mouseX - engine.camera.x * window.devicePixelRatio) / (engine.camera.zoom * window.devicePixelRatio);
      const worldY = (mouseY - engine.camera.y * window.devicePixelRatio) / (engine.camera.zoom * window.devicePixelRatio);

      // Find closest node
      let closest: SimNode | null = null;
      let closestDist = Infinity;
      for (const n of simNodes) {
        const dx = (n.x ?? 0) - worldX;
        const dy = (n.y ?? 0) - worldY;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < n.radius + 8 && dist < closestDist) {
          closest = n;
          closestDist = dist;
        }
      }

      if (closest) {
        isDraggingNode = true;
        dragNode = closest;
        dragNode.fx = dragNode.x;
        dragNode.fy = dragNode.y;
        simulation!.alphaTarget(0.3).restart();
        e.stopPropagation();
      }
    });

    canvas.addEventListener("mousemove", (e) => {
      if (!isDraggingNode || !dragNode || !simulation) return;

      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width / rect.width;
      const scaleY = canvas.height / rect.height;
      const mouseX = (e.clientX - rect.left) * scaleX;
      const mouseY = (e.clientY - rect.top) * scaleY;

      const worldX = (mouseX - engine.camera.x * window.devicePixelRatio) / (engine.camera.zoom * window.devicePixelRatio);
      const worldY = (mouseY - engine.camera.y * window.devicePixelRatio) / (engine.camera.zoom * window.devicePixelRatio);

      dragNode.fx = worldX;
      dragNode.fy = worldY;
    });

    canvas.addEventListener("mouseup", () => {
      if (isDraggingNode && dragNode) {
        dragNode.fx = null;
        dragNode.fy = null;
        simulation?.alphaTarget(0);
        isDraggingNode = false;
        dragNode = null;
      }
    });
  }

  onDestroy(() => {
    cancelAnimationFrame(animFrame);
    simulation?.stop();
    engine?.destroy();
    window.removeEventListener("navigate-to-node", handleNavigateToNode);
  });
</script>

{#if isEmpty}
<div class="empty-state">
  <div class="empty-icon">&#x2B21;</div>
  <h2>Your memory graph is empty</h2>
  <p>Start a conversation with an AI assistant that has Synaptic Graph connected. Memories will appear here as you save them.</p>
  <div class="empty-hints">
    <div class="hint">Say <strong>"remember this"</strong> to save a memory</div>
    <div class="hint">Use <strong>Cmd+K</strong> to search your memories</div>
    <div class="hint">Go to <strong>Import</strong> to bring memories from other providers</div>
  </div>
</div>
{:else}
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="galaxy-wrapper" on:mousemove={handleCanvasMousemove} on:mouseleave={handleCanvasMouseleave}>
  <canvas bind:this={canvasEl} class="galaxy-canvas"></canvas>

  <button class="refresh-btn" on:click={refreshGraph} title="Refresh graph">&#8635;</button>

  <button class="screenshot-btn" on:click={saveScreenshot} title="Save screenshot">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/>
      <circle cx="8.5" cy="8.5" r="1.5"/>
      <polyline points="21 15 16 10 5 21"/>
    </svg>
  </button>

  {#if connectionTooltip}
  <div class="connection-tooltip" style="left: {connectionTooltip.x}px; top: {connectionTooltip.y}px">
    {connectionTooltip.relationship} &middot; {connectionTooltip.weight.toFixed(2)}
  </div>
  {/if}

  {#if $nodeCount > 0}
  <div class="graph-stats">
    <span>{$nodeCount} nodes</span>
    <span class="dot">&middot;</span>
    <span>{$edgeCount} connections</span>
  </div>
  {/if}
</div>
{/if}

<style>
  .galaxy-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
  }

  .galaxy-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }

  .empty-state {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    text-align: center;
    padding: 40px;
  }

  .empty-icon {
    font-size: 56px;
    color: var(--accent-primary);
    line-height: 1;
    margin-bottom: 4px;
    opacity: 0.8;
  }

  .empty-state h2 {
    font-size: 18px;
    font-weight: 500;
    color: var(--text-secondary);
    margin: 0;
  }

  .empty-state p {
    font-size: 13px;
    color: var(--text-muted);
    max-width: 380px;
    line-height: 1.5;
    margin: 0;
  }

  .empty-hints {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 20px;
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted);
    padding: 6px 14px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
  }

  .hint strong {
    color: var(--text-secondary);
  }

  .graph-stats {
    position: absolute;
    bottom: 12px;
    left: 12px;
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.7;
    display: flex;
    gap: 6px;
    pointer-events: none;
    user-select: none;
  }

  .graph-stats .dot {
    opacity: 0.5;
  }

  .refresh-btn {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 1px solid var(--border-subtle);
    background: var(--bg-panel);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
    z-index: 10;
    padding: 0;
    line-height: 1;
  }

  .refresh-btn:hover {
    color: var(--text-primary);
    border-color: var(--border-medium);
    background: var(--bg-hover);
  }

  .screenshot-btn {
    position: absolute;
    bottom: 12px;
    right: 12px;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-panel);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
    z-index: 10;
  }

  .screenshot-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--border-medium);
  }

  .connection-tooltip {
    position: absolute;
    pointer-events: none;
    background: var(--bg-panel-solid);
    color: var(--text-secondary);
    font-size: 11px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    box-shadow: var(--shadow-sm);
    white-space: nowrap;
    z-index: 20;
  }
</style>
