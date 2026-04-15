import { Container, Graphics, Circle, Text, TextStyle } from "pixi.js";
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

    // Node label (hidden by default, shown on hover)
    const labelStyle = new TextStyle({
      fontFamily: "DM Sans, system-ui, sans-serif",
      fontSize: 10,
      fill: "#666666",
      wordWrap: true,
      wordWrapWidth: 120,
    });
    const label = new Text({
      text: node.impulse.content.slice(0, 40) + (node.impulse.content.length > 40 ? "..." : ""),
      style: labelStyle,
    });
    label.anchor.set(0.5, 0);
    label.x = 0;
    label.y = node.radius + 4;
    label.alpha = 0;
    label.label = "nodelabel";
    g.addChild(label);

    // Source provider badge
    const providerColors: Record<string, number> = {
      claude: 0xD97757,
      openai: 0x10A37F,
      gemini: 0x4285F4,
      import: 0x8E99A4,
      ghost: 0x7B9E87,
    };
    const prov = node.impulse.source_provider;
    if (prov && prov !== "unknown" && prov !== "") {
      const provColor = providerColors[prov] || 0x8E99A4;
      g.circle(node.radius * 0.7, -node.radius * 0.7, 2.5);
      g.fill({ color: provColor, alpha: 0.9 });
    }

    g.x = node.x;
    g.y = node.y;

    // Interactivity
    g.hitArea = new Circle(0, 0, node.radius + 8);
    g.eventMode = "static";
    g.cursor = "pointer";
    g.label = node.impulse.id;

    // Hover: show label and scale up
    g.on("pointerover", () => {
      const lbl = g.children.find((c: any) => c.label === "nodelabel");
      if (lbl) lbl.alpha = 1;
      g.scale.set(1.1);
    });
    g.on("pointerout", () => {
      const lbl = g.children.find((c: any) => c.label === "nodelabel");
      if (lbl) lbl.alpha = 0;
      g.scale.set(1.0);
    });

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
