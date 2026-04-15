import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  type Simulation,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from "d3-force";
import type { Impulse, Connection, GraphNode, GraphEdge } from "../types";
import { detectClusters, getClusterColor } from "./clusters";

export interface SimNode extends SimulationNodeDatum {
  id: string;
  impulse: Impulse;
  cluster: number;
  color: string;
  radius: number;
  isGhost: boolean;
  connectionCount: number;
}

export interface SimLink extends SimulationLinkDatum<SimNode> {
  connection: Connection;
}

/**
 * Create a LIVE force simulation that keeps running.
 * Returns the simulation, nodes, and links — caller is responsible
 * for reading positions each frame and re-rendering.
 */
export function createSimulation(
  impulses: Impulse[],
  connections: Connection[],
  width: number,
  height: number,
): {
  simulation: Simulation<SimNode, SimLink>;
  simNodes: SimNode[];
  simLinks: SimLink[];
} {
  const nodeToCluster = detectClusters(impulses, connections);

  // Count connections per node for sizing
  const connCounts = new Map<string, number>();
  for (const c of connections) {
    connCounts.set(c.source_id, (connCounts.get(c.source_id) || 0) + 1);
    connCounts.set(c.target_id, (connCounts.get(c.target_id) || 0) + 1);
  }

  const simNodes: SimNode[] = impulses.map((impulse) => {
    const cluster = nodeToCluster.get(impulse.id) ?? 0;
    const count = connCounts.get(impulse.id) || 0;
    // Size based on connection count: min 4, scales up with connections
    const radius = 4 + Math.min(count * 2, 16);

    return {
      id: impulse.id,
      impulse,
      cluster,
      color: getClusterColor(cluster),
      radius,
      isGhost: false,
      connectionCount: count,
      x: (Math.random() - 0.5) * width * 0.5 + width / 2,
      y: (Math.random() - 0.5) * height * 0.5 + height / 2,
    };
  });

  const nodeMap = new Map(simNodes.map((n) => [n.id, n]));

  const simLinks: SimLink[] = connections
    .filter((c) => nodeMap.has(c.source_id) && nodeMap.has(c.target_id))
    .map((c) => ({
      source: c.source_id,
      target: c.target_id,
      connection: c,
    }));

  const simulation = forceSimulation<SimNode>(simNodes)
    .force(
      "link",
      forceLink<SimNode, SimLink>(simLinks)
        .id((d) => d.id)
        .distance(100)
        .strength((d) => 0.3 + d.connection.weight * 0.4)
    )
    .force("charge", forceManyBody<SimNode>().strength(-200))
    .force("center", forceCenter(width / 2, height / 2).strength(0.05))
    .force("collide", forceCollide<SimNode>().radius((d) => d.radius + 4).strength(0.7))
    .alphaDecay(0.01)  // slow decay = longer settling, more organic movement
    .velocityDecay(0.3);  // some drag for smooth movement

  return { simulation, simNodes, simLinks };
}

/**
 * Convert live simulation state to GraphNode/GraphEdge for rendering.
 */
export function snapshotGraph(
  simNodes: SimNode[],
  simLinks: SimLink[],
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const graphNodes: GraphNode[] = simNodes.map((n) => ({
    impulse: n.impulse,
    x: n.x ?? 0,
    y: n.y ?? 0,
    vx: n.vx ?? 0,
    vy: n.vy ?? 0,
    cluster: n.cluster,
    color: n.color,
    radius: n.radius,
    isGhost: n.isGhost,
  }));

  const graphNodeMap = new Map(graphNodes.map((n) => [n.impulse.id, n]));

  const graphEdges: GraphEdge[] = simLinks
    .map((l) => {
      const sourceId = typeof l.source === "string" ? l.source : (l.source as SimNode).id;
      const targetId = typeof l.target === "string" ? l.target : (l.target as SimNode).id;
      const sourceNode = graphNodeMap.get(sourceId);
      const targetNode = graphNodeMap.get(targetId);
      if (!sourceNode || !targetNode) return null;
      return {
        connection: l.connection,
        source: sourceNode,
        target: targetNode,
      };
    })
    .filter((e): e is GraphEdge => e !== null);

  return { nodes: graphNodes, edges: graphEdges };
}
