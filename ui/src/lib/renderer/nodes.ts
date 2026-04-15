import { Container, Graphics, Circle } from "pixi.js";
import type { GraphNode } from "../types";

export function renderNodes(
  layer: Container,
  nodes: GraphNode[],
  onClick?: (nodeId: string) => void
) {
  layer.removeChildren();

  for (const node of nodes) {
    const g = new Graphics();
    const weight = node.impulse.weight;

    // Parse cluster color to get RGB values
    const colorHex = node.color || "#8E99A4";
    const cr = parseInt(colorHex.slice(1, 3), 16);
    const cg = parseInt(colorHex.slice(3, 5), 16);
    const cb = parseInt(colorHex.slice(5, 7), 16);
    const nodeColor = (cr << 16) | (cg << 8) | cb;

    const radius = node.radius;

    // Subtle drop shadow (darker circle offset slightly below)
    g.circle(0, 1.5, radius);
    g.fill({ color: 0x000000, alpha: 0.06 });

    // Main node circle — clean filled circle with muted cluster color
    g.circle(0, 0, radius);
    g.fill({ color: nodeColor, alpha: 0.35 + weight * 0.35 });

    // Bright center dot for high-weight nodes only
    if (weight > 0.6) {
      g.circle(0, 0, radius * 0.25);
      g.fill({ color: 0xFFFFFF, alpha: 0.4 + (weight - 0.6) * 0.5 });
    }

    g.x = node.x;
    g.y = node.y;

    // Hit area and interactivity
    const hitRadius = radius * 2.2;
    g.hitArea = new Circle(0, 0, hitRadius);
    g.eventMode = "static";
    g.cursor = "pointer";
    g.label = node.impulse.id;

    if (onClick) {
      g.on("pointerdown", () => {
        onClick(node.impulse.id);
      });
    }

    layer.addChild(g);
  }
}

export function updateNodes(layer: Container, nodes: GraphNode[]) {
  // For now, just re-render. Optimize later with sprite pooling.
  renderNodes(layer, nodes);
}

export function highlightNode(layer: Container, nodeId: string, highlight: boolean) {
  for (const child of layer.children) {
    if (child.label === nodeId) {
      child.alpha = highlight ? 1.0 : 0.7;
      child.scale.set(highlight ? 1.3 : 1.0);
    }
  }
}
