//! Node layout algorithms for dependency graphs.
//!
//! Implements Sugiyama-style hierarchical layout:
//! 1. Layer assignment (provided by caller via depth)
//! 2. Crossing minimization (barycenter heuristic)
//! 3. Coordinate assignment (median alignment with proper spacing)

use std::collections::HashMap;

//=============================================================================
// Data Structures
//=============================================================================

/// Node info for layout calculation.
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub layer: usize,
    #[allow(dead_code)]
    pub width: f32,
    pub height: f32,
}

/// Edge info for layout.
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
}

/// Result of layout calculation.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub positions: HashMap<String, (f32, f32)>,
}

/// Layout configuration.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub h_spacing: f32,
    pub v_spacing: f32,
    #[allow(dead_code)]
    pub node_sep: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            h_spacing: 330.0,
            v_spacing: 80.0,
            node_sep: 20.0,
        }
    }
}

//=============================================================================
// Public API
//=============================================================================

pub fn layout_graph(
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    config: LayoutConfig,
) -> LayoutResult {
    if nodes.is_empty() {
        return LayoutResult { positions: HashMap::new() };
    }

    // Build layer structure
    let mut layers: HashMap<usize, Vec<String>> = HashMap::new();
    let mut node_map: HashMap<String, LayoutNode> = HashMap::new();
    let mut max_layer = 0;

    for node in nodes {
        max_layer = max_layer.max(node.layer);
        layers.entry(node.layer).or_default().push(node.id.clone());
        node_map.insert(node.id.clone(), node);
    }

    // Build adjacency (both directions)
    let mut adj_upper: HashMap<String, Vec<String>> = HashMap::new(); // neighbors in layer-1
    let mut adj_lower: HashMap<String, Vec<String>> = HashMap::new(); // neighbors in layer+1

    for edge in &edges {
        let from_layer = node_map.get(&edge.from).map(|n| n.layer);
        let to_layer = node_map.get(&edge.to).map(|n| n.layer);

        if let (Some(fl), Some(tl)) = (from_layer, to_layer) {
            if fl < tl {
                adj_lower.entry(edge.from.clone()).or_default().push(edge.to.clone());
                adj_upper.entry(edge.to.clone()).or_default().push(edge.from.clone());
            } else if fl > tl {
                adj_upper.entry(edge.from.clone()).or_default().push(edge.to.clone());
                adj_lower.entry(edge.to.clone()).or_default().push(edge.from.clone());
            }
        }
    }

    // Convert to vector of layers
    let mut layer_vec: Vec<Vec<String>> = Vec::with_capacity(max_layer + 1);
    for i in 0..=max_layer {
        let mut layer = layers.remove(&i).unwrap_or_default();
        layer.sort(); // Initial alphabetical order
        layer_vec.push(layer);
    }

    // Phase 1: Crossing minimization via barycenter
    let mut positions: HashMap<String, usize> = HashMap::new();
    update_positions(&layer_vec, &mut positions);

    for _ in 0..20 {
        // Forward sweep
        for layer_idx in 1..layer_vec.len() {
            order_by_barycenter(&mut layer_vec[layer_idx], &adj_upper, &positions);
            update_layer_positions(&layer_vec[layer_idx], layer_idx, &mut positions);
        }
        // Backward sweep
        for layer_idx in (0..layer_vec.len().saturating_sub(1)).rev() {
            order_by_barycenter(&mut layer_vec[layer_idx], &adj_lower, &positions);
            update_layer_positions(&layer_vec[layer_idx], layer_idx, &mut positions);
        }
    }

    // Phase 2: Y coordinate assignment
    // Start with initial placement
    let mut y_coords: HashMap<String, f32> = HashMap::new();
    let spacing = config.v_spacing.max(20.0);
    
    for layer in &layer_vec {
        let mut y = 100.0;
        for id in layer {
            y_coords.insert(id.clone(), y);
            let h = node_map.get(id).map(|n| n.height).unwrap_or(50.0);
            y += h + spacing;
        }
    }

    // Iterative alignment - align nodes with their connected neighbors
    for _ in 0..30 {
        let mut changed = false;

        // Forward: align with upper neighbors
        for layer_idx in 1..layer_vec.len() {
            let layer = &layer_vec[layer_idx];
            let mut new_ys: Vec<(String, f32)> = Vec::new();

            for id in layer {
                let neighbors = adj_upper.get(id).map(|v| v.as_slice()).unwrap_or(&[]);
                let ideal = if neighbors.is_empty() {
                    y_coords.get(id).copied().unwrap_or(100.0)
                } else {
                    median_y(neighbors, &y_coords)
                };
                new_ys.push((id.clone(), ideal));
            }

            // Sort by ideal Y to maintain order
            new_ys.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            // Assign with spacing constraints
            if let Some(delta) = assign_with_spacing(&new_ys, &node_map, spacing, &mut y_coords) {
                if delta > 1.0 {
                    changed = true;
                }
            }
        }

        // Backward: align with lower neighbors
        for layer_idx in (0..layer_vec.len().saturating_sub(1)).rev() {
            let layer = &layer_vec[layer_idx];
            let mut new_ys: Vec<(String, f32)> = Vec::new();

            for id in layer {
                let neighbors = adj_lower.get(id).map(|v| v.as_slice()).unwrap_or(&[]);
                let ideal = if neighbors.is_empty() {
                    y_coords.get(id).copied().unwrap_or(100.0)
                } else {
                    median_y(neighbors, &y_coords)
                };
                new_ys.push((id.clone(), ideal));
            }

            new_ys.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some(delta) = assign_with_spacing(&new_ys, &node_map, spacing, &mut y_coords) {
                if delta > 1.0 {
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Center vertically
    center_graph(&mut y_coords);

    // Build final positions (X based on layer, Y from algorithm)
    let mut result = HashMap::new();
    for (id, node) in &node_map {
        let x = (max_layer - node.layer) as f32 * config.h_spacing + 100.0;
        let y = y_coords.get(id).copied().unwrap_or(300.0);
        result.insert(id.clone(), (x, y));
    }

    LayoutResult { positions: result }
}

//=============================================================================
// Helper Functions
//=============================================================================

fn update_positions(layers: &[Vec<String>], positions: &mut HashMap<String, usize>) {
    positions.clear();
    for layer in layers {
        for (i, id) in layer.iter().enumerate() {
            positions.insert(id.clone(), i);
        }
    }
}

fn update_layer_positions(layer: &[String], _layer_idx: usize, positions: &mut HashMap<String, usize>) {
    for (i, id) in layer.iter().enumerate() {
        positions.insert(id.clone(), i);
    }
}

/// Order layer by barycenter of neighbors.
fn order_by_barycenter(
    layer: &mut Vec<String>,
    adj: &HashMap<String, Vec<String>>,
    positions: &HashMap<String, usize>,
) {
    let mut barycenters: Vec<(String, f32)> = layer.iter().map(|id| {
        let neighbors = adj.get(id).map(|v| v.as_slice()).unwrap_or(&[]);
        let bc = if neighbors.is_empty() {
            positions.get(id).copied().unwrap_or(0) as f32
        } else {
            let sum: f32 = neighbors.iter()
                .filter_map(|n| positions.get(n))
                .map(|&p| p as f32)
                .sum();
            sum / neighbors.len() as f32
        };
        (id.clone(), bc)
    }).collect();

    barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    *layer = barycenters.into_iter().map(|(id, _)| id).collect();
}

/// Get median Y of neighbors.
fn median_y(neighbors: &[String], y_coords: &HashMap<String, f32>) -> f32 {
    let mut ys: Vec<f32> = neighbors.iter()
        .filter_map(|n| y_coords.get(n).copied())
        .collect();
    
    if ys.is_empty() {
        return 100.0;
    }
    
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let mid = ys.len() / 2;
    if ys.len() % 2 == 0 && ys.len() > 1 {
        (ys[mid - 1] + ys[mid]) / 2.0
    } else {
        ys[mid]
    }
}

/// Assign Y coordinates respecting spacing, returns max delta.
fn assign_with_spacing(
    sorted_nodes: &[(String, f32)],
    node_map: &HashMap<String, LayoutNode>,
    spacing: f32,
    y_coords: &mut HashMap<String, f32>,
) -> Option<f32> {
    if sorted_nodes.is_empty() {
        return None;
    }

    let mut max_delta: f32 = 0.0;

    // Find minimum Y from sorted ideals
    let min_ideal = sorted_nodes.iter().map(|(_, y)| *y).fold(f32::MAX, f32::min);
    let mut current_y = min_ideal;

    for (id, ideal) in sorted_nodes {
        let height = node_map.get(id).map(|n| n.height).unwrap_or(50.0);
        
        // Use ideal if it's >= current_y, otherwise use current_y
        let new_y = ideal.max(current_y);
        
        let old_y = y_coords.get(id).copied().unwrap_or(0.0);
        max_delta = max_delta.max((new_y - old_y).abs());
        
        y_coords.insert(id.clone(), new_y);
        current_y = new_y + height + spacing;
    }

    Some(max_delta)
}

fn center_graph(y_coords: &mut HashMap<String, f32>) {
    if y_coords.is_empty() {
        return;
    }

    let min_y = y_coords.values().copied().fold(f32::MAX, f32::min);
    let max_y = y_coords.values().copied().fold(f32::MIN, f32::max);
    let center = (min_y + max_y) / 2.0;
    let target_center = 400.0;
    let offset = target_center - center;

    for y in y_coords.values_mut() {
        *y += offset;
    }
}

//=============================================================================
// Tests
//=============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_layout() {
        let nodes = vec![
            LayoutNode { id: "A".into(), layer: 0, width: 100.0, height: 50.0 },
            LayoutNode { id: "B".into(), layer: 1, width: 100.0, height: 50.0 },
            LayoutNode { id: "C".into(), layer: 1, width: 100.0, height: 50.0 },
        ];
        let edges = vec![
            LayoutEdge { from: "B".into(), to: "A".into() },
            LayoutEdge { from: "C".into(), to: "A".into() },
        ];

        let result = layout_graph(nodes, edges, LayoutConfig::default());
        assert_eq!(result.positions.len(), 3);

        let a_y = result.positions.get("A").unwrap().1;
        let b_y = result.positions.get("B").unwrap().1;
        let c_y = result.positions.get("C").unwrap().1;

        let mid = (b_y + c_y) / 2.0;
        assert!((a_y - mid).abs() < 50.0, "A should be near middle of B and C");
    }

    #[test]
    fn test_chain_alignment() {
        let nodes = vec![
            LayoutNode { id: "A".into(), layer: 0, width: 100.0, height: 50.0 },
            LayoutNode { id: "B".into(), layer: 1, width: 100.0, height: 50.0 },
            LayoutNode { id: "C".into(), layer: 2, width: 100.0, height: 50.0 },
        ];
        let edges = vec![
            LayoutEdge { from: "B".into(), to: "A".into() },
            LayoutEdge { from: "C".into(), to: "B".into() },
        ];

        let result = layout_graph(nodes, edges, LayoutConfig::default());

        let a_y = result.positions.get("A").unwrap().1;
        let b_y = result.positions.get("B").unwrap().1;
        let c_y = result.positions.get("C").unwrap().1;

        assert!((a_y - b_y).abs() < 10.0, "A and B should be aligned: {} vs {}", a_y, b_y);
        assert!((b_y - c_y).abs() < 10.0, "B and C should be aligned: {} vs {}", b_y, c_y);
    }
}
