import { Container, Graphics } from "pixi.js";
import type { GraphNode } from "../types";

export function renderGhostNodes(layer: Container, ghostNodes: GraphNode[]) {
  for (const node of ghostNodes) {
    const g = new Graphics();

    // Ethereal outer glow — very faint
    g.circle(0, 0, node.radius * 2.5);
    g.fill({ color: 0x67e8f9, alpha: 0.06 });

    // Ghost core — translucent
    g.circle(0, 0, node.radius);
    g.fill({ color: 0x67e8f9, alpha: 0.2 + node.impulse.weight * 0.2 });

    // Tiny bright center for pulled-through nodes
    if (node.impulse.weight > 0.4) {
      g.circle(0, 0, node.radius * 0.3);
      g.fill({ color: 0xa5f3fc, alpha: 0.5 });
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
    g.moveTo(edge.sourceX, edge.sourceY);
    g.lineTo(edge.targetX, edge.targetY);
    g.stroke({ color: 0x67e8f9, width: 0.5, alpha });
  }

  layer.addChild(g);
}

export function renderBridgeConnections(
  layer: Container,
  bridges: { sourceX: number; sourceY: number; targetX: number; targetY: number }[]
) {
  const g = new Graphics();

  for (const bridge of bridges) {
    // Dashed effect via short segments
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
      g.stroke({ color: 0x67e8f9, width: 0.5, alpha: 0.12 });
    }
  }

  layer.addChild(g);
}
