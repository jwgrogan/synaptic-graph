import { Container, Graphics } from "pixi.js";
import type { GraphEdge } from "../types";

export function renderConnections(layer: Container, edges: GraphEdge[]) {
  layer.removeChildren();

  const g = new Graphics();

  for (const edge of edges) {
    const weight = edge.connection.weight;
    // Monochrome: rgba(0,0,0,0.06) for weak, rgba(0,0,0,0.15) for strong
    const alpha = 0.06 + weight * 0.09;
    // Width: 0.5 to 1.5px
    const width = 0.5 + weight * 1.0;

    // Draw curved connection using quadratic bezier
    const mx = (edge.source.x + edge.target.x) / 2;
    const my = (edge.source.y + edge.target.y) / 2;
    const dx = edge.target.x - edge.source.x;
    const dy = edge.target.y - edge.source.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curvature = Math.min(dist * 0.15, 30);
    const cpX = mx + (-dy / dist) * curvature;
    const cpY = my + (dx / dist) * curvature;

    g.moveTo(edge.source.x, edge.source.y);
    g.quadraticCurveTo(cpX, cpY, edge.target.x, edge.target.y);
    g.stroke({ color: 0x000000, width, alpha });
  }

  layer.addChild(g);
}

export function updateConnections(layer: Container, edges: GraphEdge[]) {
  renderConnections(layer, edges);
}

export function highlightPath(
  layer: Container,
  edges: GraphEdge[],
  pathNodeIds: Set<string>
) {
  layer.removeChildren();
  const g = new Graphics();

  for (const edge of edges) {
    const isOnPath =
      pathNodeIds.has(edge.source.impulse.id) &&
      pathNodeIds.has(edge.target.impulse.id);

    const alpha = isOnPath ? 0.7 : 0.03;
    const width = isOnPath ? 2 : 0.5;
    // accent-primary (#5C6BC0) for highlighted path
    const color = isOnPath ? 0x5C6BC0 : 0x000000;

    const mx = (edge.source.x + edge.target.x) / 2;
    const my = (edge.source.y + edge.target.y) / 2;
    const dx = edge.target.x - edge.source.x;
    const dy = edge.target.y - edge.source.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curvature = Math.min(dist * 0.15, 30);
    const cpX = mx + (-dy / dist) * curvature;
    const cpY = my + (dx / dist) * curvature;

    g.moveTo(edge.source.x, edge.source.y);
    g.quadraticCurveTo(cpX, cpY, edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
  }

  layer.addChild(g);
}
