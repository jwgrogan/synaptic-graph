import { Container, Graphics } from "pixi.js";
import type { GraphEdge } from "../types";

export function renderConnections(layer: Container, edges: GraphEdge[]) {
  layer.removeChildren();
  const g = new Graphics();

  for (const edge of edges) {
    const weight = edge.connection.weight;
    const alpha = 0.08 + weight * 0.12;
    const width = 0.5 + weight * 1.0;

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color: 0x888888, width, alpha });
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

    const alpha = isOnPath ? 0.6 : 0.04;
    const width = isOnPath ? 2 : 0.5;
    const color = isOnPath ? 0x5C6BC0 : 0x888888;

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
  }

  layer.addChild(g);
}
