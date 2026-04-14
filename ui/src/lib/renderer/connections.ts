import { Container, Graphics } from "pixi.js";
import type { GraphEdge } from "../types";

export function renderConnections(layer: Container, edges: GraphEdge[]) {
  layer.removeChildren();

  const g = new Graphics();

  for (const edge of edges) {
    const weight = edge.connection.weight;
    const alpha = Math.max(0.05, weight * 0.4);
    const width = 0.5 + weight * 1.5;

    // Parse source color for the line
    const color = edge.source.color;

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
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

    const alpha = isOnPath ? 0.8 : 0.05;
    const width = isOnPath ? 2 : 0.5;
    const color = isOnPath ? 0xfbbf24 : edge.source.color;

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
  }

  layer.addChild(g);
}
