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

    // Outer glow
    const glowAlpha = node.impulse.engagement_level === "high" ? 0.3 : 0.15;
    const glowRadius = node.radius * 3;
    g.circle(0, 0, glowRadius);
    g.fill({ color: node.color, alpha: glowAlpha });

    // Core star
    const coreAlpha = 0.5 + node.impulse.weight * 0.5;
    g.circle(0, 0, node.radius);
    g.fill({ color: node.color, alpha: coreAlpha });

    // Bright center
    g.circle(0, 0, node.radius * 0.4);
    g.fill({ color: 0xffffff, alpha: 0.6 + node.impulse.weight * 0.4 });

    g.x = node.x;
    g.y = node.y;

    // Hit area and interactivity
    g.hitArea = new Circle(0, 0, glowRadius);
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
