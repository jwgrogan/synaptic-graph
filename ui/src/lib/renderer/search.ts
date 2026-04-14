import type { GraphEdge } from "../types";
import { highlightPath } from "./connections";
import { highlightNode } from "./nodes";
import type { Container } from "pixi.js";

export function applySearchHighlight(
  connectionLayer: Container,
  nodeLayer: Container,
  edges: GraphEdge[],
  activatedNodeIds: Set<string>
) {
  highlightPath(connectionLayer, edges, activatedNodeIds);

  for (const child of nodeLayer.children) {
    if (child.label) {
      const isActivated = activatedNodeIds.has(child.label as string);
      child.alpha = isActivated ? 1.0 : 0.15;
      child.scale.set(isActivated ? 1.2 : 0.8);
    }
  }
}

export function clearSearchHighlight(
  connectionLayer: Container,
  nodeLayer: Container,
  edges: GraphEdge[]
) {
  highlightPath(connectionLayer, edges, new Set());

  for (const child of nodeLayer.children) {
    child.alpha = 1.0;
    child.scale.set(1.0);
  }
}
