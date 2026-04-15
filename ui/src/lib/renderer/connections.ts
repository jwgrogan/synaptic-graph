import { Container, Graphics } from "pixi.js";
import type { GraphEdge } from "../types";

export function renderConnections(layer: Container, edges: GraphEdge[]) {
  layer.removeChildren();

  const g = new Graphics();

  for (const edge of edges) {
    const weight = edge.connection.weight;
    const alpha = Math.max(0.08, weight * 0.5);
    const width = 1 + weight * 2; // 1 to 3px

    // Color based on weight: sage for strong, sand for moderate, very light for weak
    let color: number;
    if (weight > 0.5) {
      color = 0xA8B5A0; // sage
    } else if (weight > 0.25) {
      color = 0xD4C5A9; // sand
    } else {
      color = 0xEDE8E1; // very light
    }

    // Draw curved connection using quadratic bezier
    const mx = (edge.source.x + edge.target.x) / 2;
    const my = (edge.source.y + edge.target.y) / 2;
    // Offset control point perpendicular to the line
    const dx = edge.target.x - edge.source.x;
    const dy = edge.target.y - edge.source.y;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const curvature = Math.min(dist * 0.15, 30);
    // Use perpendicular direction for control point offset
    const cpX = mx + (-dy / dist) * curvature;
    const cpY = my + (dx / dist) * curvature;

    g.moveTo(edge.source.x, edge.source.y);

    // Synaptic gap: stop the line slightly before the target node
    const gapFraction = 0.92;
    // Calculate the point at gapFraction along the bezier
    const t = gapFraction;
    const endX = (1-t)*(1-t)*edge.source.x + 2*(1-t)*t*cpX + t*t*edge.target.x;
    const endY = (1-t)*(1-t)*edge.source.y + 2*(1-t)*t*cpY + t*t*edge.target.y;

    g.quadraticCurveTo(cpX, cpY, endX, endY);
    g.stroke({ color, width, alpha });

    // Narrow gap segment near target (thinner width to suggest synaptic gap)
    const gapStart = 0.93;
    const gapEnd = 0.98;
    const gsX = (1-gapStart)*(1-gapStart)*edge.source.x + 2*(1-gapStart)*gapStart*cpX + gapStart*gapStart*edge.target.x;
    const gsY = (1-gapStart)*(1-gapStart)*edge.source.y + 2*(1-gapStart)*gapStart*cpY + gapStart*gapStart*edge.target.y;
    const geX = (1-gapEnd)*(1-gapEnd)*edge.source.x + 2*(1-gapEnd)*gapEnd*cpX + gapEnd*gapEnd*edge.target.x;
    const geY = (1-gapEnd)*(1-gapEnd)*edge.source.y + 2*(1-gapEnd)*gapEnd*cpY + gapEnd*gapEnd*edge.target.y;

    g.moveTo(gsX, gsY);
    g.lineTo(geX, geY);
    g.stroke({ color, width: Math.max(0.5, width * 0.4), alpha: alpha * 0.5 });
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
    const width = isOnPath ? 2.5 : 0.5;
    const color = isOnPath ? 0xA67B8A : 0xEDE8E1; // mauve-deep for path, light for background

    // Curved connection
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
