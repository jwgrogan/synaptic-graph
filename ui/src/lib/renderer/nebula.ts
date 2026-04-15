import { Container, Graphics } from "pixi.js";
import type { Cluster } from "./clusters";

export function renderNebulae(
  layer: Container,
  clusters: Cluster[],
  nodePositions: Map<string, { x: number; y: number }>
) {
  layer.removeChildren();

  for (const cluster of clusters) {
    if (cluster.nodeIds.size < 2) continue;

    // Calculate cluster bounds
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;

    for (const nodeId of cluster.nodeIds) {
      const pos = nodePositions.get(nodeId);
      if (pos) {
        minX = Math.min(minX, pos.x);
        maxX = Math.max(maxX, pos.x);
        minY = Math.min(minY, pos.y);
        maxY = Math.max(maxY, pos.y);
      }
    }

    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    const radiusX = Math.max((maxX - minX) / 2, 30) + 40;
    const radiusY = Math.max((maxY - minY) / 2, 30) + 35;

    const g = new Graphics();

    // Organic blob shape — use cream/sand tones with very low alpha
    // Multiple overlapping ellipses for an organic feel
    g.ellipse(cx, cy, radiusX * 1.4, radiusY * 1.3);
    g.fill({ color: 0xF0EAE0, alpha: 0.04 }); // cream, very faint

    g.ellipse(cx - radiusX * 0.1, cy + radiusY * 0.05, radiusX * 1.0, radiusY * 0.9);
    g.fill({ color: 0xD4C5A9, alpha: 0.05 }); // sand

    g.ellipse(cx + radiusX * 0.05, cy - radiusY * 0.08, radiusX * 0.7, radiusY * 0.65);
    g.fill({ color: 0xF0EAE0, alpha: 0.06 }); // cream, slightly more

    g.ellipse(cx, cy, radiusX * 0.35, radiusY * 0.3);
    g.fill({ color: 0xD4C5A9, alpha: 0.08 }); // sand core

    layer.addChild(g);
  }
}
