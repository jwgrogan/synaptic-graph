// Spreading activation retrieval algorithm

use std::collections::{HashMap, HashSet};

use crate::db::Database;
use crate::models::*;
use crate::weight;

pub struct ActivationEngine<'a> {
    db: &'a Database,
}

impl<'a> ActivationEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn retrieve(&self, request: &RetrievalRequest) -> Result<RetrievalResult, String> {
        // Phase 1: Seed — find directly matching nodes via FTS
        let seed_matches = self
            .db
            .search_impulses_fts(&request.query)
            .map_err(|e| format!("FTS search failed: {}", e))?;

        // Also search ghost nodes via FTS
        let ghost_matches = self
            .db
            .search_ghost_nodes_fts(&request.query)
            .map_err(|e| format!("Ghost FTS failed: {}", e))?;

        // Build ghost activations map with same normalization as impulse matches
        let mut ghost_activations_map: HashMap<String, f64> = HashMap::new();
        for (id, rank) in &ghost_matches {
            let score = (-rank).min(1.0).max(0.1);
            ghost_activations_map.insert(id.clone(), score);
        }

        if seed_matches.is_empty() && ghost_matches.is_empty() {
            return Ok(RetrievalResult {
                memories: vec![],
                total_nodes_activated: 0,
                ghost_activations: vec![],
            });
        }

        // Initialize activation scores from seed matches
        // FTS rank is negative (closer to 0 is better), normalize to 0-1
        let mut activations: HashMap<String, f64> = HashMap::new();
        let mut activation_paths: HashMap<String, Vec<String>> = HashMap::new();

        for (id, rank) in &seed_matches {
            // FTS5 rank is negative, more negative = better match
            // Normalize: use absolute value, then scale
            let score = (-rank).min(1.0).max(0.1);
            activations.insert(id.clone(), score);
            activation_paths.insert(id.clone(), vec![id.clone()]);
        }

        // Phase 2: Propagate — spread activation through connections
        let mut traversed_connections: HashSet<String> = HashSet::new();

        for _iteration in 0..MAX_PROPAGATION_ITERATIONS {
            let mut new_activations: HashMap<String, f64> = HashMap::new();
            let mut changed = false;

            let current_nodes: Vec<(String, f64)> =
                activations.iter().map(|(k, v)| (k.clone(), *v)).collect();

            for (node_id, node_activation) in &current_nodes {
                let connections = self
                    .db
                    .get_connections_for_node(node_id)
                    .map_err(|e| format!("Failed to get connections: {}", e))?;

                for conn in &connections {
                    let neighbor_id = if conn.source_id == *node_id {
                        &conn.target_id
                    } else {
                        &conn.source_id
                    };

                    // Skip if neighbor is deleted or superseded
                    let neighbor = match self.db.get_impulse(neighbor_id) {
                        Ok(imp) => imp,
                        Err(_) => continue,
                    };

                    if neighbor.status == ImpulseStatus::Deleted
                        || neighbor.status == ImpulseStatus::Superseded
                    {
                        continue;
                    }

                    // Calculate propagated activation
                    let now = chrono::Utc::now();
                    let hours = weight::hours_since(&conn.last_traversed_at, &now);
                    let effective_conn_weight =
                        weight::effective_weight(conn.weight, hours, DECAY_SEMANTIC);

                    // Emotional amplification: high engagement reduces proximity decay
                    let engagement_factor = match neighbor.engagement_level {
                        EngagementLevel::High => 0.8,
                        EngagementLevel::Medium => 0.5,
                        EngagementLevel::Low => 0.3,
                    };

                    let propagated =
                        node_activation * effective_conn_weight * engagement_factor;

                    let current = activations.get(neighbor_id).copied().unwrap_or(0.0);
                    let new_score = new_activations
                        .get(neighbor_id)
                        .copied()
                        .unwrap_or(current);

                    if propagated > new_score - current {
                        new_activations.insert(neighbor_id.clone(), current + propagated);
                        traversed_connections.insert(conn.id.clone());

                        // Update activation path
                        let mut path = activation_paths
                            .get(node_id)
                            .cloned()
                            .unwrap_or_default();
                        path.push(neighbor_id.clone());
                        activation_paths.insert(neighbor_id.clone(), path);

                        changed = true;
                    }
                }
            }

            // Merge new activations
            for (id, score) in new_activations {
                activations.insert(id, score);
            }

            if !changed {
                break;
            }
        }

        // Phase 3: Reinforce traversed connections
        for conn_id in &traversed_connections {
            if let Ok(conn) = self.db.get_connection(conn_id) {
                let new_weight = weight::reinforce(conn.weight);
                let _ = self.db.update_connection_weight(conn_id, new_weight);
                let _ = self.db.touch_connection(conn_id);
            }
        }

        // Touch accessed impulses
        for node_id in activations.keys() {
            let _ = self.db.touch_impulse(node_id);
        }

        // Phase 4: Assemble results
        let mut results: Vec<RetrievedMemory> = Vec::new();

        for (id, score) in &activations {
            if *score < ACTIVATION_THRESHOLD {
                continue;
            }

            let impulse = match self.db.get_impulse(id) {
                Ok(imp) => imp,
                Err(_) => continue,
            };

            if impulse.status == ImpulseStatus::Deleted
                || impulse.status == ImpulseStatus::Superseded
            {
                continue;
            }

            let path = activation_paths.get(id).cloned().unwrap_or_default();

            results.push(RetrievedMemory {
                impulse,
                activation_score: *score,
                activation_path: path,
            });
        }

        results.sort_by(|a, b| {
            b.activation_score
                .partial_cmp(&a.activation_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(request.max_results);

        let total_activated = activations.len();

        // Build ghost activations from ghost FTS matches
        let mut ghost_activations: Vec<GhostActivation> = Vec::new();
        for (id, score) in &ghost_activations_map {
            if *score < ACTIVATION_THRESHOLD {
                continue;
            }
            if let Ok(ghost_node) = self.db.get_ghost_node(id) {
                let source_graph = ghost_node.source_graph.clone();
                ghost_activations.push(GhostActivation {
                    ghost_node,
                    activation_score: *score,
                    source_graph,
                });
            }
        }
        ghost_activations.sort_by(|a, b| {
            b.activation_score
                .partial_cmp(&a.activation_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(RetrievalResult {
            memories: results,
            total_nodes_activated: total_activated,
            ghost_activations,
        })
    }
}
