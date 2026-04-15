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

    // Parse cluster color
    const hex = node.color || "#8E99A4";
    const color = parseInt(hex.replace("#", ""), 16);

    // Simple filled circle — that's it
    g.circle(0, 0, node.radius);
    g.fill({ color, alpha: 0.75 });

    // Subtle lighter ring on hover-ready nodes
    g.circle(0, 0, node.radius + 1);
    g.stroke({ color, width: 0.5, alpha: 0.2 });

    g.x = node.x;
    g.y = node.y;

    // Interactivity
    g.hitArea = new Circle(0, 0, node.radius + 8);
    g.eventMode = "static";
    g.cursor = "pointer";
    g.label = node.impulse.id;

    if (onClick) {
      g.on("pointerdown", () => onClick(node.impulse.id));
    }

    layer.addChild(g);
  }
}

export function updateNodePositions(layer: Container, nodes: GraphNode[]) {
  const children = layer.children;
  for (let i = 0; i < children.length && i < nodes.length; i++) {
    children[i].x = nodes[i].x;
    children[i].y = nodes[i].y;
  }
}

export function updateNodes(layer: Container, nodes: GraphNode[]) {
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
