import { Container, Graphics } from "pixi.js";
import type { Cluster } from "./clusters";
import { NEBULA_COLORS } from "../types";

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
    const radius = Math.max((Math.max(maxX - minX, maxY - minY)) / 2, 30) + 50;

    const g = new Graphics();

    // Get cluster color
    const colorHex = NEBULA_COLORS[cluster.id % NEBULA_COLORS.length];
    const cr = parseInt(colorHex.slice(1, 3), 16);
    const cg = parseInt(colorHex.slice(3, 5), 16);
    const cb = parseInt(colorHex.slice(5, 7), 16);
    const color = (cr << 16) | (cg << 8) | cb;

    // Single circle per cluster, very subtle — alpha 0.02 to 0.04
    const alpha = 0.02 + Math.min(cluster.nodeIds.size / 50, 0.02);
    g.circle(cx, cy, radius);
    g.fill({ color, alpha });

    layer.addChild(g);
  }
}
