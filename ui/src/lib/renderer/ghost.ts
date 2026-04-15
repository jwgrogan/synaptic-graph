import { Container, Graphics } from "pixi.js";
import type { GraphNode } from "../types";

export function renderGhostNodes(layer: Container, ghostNodes: GraphNode[]) {
  for (const node of ghostNodes) {
    const g = new Graphics();

    // Ghost outer halo — very light sage, faint
    g.circle(0, 0, node.radius * 2.5);
    g.fill({ color: 0xA8B5A0, alpha: 0.06 });

    // Ghost core — light sage, semi-transparent
    g.circle(0, 0, node.radius);
    g.fill({ color: 0xA8B5A0, alpha: 0.15 + node.impulse.weight * 0.2 });

    // Tiny bright center for pulled-through nodes
    if (node.impulse.weight > 0.4) {
      g.circle(0, 0, node.radius * 0.3);
      g.fill({ color: 0xC8D5C0, alpha: 0.4 });
    }

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
    const alpha = Math.max(0.03, edge.weight * 0.15);

    // Dashed line for ghost connections (olive color)
    const dx = edge.targetX - edge.sourceX;
    const dy = edge.targetY - edge.sourceY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const segments = Math.floor(dist / 6);
    const ux = dx / dist;
    const uy = dy / dist;

    for (let i = 0; i < segments; i += 2) {
      const sx = edge.sourceX + ux * i * 6;
      const sy = edge.sourceY + uy * i * 6;
      const ex = edge.sourceX + ux * Math.min((i + 1) * 6, dist);
      const ey = edge.sourceY + uy * Math.min((i + 1) * 6, dist);

      g.moveTo(sx, sy);
      g.lineTo(ex, ey);
      g.stroke({ color: 0x8B9B6B, width: 0.5, alpha });
    }
  }

  layer.addChild(g);
}

export function renderBridgeConnections(
  layer: Container,
  bridges: { sourceX: number; sourceY: number; targetX: number; targetY: number }[]
) {
  const g = new Graphics();

  for (const bridge of bridges) {
    // Dashed effect via short segments — olive color
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
      g.stroke({ color: 0x8B9B6B, width: 0.5, alpha: 0.12 });
    }
  }

  layer.addChild(g);
}
