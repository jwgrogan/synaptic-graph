import { Container, Graphics } from "pixi.js";
import type { GraphNode } from "../types";

export function renderGhostNodes(layer: Container, ghostNodes: GraphNode[]) {
  for (const node of ghostNodes) {
    const g = new Graphics();

    // Parse cluster color
    const colorHex = node.color || "#8E99A4";
    const cr = parseInt(colorHex.slice(1, 3), 16);
    const cg = parseInt(colorHex.slice(3, 5), 16);
    const cb = parseInt(colorHex.slice(5, 7), 16);
    const nodeColor = (cr << 16) | (cg << 8) | cb;

    // Ghost node: same as regular but at 30% opacity
    // Subtle drop shadow
    g.circle(0, 1.5, node.radius);
    g.fill({ color: 0x000000, alpha: 0.02 });

    // Main circle at 30% of normal opacity
    g.circle(0, 0, node.radius);
    g.fill({ color: nodeColor, alpha: 0.12 + node.impulse.weight * 0.1 });

    g.x = node.x;
    g.y = node.y;
    g.eventMode = "static";
    g.cursor = "pointer";
    g.label = node.impulse.id;

    layer.addChild(g);
  }
}

export function renderGhostConnections(
  layer: Container,
  edges: { sourceX: number; sourceY: number; targetX: number; targetY: number; weight: number }[]
) {
  const g = new Graphics();

  for (const edge of edges) {
    // Very subtle monochrome connections
    const alpha = Math.max(0.02, edge.weight * 0.05);

    g.moveTo(edge.sourceX, edge.sourceY);
    g.lineTo(edge.targetX, edge.targetY);
    g.stroke({ color: 0x000000, width: 0.5, alpha });
  }

  layer.addChild(g);
}

export function renderBridgeConnections(
  layer: Container,
  bridges: { sourceX: number; sourceY: number; targetX: number; targetY: number }[]
) {
  const g = new Graphics();

  for (const bridge of bridges) {
    // Dashed effect via short segments — very subtle
    const dx = bridge.targetX - bridge.sourceX;
    const dy = bridge.targetY - bridge.sourceY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const segments = Math.floor(dist / 8);
    const ux = dx / dist;
    const uy = dy / dist;

    for (let i = 0; i < segments; i += 2) {
      const sx = bridge.sourceX + ux * i * 8;
      const sy = bridge.sourceY + uy * i * 8;
      const ex = bridge.sourceX + ux * Math.min((i + 1) * 8, dist);
      const ey = bridge.sourceY + uy * Math.min((i + 1) * 8, dist);

      g.moveTo(sx, sy);
      g.lineTo(ex, ey);
      g.stroke({ color: 0x000000, width: 0.5, alpha: 0.04 });
    }
  }

  layer.addChild(g);
}
