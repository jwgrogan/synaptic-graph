import { Container, Graphics } from "pixi.js";
import type { GraphEdge } from "../types";

export function renderConnections(layer: Container, edges: GraphEdge[]) {
  layer.removeChildren();
  const g = new Graphics();

  for (const edge of edges) {
    const weight = edge.connection.weight;
    const alpha = 0.05 + weight * 0.25;
    const width = 0.5 + weight * 1.5;

    // Use source node's cluster color
    const hex = edge.source.color || "#818cf8";
    const color = parseInt(hex.replace("#", ""), 16);

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

    const alpha = isOnPath ? 0.6 : 0.03;
    const width = isOnPath ? 2 : 0.5;
    // Highlight path in amber (accent-warm), dim edges use source color
    const color = isOnPath ? 0xfbbf24 : parseInt((edge.source.color || "#818cf8").replace("#", ""), 16);

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
  }

  layer.addChild(g);
}
