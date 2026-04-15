import { Container, Graphics } from "pixi.js";
import type { Cluster } from "./clusters";

export function renderNebulae(
  layer: Container,
  clusters: Cluster[],
  nodePositions: Map<string, { x: number; y: number }>
) {
  layer.removeChildren();

  for (const cluster of clusters) {
    // Only render nebula for clusters with 2+ nodes
    if (cluster.nodeIds.length < 2) continue;

    // Calculate cluster center and radius from node positions
    const positions: { x: number; y: number }[] = [];
    for (const id of cluster.nodeIds) {
      const pos = nodePositions.get(id);
      if (pos) positions.push(pos);
    }

    if (positions.length < 2) continue;

    const cx = positions.reduce((s, p) => s + p.x, 0) / positions.length;
    const cy = positions.reduce((s, p) => s + p.y, 0) / positions.length;

    // Compute radius as max distance from center to any node
    let maxDist = 0;
    for (const p of positions) {
      const dx = p.x - cx;
      const dy = p.y - cy;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist > maxDist) maxDist = dist;
    }

    // Add padding so the nebula extends beyond the outermost nodes
    const baseRadius = maxDist + 40;

    // Parse cluster color
    const hex = cluster.color || "#818cf8";
    const color = parseInt(hex.replace("#", ""), 16);

    const g = new Graphics();

    // 4 concentric layers at decreasing alpha for nebula cloud effect
    const layers: [number, number][] = [
      [1.5, 0.03],
      [1.0, 0.06],
      [0.6, 0.10],
      [0.3, 0.15],
    ];

    for (const [radiusMul, alpha] of layers) {
      g.circle(cx, cy, baseRadius * radiusMul);
      g.fill({ color, alpha });
    }

    layer.addChild(g);
  }
}
