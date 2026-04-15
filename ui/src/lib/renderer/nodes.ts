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

    // Neuron soma — slightly irregular filled ellipse in mauve tones
    // Core color: high weight = deep mauve (#A67B8A), low weight = light sand (#D4C5A9)
    const deepMauve = { r: 166, g: 123, b: 138 };
    const lightSand = { r: 212, g: 197, b: 169 };
    const r = Math.round(lightSand.r + (deepMauve.r - lightSand.r) * weight);
    const gC = Math.round(lightSand.g + (deepMauve.g - lightSand.g) * weight);
    const b = Math.round(lightSand.b + (deepMauve.b - lightSand.b) * weight);
    const somaColor = (r << 16) | (gC << 8) | b;

    const radius = node.radius;

    // Soft outer halo (warm cream)
    g.ellipse(0, 0, radius * 2.2, radius * 1.9);
    g.fill({ color: 0xF0EAE0, alpha: 0.12 });

    // Soma body — slightly irregular via ellipse
    g.ellipse(0, 0, radius * 1.1, radius * 0.95);
    g.fill({ color: somaColor, alpha: 0.5 + weight * 0.4 });

    // Inner bright spot
    g.ellipse(0, 0, radius * 0.4, radius * 0.35);
    g.fill({ color: 0xFAFAF7, alpha: 0.3 + weight * 0.3 });

    // Dendrite stubs — 3-5 short curved lines radiating outward
    const dendriteCount = 3 + Math.floor(weight * 2);
    const dendriteAlpha = 0.2 + weight * 0.3;
    for (let i = 0; i < dendriteCount; i++) {
      const angle = (i / dendriteCount) * Math.PI * 2 + (node.impulse.id.charCodeAt(0) * 0.5);
      const startX = Math.cos(angle) * radius * 0.9;
      const startY = Math.sin(angle) * radius * 0.8;
      const endX = Math.cos(angle) * radius * 2.0;
      const endY = Math.sin(angle) * radius * 1.8;
      // Control point offset for curve
      const cpX = (startX + endX) / 2 + Math.sin(angle) * radius * 0.4;
      const cpY = (startY + endY) / 2 - Math.cos(angle) * radius * 0.4;

      g.moveTo(startX, startY);
      g.quadraticCurveTo(cpX, cpY, endX, endY);
      g.stroke({ color: somaColor, width: 1, alpha: dendriteAlpha });
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
