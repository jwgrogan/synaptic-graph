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
    const radius = Math.max((maxX - minX) / 2, (maxY - minY) / 2) + 40;

    const g = new Graphics();
    const color = cluster.color;

    // Multiple concentric circles for glow effect
    g.circle(cx, cy, radius * 1.5);
    g.fill({ color, alpha: 0.03 });

    g.circle(cx, cy, radius * 1.0);
    g.fill({ color, alpha: 0.06 });

    g.circle(cx, cy, radius * 0.6);
    g.fill({ color, alpha: 0.1 });

    g.circle(cx, cy, radius * 0.3);
    g.fill({ color, alpha: 0.15 });

    layer.addChild(g);
  }
}
