import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from "d3-force";
import type { Impulse, Connection, GraphNode, GraphEdge } from "../types";
import { detectClusters, getClusterColor } from "./clusters";

interface SimNode extends SimulationNodeDatum {
  id: string;
  impulse: Impulse;
  cluster: number;
  color: string;
  radius: number;
  isGhost: boolean;
}

interface SimLink extends SimulationLinkDatum<SimNode> {
  connection: Connection;
}

export function computeLayout(
  impulses: Impulse[],
  connections: Connection[]
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodeToCluster = detectClusters(impulses, connections);

  const simNodes: SimNode[] = impulses.map((impulse) => {
    const cluster = nodeToCluster.get(impulse.id) ?? 0;
    const weight = impulse.weight;
    // Node radius: 3 to 12 based on weight
    const radius = 3 + weight * 9;

    return {
      id: impulse.id,
      impulse,
      cluster,
      color: getClusterColor(cluster),
      radius,
      isGhost: false,
      x: undefined,
      y: undefined,
    };
  });

  const nodeMap = new Map(simNodes.map((n) => [n.id, n]));

  const simLinks: SimLink[] = connections
    .filter(
      (c) => nodeMap.has(c.source_id) && nodeMap.has(c.target_id)
    )
    .map((c) => ({
      source: c.source_id,
      target: c.target_id,
      connection: c,
    }));

  // Run force simulation
  const simulation = forceSimulation<SimNode>(simNodes)
    .force(
      "link",
      forceLink<SimNode, SimLink>(simLinks)
        .id((d) => d.id)
        .distance(80)
        .strength((d) => d.connection.weight * 0.5)
    )
    .force("charge", forceManyBody().strength(-120))
    .force("center", forceCenter(0, 0))
    .force("collide", forceCollide<SimNode>().radius((d) => d.radius + 2))
    .stop();

  // Run synchronously for 300 ticks
  for (let i = 0; i < 300; i++) {
    simulation.tick();
  }

  // Convert to GraphNode/GraphEdge
  const graphNodes: GraphNode[] = simNodes.map((n) => ({
    impulse: n.impulse,
    x: n.x ?? 0,
    y: n.y ?? 0,
    vx: 0,
    vy: 0,
    cluster: n.cluster,
    color: n.color,
    radius: n.radius,
    isGhost: false,
  }));

  const graphNodeMap = new Map(graphNodes.map((n) => [n.impulse.id, n]));

  const graphEdges: GraphEdge[] = simLinks
    .map((l) => {
      const sourceNode = graphNodeMap.get(
        typeof l.source === "string" ? l.source : (l.source as SimNode).id
      );
      const targetNode = graphNodeMap.get(
        typeof l.target === "string" ? l.target : (l.target as SimNode).id
      );
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
