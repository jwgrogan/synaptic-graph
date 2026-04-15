import { writable, derived } from "svelte/store";
import type {
  GraphNode,
  GraphEdge,
  Impulse,
  Connection,
  ZoomLevel,
  ImpulseDetail,
  SearchResult,
} from "./types";

// Raw data from backend
export const impulses = writable<Impulse[]>([]);
export const connections = writable<Connection[]>([]);

// Graph state
export const nodes = writable<GraphNode[]>([]);
export const edges = writable<GraphEdge[]>([]);

// UI state
export const zoomLevel = writable<ZoomLevel>("galaxy");
export const selectedNodeId = writable<string | null>(null);
export const selectedDetail = writable<ImpulseDetail | null>(null);
export const focusedCluster = writable<number | null>(null);
export const searchOpen = writable(false);
export const searchResults = writable<SearchResult | null>(null);
export const activationPath = writable<Set<string>>(new Set());
export const currentView = writable<"galaxy" | "ghosts" | "fading" | "stats" | "import">("galaxy");

// Camera state
export const camera = writable({
  x: 0,
  y: 0,
  zoom: 1,
});

// Derived
export const nodeCount = derived(nodes, ($nodes) => $nodes.length);
export const edgeCount = derived(edges, ($edges) => $edges.length);
