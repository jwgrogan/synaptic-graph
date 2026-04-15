import { Application, Container } from "pixi.js";
import type { GraphNode, GraphEdge } from "../types";
import { renderNodes, updateNodes } from "./nodes";
import { renderConnections, updateConnections } from "./connections";
import { renderNebulae } from "./nebula";
import type { Cluster } from "./clusters";
import { Camera } from "./camera";

export class GalaxyEngine {
  app: Application;
  camera: Camera;
  worldContainer: Container;
  nebulaLayer: Container;
  ghostLayer: Container;
  connectionLayer: Container;
  nodeLayer: Container;
  private resizeObserver: ResizeObserver | null = null;

  constructor() {
    this.app = new Application();
    this.camera = new Camera();
    this.worldContainer = new Container();
    this.nebulaLayer = new Container();
    this.ghostLayer = new Container();
    this.connectionLayer = new Container();
    this.nodeLayer = new Container();
  }

  async init(canvas: HTMLCanvasElement) {
    await this.app.init({
      canvas,
      background: 0x06060f,
      resizeTo: canvas.parentElement ?? undefined,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    });

    this.worldContainer.addChild(this.nebulaLayer);
    this.worldContainer.addChild(this.ghostLayer);
    this.worldContainer.addChild(this.connectionLayer);
    this.worldContainer.addChild(this.nodeLayer);
    this.app.stage.addChild(this.worldContainer);

    // Enable interactivity
    this.app.stage.eventMode = "static";
    this.app.stage.hitArea = this.app.screen;

    this.setupPanZoom(canvas);
  }

  private setupPanZoom(canvas: HTMLCanvasElement) {
    let isDragging = false;
    let lastX = 0;
    let lastY = 0;

    canvas.addEventListener("mousedown", (e) => {
      isDragging = true;
      lastX = e.clientX;
      lastY = e.clientY;
    });

    canvas.addEventListener("mousemove", (e) => {
      if (!isDragging) return;
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      this.camera.pan(dx, dy);
      lastX = e.clientX;
      lastY = e.clientY;
      this.applyCameraTransform();
    });

    canvas.addEventListener("mouseup", () => {
      isDragging = false;
    });

    canvas.addEventListener("mouseleave", () => {
      isDragging = false;
    });

    canvas.addEventListener("wheel", (e) => {
      e.preventDefault();
      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      this.camera.zoomAt(e.offsetX, e.offsetY, zoomFactor);
      this.applyCameraTransform();
    }, { passive: false });
  }

  applyCameraTransform() {
    this.worldContainer.x = this.camera.x;
    this.worldContainer.y = this.camera.y;
    this.worldContainer.scale.set(this.camera.zoom);
  }

  renderGraph(
    nodes: GraphNode[],
    edges: GraphEdge[],
    onNodeClick?: (nodeId: string) => void
  ) {
    renderConnections(this.connectionLayer, edges);
    renderNodes(this.nodeLayer, nodes, onNodeClick);
    this.centerOnGraph(nodes);
  }

  updateGraph(nodes: GraphNode[], edges: GraphEdge[]) {
    updateConnections(this.connectionLayer, edges);
    updateNodes(this.nodeLayer, nodes);
  }

  centerOnGraph(nodes: GraphNode[]) {
    if (nodes.length === 0) return;
    const cx = nodes.reduce((s, n) => s + n.x, 0) / nodes.length;
    const cy = nodes.reduce((s, n) => s + n.y, 0) / nodes.length;
    const screenW = this.app.screen.width;
    const screenH = this.app.screen.height;
    this.camera.x = screenW / 2 - cx * this.camera.zoom;
    this.camera.y = screenH / 2 - cy * this.camera.zoom;
    this.applyCameraTransform();
  }

  zoomToNode(x: number, y: number, screenWidth: number, screenHeight: number) {
    const targetZoom = 2.0;
    this.camera.zoom = targetZoom;
    this.camera.x = screenWidth / 2 - x * targetZoom;
    this.camera.y = screenHeight / 2 - y * targetZoom;
    this.applyCameraTransform();
  }

  renderNebulae(
    clusters: Cluster[],
    nodePositions: Map<string, { x: number; y: number }>
  ) {
    renderNebulae(this.nebulaLayer, clusters, nodePositions);
  }

  destroy() {
    this.resizeObserver?.disconnect();
    this.app.destroy(true);
  }
}
