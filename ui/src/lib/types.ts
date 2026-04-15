export interface Impulse {
  id: string;
  content: string;
  impulse_type: string;
  weight: number;
  initial_weight: number;
  emotional_valence: string;
  engagement_level: string;
  source_type: string;
  source_ref: string;
  status: string;
  created_at: string;
  last_accessed_at: string;
}

export interface Connection {
  id: string;
  source_id: string;
  target_id: string;
  weight: number;
  relationship: string;
  traversal_count: number;
}

export interface GraphNode {
  impulse: Impulse;
  x: number;
  y: number;
  vx: number;
  vy: number;
  cluster: number;
  color: string;
  radius: number;
  isGhost: boolean;
}

export interface GraphEdge {
  connection: Connection;
  source: GraphNode;
  target: GraphNode;
}

export interface GhostSource {
  name: string;
  root_path: string;
  source_type: string;
  node_count: number;
  last_scanned_at: string | null;
}

export interface SearchResult {
  memories: {
    id: string;
    content: string;
    activation_score: number;
    activation_path: string[];
  }[];
  ghost_activations: {
    ghost_node_id: string;
    title: string;
    source_graph: string;
    activation_score: number;
  }[];
  total_activated: number;
}

export interface MemoryStats {
  total_impulses: number;
  confirmed_impulses: number;
  candidate_impulses: number;
  total_connections: number;
}

export interface ImpulseDetail {
  impulse: Impulse;
  connections: {
    id: string;
    other_id: string;
    other_content: string;
    relationship: string;
    weight: number;
    traversal_count: number;
  }[];
}

export type ZoomLevel = "galaxy" | "cluster" | "node";

export const NEBULA_COLORS = [
  "#5C6BC0", // muted indigo
  "#7B9E87", // sage
  "#C17F59", // warm amber
  "#C4727F", // dusty rose
  "#8E99A4", // steel
  "#9B8EC4", // lavender
  "#6B9DAD", // teal
  "#B8956B", // caramel
  "#7BAD8E", // mint
  "#AD7B8E", // mauve
] as const;
