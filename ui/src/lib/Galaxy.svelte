<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { GalaxyEngine } from "./renderer/engine";
  import { computeLayout } from "./renderer/layout";
  import { detectClusters, buildClusterInfo } from "./renderer/clusters";
  import { nodes, edges, impulses, connections, selectedNodeId } from "./stores";
  import { getAllImpulses, getAllConnections } from "./api";

  let canvasEl: HTMLCanvasElement;
  let engine: GalaxyEngine;

  onMount(async () => {
    engine = new GalaxyEngine();
    await engine.init(canvasEl);

    // Load data
    let impulseData: import("./types").Impulse[] = [];
    let connectionData: import("./types").Connection[] = [];
    try {
      impulseData = await getAllImpulses();
      connectionData = await getAllConnections();
      console.log(`[synaptic-graph] Loaded ${impulseData.length} impulses, ${connectionData.length} connections`);
    } catch (err) {
      console.error("[synaptic-graph] Failed to load data:", err);
    }
    impulses.set(impulseData);
    connections.set(connectionData);

    if (impulseData.length === 0) {
      console.log("[synaptic-graph] No impulses found — galaxy will be empty");
      return;
    }

    // Compute layout
    const layout = computeLayout(impulseData, connectionData);
    nodes.set(layout.nodes);
    edges.set(layout.edges);
    console.log(`[synaptic-graph] Layout computed: ${layout.nodes.length} nodes at positions`, layout.nodes.map(n => ({ id: n.impulse.id.slice(0,8), x: Math.round(n.x), y: Math.round(n.y) })));

    // Build nebula cluster info
    const nodePositions = new Map<string, { x: number; y: number }>();
    for (const node of layout.nodes) {
      nodePositions.set(node.impulse.id, { x: node.x, y: node.y });
    }
    const nodeToCluster = detectClusters(impulseData, connectionData);
    const clusterInfo = buildClusterInfo(nodeToCluster, nodePositions);

    // Render with click handler
    engine.renderGraph(layout.nodes, layout.edges, (nodeId: string) => {
      console.log(`[synaptic-graph] Node clicked: ${nodeId}`);
      selectedNodeId.set(nodeId);
    });
    engine.renderNebulae(clusterInfo, nodePositions);
  });

  onDestroy(() => {
    engine?.destroy();
  });
</script>

<canvas bind:this={canvasEl} class="galaxy-canvas"></canvas>

<style>
  .galaxy-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
