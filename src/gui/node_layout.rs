//! Node layout algorithms for dependency graphs.
//!
//! # Overview
//! Implements Sugiyama-style hierarchical layout algorithm for visualizing
//! package dependency graphs. The goal is to minimize edge crossings and
//! make connections as horizontal as possible.
//!
//! # Algorithm Phases
//! 1. **Layer Assignment** - provided by caller (package depth in dependency tree)
//! 2. **Crossing Minimization** - barycenter heuristic with PIN position awareness
//! 3. **Coordinate Assignment** - align nodes with their connected neighbors
//!
//! # Why Sugiyama?
//! - Industry standard for DAG visualization (dependency graphs are DAGs)
//! - Produces readable layouts with minimal edge crossings
//! - Handles varying node sizes and connection counts
//!
//! # Key Innovation: PIN-aware Barycenter
//! Standard barycenter uses node center positions. But when a node has multiple
//! input pins (like ROOT with 10+ requirements), all connected nodes get the
//! same barycenter value and end up sorted alphabetically - causing edge crossings.
//!
//! Our solution: track which INPUT PIN each edge connects to, and use that pin's
//! Y position for barycenter calculation. This ensures nodes are ordered to match
//! their pin positions on the neighbor node.
//!
//! # Usage
//! Called from `node_graph.rs` during graph rebuild:
//! ```ignore
//! let result = layout_graph(layout_nodes, layout_edges, config);
//! // result.positions contains (x, y) for each node ID
//! ```

use std::collections::HashMap;

//=============================================================================
// Data Structures
//=============================================================================

/// Node info for layout calculation.
/// 
/// Created in node_graph.rs from PackageNode data.
/// Layer = depth in dependency tree (0 = root/selected package).
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    /// Layer in the graph (0 = root, 1 = direct deps, etc.)
    /// Lower layer = further right in visualization (root is rightmost)
    pub layer: usize,
    #[allow(dead_code)]
    pub width: f32,
    /// Node height - affects spacing calculations
    /// Computed as: base_height + num_pins * pin_spacing
    pub height: f32,
}

/// Edge info for layout.
/// 
/// Direction: from dependency TO dependent (from -> to).
/// Example: "houdini" -> "houdini-fx" means houdini-fx requires houdini.
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    /// Source node (the dependency)
    pub from: String,
    /// Target node (the dependent) - has input pin for this connection
    pub to: String,
}

/// Result of layout calculation.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// Map of node ID -> (x, y) screen coordinates
    pub positions: HashMap<String, (f32, f32)>,
}

/// Layout configuration from UI sliders.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Horizontal spacing between layers (H slider, 150-500)
    pub h_spacing: f32,
    /// Vertical spacing between nodes in same layer (V slider, 10-100)
    pub v_spacing: f32,
    #[allow(dead_code)]
    pub node_sep: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            h_spacing: 330.0,
            v_spacing: 30.0,
            node_sep: 20.0,
        }
    }
}

//=============================================================================
// Public API
//=============================================================================

/// Main entry point for layout calculation.
/// 
/// # Algorithm
/// 1. Build layer structure from nodes
/// 2. Compute input pin indices for each edge (for PIN-aware barycenter)
/// 3. Build adjacency lists with pin position fractions
/// 4. Phase 1: Order nodes within layers using barycenter (30 iterations)
/// 5. Phase 2: Fine-tune Y positions to align with neighbors (30 iterations)
/// 6. Center graph vertically and compute final X,Y positions
/// 
/// # Arguments
/// * `nodes` - Nodes with layer assignments
/// * `edges` - Edges from dependencies to dependents  
/// * `config` - Spacing configuration from UI
/// 
/// # Returns
/// HashMap of node ID -> (x, y) screen coordinates
pub fn layout_graph(
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    config: LayoutConfig,
) -> LayoutResult {
    if nodes.is_empty() {
        return LayoutResult { positions: HashMap::new() };
    }

    // Build layer structure: layer_index -> list of node IDs
    let mut layers: HashMap<usize, Vec<String>> = HashMap::new();
    let mut node_map: HashMap<String, LayoutNode> = HashMap::new();
    let mut max_layer = 0;

    for node in nodes {
        max_layer = max_layer.max(node.layer);
        layers.entry(node.layer).or_default().push(node.id.clone());
        node_map.insert(node.id.clone(), node);
    }

    // =======================================================================
    // PIN INDEX TRACKING
    // =======================================================================
    // For each edge, track which input pin it connects to on the target node.
    // This is crucial for PIN-aware barycenter calculation.
    //
    // Example: ROOT has reqs [houdini, python, numpy, ..., alembic]
    // - Edge houdini->ROOT gets pin index 0 (top pin)
    // - Edge alembic->ROOT gets pin index 10 (bottom pin)
    //
    // Without this, all nodes connecting to ROOT would have same barycenter
    // and would be sorted alphabetically, causing massive edge crossings.
    // =======================================================================
    
    let mut input_count: HashMap<String, usize> = HashMap::new();
    let mut input_index: HashMap<(String, String), usize> = HashMap::new();
    
    for edge in &edges {
        let count = input_count.entry(edge.to.clone()).or_insert(0);
        input_index.insert((edge.to.clone(), edge.from.clone()), *count);
        *count += 1;
    }

    // =======================================================================
    // ADJACENCY LISTS WITH PIN FRACTIONS
    // =======================================================================
    // adj_upper[node] = neighbors in layer-1 (smaller layer number, to the RIGHT)
    // adj_lower[node] = neighbors in layer+1 (larger layer number, to the LEFT)
    //
    // Each entry is (neighbor_id, pin_fraction) where:
    // - pin_fraction = pin_index / total_pins (0.0 = top, 1.0 = bottom)
    // - Used to compute effective Y position of the connection point
    // =======================================================================
    
    let mut adj_upper: HashMap<String, Vec<(String, f32)>> = HashMap::new();
    let mut adj_lower: HashMap<String, Vec<(String, f32)>> = HashMap::new();

    for edge in &edges {
        let from_layer = node_map.get(&edge.from).map(|n| n.layer);
        let to_layer = node_map.get(&edge.to).map(|n| n.layer);

        if let (Some(fl), Some(tl)) = (from_layer, to_layer) {
            // pin_frac: relative position of input pin on target node
            let to_pins = input_count.get(&edge.to).copied().unwrap_or(1).max(1);
            let pin_idx = input_index.get(&(edge.to.clone(), edge.from.clone())).copied().unwrap_or(0);
            let pin_frac = pin_idx as f32 / to_pins as f32;

            if fl < tl {
                // from is upper (smaller layer), to is lower (larger layer)
                // edge arrives at input pin on "to" node
                adj_lower.entry(edge.from.clone()).or_default().push((edge.to.clone(), pin_frac));
                adj_upper.entry(edge.to.clone()).or_default().push((edge.from.clone(), 0.5));
            } else if fl > tl {
                // from is lower (larger layer), to is upper (smaller layer)  
                // edge arrives at input pin on "to" node - USE pin_frac!
                adj_upper.entry(edge.from.clone()).or_default().push((edge.to.clone(), pin_frac));
                adj_lower.entry(edge.to.clone()).or_default().push((edge.from.clone(), 0.5));
            }
        }
    }

    // Convert HashMap layers to ordered Vec for iteration
    let mut layer_vec: Vec<Vec<String>> = Vec::with_capacity(max_layer + 1);
    for i in 0..=max_layer {
        let mut layer = layers.remove(&i).unwrap_or_default();
        layer.sort(); // Initial alphabetical order
        layer_vec.push(layer);
    }

    // =======================================================================
    // PHASE 1: CROSSING MINIMIZATION (Barycenter Method)
    // =======================================================================
    // Iteratively reorder nodes within each layer to minimize edge crossings.
    // 
    // Barycenter = weighted average Y position of connected neighbors.
    // Nodes with lower barycenter are placed higher (smaller Y).
    //
    // We do both forward (layer 1->N) and backward (N->1) sweeps because
    // ordering depends on neighbors, and neighbors' positions change.
    // 30 iterations is usually enough for convergence.
    // =======================================================================
    
    let mut y_coords: HashMap<String, f32> = HashMap::new();
    let spacing = config.v_spacing.max(10.0);
    
    // Initial Y placement: stack nodes vertically with spacing
    for layer in &layer_vec {
        let mut y = 100.0;
        for id in layer {
            y_coords.insert(id.clone(), y);
            let h = node_map.get(id).map(|n| n.height).unwrap_or(50.0);
            y += h + spacing;
        }
    }

    // Iterative barycenter ordering
    for _ in 0..30 {
        // Forward sweep: order each layer by connections to previous layer
        for layer_idx in 1..layer_vec.len() {
            order_by_pin_barycenter(&mut layer_vec[layer_idx], &adj_upper, &node_map, &y_coords);
            // Update Y positions after reordering
            let mut y = 100.0;
            for id in &layer_vec[layer_idx] {
                y_coords.insert(id.clone(), y);
                let h = node_map.get(id).map(|n| n.height).unwrap_or(50.0);
                y += h + spacing;
            }
        }

        // Backward sweep: order each layer by connections to next layer
        for layer_idx in (0..layer_vec.len().saturating_sub(1)).rev() {
            order_by_pin_barycenter(&mut layer_vec[layer_idx], &adj_lower, &node_map, &y_coords);
            let mut y = 100.0;
            for id in &layer_vec[layer_idx] {
                y_coords.insert(id.clone(), y);
                let h = node_map.get(id).map(|n| n.height).unwrap_or(50.0);
                y += h + spacing;
            }
        }
    }

    // =======================================================================
    // PHASE 2: COORDINATE ASSIGNMENT (Alignment)
    // =======================================================================
    // After ordering is fixed, fine-tune Y positions to make edges more
    // horizontal. Each node tries to align with the median Y of its neighbors.
    //
    // Constraints:
    // - Maintain node order within layer (no reordering)
    // - Maintain minimum spacing between adjacent nodes
    // - Use dampening (0.3) to prevent oscillation
    // =======================================================================
    
    for _ in 0..30 {
        let mut changed = false;

        // Forward: align with upper neighbors (to the right)
        for layer_idx in 1..layer_vec.len() {
            if align_to_neighbors(&layer_vec[layer_idx], &adj_upper, &node_map, spacing, &mut y_coords) {
                changed = true;
            }
        }

        // Backward: align with lower neighbors (to the left)
        for layer_idx in (0..layer_vec.len().saturating_sub(1)).rev() {
            if align_to_neighbors(&layer_vec[layer_idx], &adj_lower, &node_map, spacing, &mut y_coords) {
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    // Center the entire graph vertically around Y=400
    center_graph(&mut y_coords);

    // =======================================================================
    // FINAL POSITION CALCULATION
    // =======================================================================
    // X position: based on layer (layer 0 = rightmost, higher layers = left)
    // Y position: from the layout algorithm above
    // =======================================================================
    
    let mut result = HashMap::new();
    for (id, node) in &node_map {
        // X: rightmost layer (0) gets highest X, leftmost gets lowest
        let x = (max_layer - node.layer) as f32 * config.h_spacing + 100.0;
        let y = y_coords.get(id).copied().unwrap_or(300.0);
        result.insert(id.clone(), (x, y));
    }

    LayoutResult { positions: result }
}

//=============================================================================
// Helper Functions
//=============================================================================

/// Order nodes in a layer by PIN-aware barycenter.
/// 
/// Barycenter = average Y position of connected PIN positions on neighbors.
/// This is the key innovation: we use PIN positions, not node centers.
/// 
/// # Why PIN positions matter
/// If ROOT has 10 input pins and 10 nodes connect to it, standard barycenter
/// would give all 10 nodes the same value (ROOT's center). With PIN positions,
/// the node connecting to pin 0 (top) gets lower barycenter than the node
/// connecting to pin 9 (bottom).
fn order_by_pin_barycenter(
    layer: &mut Vec<String>,
    adj: &HashMap<String, Vec<(String, f32)>>,
    node_map: &HashMap<String, LayoutNode>,
    y_coords: &HashMap<String, f32>,
) {
    let mut barycenters: Vec<(String, f32)> = layer.iter().map(|id| {
        let neighbors = adj.get(id).map(|v| v.as_slice()).unwrap_or(&[]);
        
        if neighbors.is_empty() {
            // No neighbors: keep current position
            (id.clone(), y_coords.get(id).copied().unwrap_or(0.0))
        } else {
            // Calculate weighted average Y of neighbor PIN positions
            let mut sum_y = 0.0;
            for (neighbor_id, pin_frac) in neighbors {
                let neighbor_y = y_coords.get(neighbor_id).copied().unwrap_or(0.0);
                let neighbor_h = node_map.get(neighbor_id).map(|n| n.height).unwrap_or(50.0);
                // Pin Y = top of node + fraction * height
                let pin_y = neighbor_y + pin_frac * neighbor_h;
                sum_y += pin_y;
            }
            (id.clone(), sum_y / neighbors.len() as f32)
        }
    }).collect();

    // Sort by barycenter (lower value = higher position)
    barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    *layer = barycenters.into_iter().map(|(id, _)| id).collect();
}

/// Align nodes toward their neighbors while maintaining order and spacing.
/// 
/// For each node, compute ideal Y (median of neighbor PIN positions),
/// then move toward it with dampening. Respects minimum spacing.
/// 
/// Returns true if any node moved significantly (>0.5 pixels).
fn align_to_neighbors(
    layer: &[String],
    adj: &HashMap<String, Vec<(String, f32)>>,
    node_map: &HashMap<String, LayoutNode>,
    spacing: f32,
    y_coords: &mut HashMap<String, f32>,
) -> bool {
    if layer.is_empty() {
        return false;
    }

    let mut changed = false;
    let mut prev_bottom = f32::MIN;

    for id in layer {
        let height = node_map.get(id).map(|n| n.height).unwrap_or(50.0);
        let current_y = y_coords.get(id).copied().unwrap_or(100.0);
        
        let neighbors = adj.get(id).map(|v| v.as_slice()).unwrap_or(&[]);
        
        // Compute ideal Y: median of neighbor PIN positions
        let ideal = if neighbors.is_empty() {
            current_y
        } else {
            let mut pin_ys: Vec<f32> = neighbors.iter().map(|(neighbor_id, pin_frac)| {
                let neighbor_y = y_coords.get(neighbor_id).copied().unwrap_or(0.0);
                let neighbor_h = node_map.get(neighbor_id).map(|n| n.height).unwrap_or(50.0);
                neighbor_y + pin_frac * neighbor_h
            }).collect();
            
            pin_ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = pin_ys.len() / 2;
            if pin_ys.len() % 2 == 0 && pin_ys.len() > 1 {
                (pin_ys[mid - 1] + pin_ys[mid]) / 2.0
            } else {
                pin_ys.get(mid).copied().unwrap_or(current_y)
            }
        };

        // Minimum Y: must be below previous node with spacing
        let min_y = if prev_bottom == f32::MIN {
            f32::MIN
        } else {
            prev_bottom + spacing
        };

        // Move toward ideal with 30% dampening (prevents oscillation)
        let target = ideal.max(min_y);
        let new_y = current_y + (target - current_y) * 0.3;
        let final_y = new_y.max(min_y);

        if (final_y - current_y).abs() > 0.5 {
            changed = true;
        }

        y_coords.insert(id.clone(), final_y);
        prev_bottom = final_y + height;
    }

    changed
}

/// Center the graph vertically around Y=400.
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

    /// Test basic layout with one parent and two children.
    /// Children should be positioned around the parent's Y.
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
    }

    /// Test chain layout: A <- B <- C should be horizontally aligned.
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

        // All three should be roughly aligned (within 25 pixels)
        assert!((a_y - b_y).abs() < 25.0, "A and B should be aligned: {} vs {}", a_y, b_y);
        assert!((b_y - c_y).abs() < 25.0, "B and C should be aligned: {} vs {}", b_y, c_y);
    }
}
