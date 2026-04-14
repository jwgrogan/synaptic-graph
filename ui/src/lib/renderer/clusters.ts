import type { Impulse, Connection } from "../types";
import { NEBULA_COLORS } from "../types";

export interface Cluster {
  id: number;
  nodeIds: Set<string>;
  color: string;
  cx: number;
  cy: number;
}

/**
 * Detect clusters using connected components on strongly-weighted connections.
 * Threshold of 0.3 means only connections with weight >= 0.3 form cluster bonds.
 */
export function detectClusters(
  impulses: Impulse[],
  connections: Connection[],
  threshold = 0.3
): Map<string, number> {
  const nodeToCluster = new Map<string, number>();
  const adjacency = new Map<string, Set<string>>();

  // Build adjacency for strong connections only
  for (const impulse of impulses) {
    adjacency.set(impulse.id, new Set());
  }

  for (const conn of connections) {
    if (conn.weight >= threshold) {
      adjacency.get(conn.source_id)?.add(conn.target_id);
      adjacency.get(conn.target_id)?.add(conn.source_id);
    }
  }

  // BFS to find connected components
  let clusterId = 0;
  const visited = new Set<string>();

  for (const impulse of impulses) {
    if (visited.has(impulse.id)) continue;

    const queue = [impulse.id];
    visited.add(impulse.id);

    while (queue.length > 0) {
      const current = queue.shift()!;
      nodeToCluster.set(current, clusterId);

      for (const neighbor of adjacency.get(current) ?? []) {
        if (!visited.has(neighbor)) {
          visited.add(neighbor);
          queue.push(neighbor);
        }
      }
    }

    clusterId++;
  }

  return nodeToCluster;
}

export function getClusterColor(clusterId: number): string {
  return NEBULA_COLORS[clusterId % NEBULA_COLORS.length];
}

export function buildClusterInfo(
  nodeToCluster: Map<string, number>,
  positions: Map<string, { x: number; y: number }>
): Cluster[] {
  const clusters = new Map<number, Cluster>();

  for (const [nodeId, clusterId] of nodeToCluster) {
    if (!clusters.has(clusterId)) {
      clusters.set(clusterId, {
        id: clusterId,
        nodeIds: new Set(),
        color: getClusterColor(clusterId),
        cx: 0,
        cy: 0,
      });
    }
    clusters.get(clusterId)!.nodeIds.add(nodeId);
  }

  // Calculate cluster centers
  for (const cluster of clusters.values()) {
    let sumX = 0;
    let sumY = 0;
    let count = 0;
    for (const nodeId of cluster.nodeIds) {
      const pos = positions.get(nodeId);
      if (pos) {
        sumX += pos.x;
        sumY += pos.y;
        count++;
      }
    }
    if (count > 0) {
      cluster.cx = sumX / count;
      cluster.cy = sumY / count;
    }
  }

  return Array.from(clusters.values());
}
