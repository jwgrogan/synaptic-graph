import { Container } from "pixi.js";
import type { Cluster } from "./clusters";

// Clean graph view — no cluster background effects
export function renderNebulae(
  _layer: Container,
  _clusters: Cluster[],
  _nodePositions: Map<string, { x: number; y: number }>
) {
  // Intentionally empty
}
