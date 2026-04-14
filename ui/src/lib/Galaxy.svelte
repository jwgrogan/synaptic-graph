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
    const impulseData = await getAllImpulses();
    const connectionData = await getAllConnections();
    impulses.set(impulseData);
    connections.set(connectionData);

    // Compute layout
    const layout = computeLayout(impulseData, connectionData);
    nodes.set(layout.nodes);
    edges.set(layout.edges);

    // Build nebula cluster info
    const nodePositions = new Map<string, { x: number; y: number }>();
    for (const node of layout.nodes) {
      nodePositions.set(node.impulse.id, { x: node.x, y: node.y });
    }
    const nodeToCluster = detectClusters(impulseData, connectionData);
    const clusterInfo = buildClusterInfo(nodeToCluster, nodePositions);

    // Render
    engine.renderGraph(layout.nodes, layout.edges);
    engine.renderNebulae(clusterInfo, nodePositions);

    // Handle node clicks
    engine.nodeLayer.on("pointerdown", (e) => {
      const target = e.target;
      if (target?.label) {
        selectedNodeId.set(target.label);
      }
    });
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
