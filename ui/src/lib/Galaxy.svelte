<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { GalaxyEngine } from "./renderer/engine";
  import { createSimulation, snapshotGraph, type SimNode } from "./renderer/layout";
  import { renderNodes, updateNodePositions } from "./renderer/nodes";
  import { renderConnections } from "./renderer/connections";
  import { nodes, edges, impulses, connections, selectedNodeId } from "./stores";
  import { getAllImpulses, getAllConnections } from "./api";
  import type { Simulation } from "d3-force";

  let canvasEl: HTMLCanvasElement;
  let engine: GalaxyEngine;
  let simulation: Simulation<SimNode, any> | null = null;
  let animFrame: number;
  let simNodes: SimNode[] = [];
  let simLinks: any[] = [];
  let nodeClickHandler: ((id: string) => void) | undefined;
  let dragNode: SimNode | null = null;

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
      console.log("[synaptic-graph] No impulses found");
      return;
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

    // Setup drag behavior on canvas
    setupDrag(canvasEl);

    // Animation loop — re-render every frame while simulation is active
    function tick() {
      if (!simulation) return;

      const snap = snapshotGraph(simNodes, simLinks);
      nodes.set(snap.nodes);
      edges.set(snap.edges);

      // Update positions efficiently
      updateNodePositions(engine.nodeLayer, snap.nodes);
      renderConnections(engine.connectionLayer, snap.edges);

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
