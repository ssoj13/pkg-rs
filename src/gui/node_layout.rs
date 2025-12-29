//! Node layout algorithms for dependency graphs.
//!
//! Simple hierarchical layout with barycenter crossing minimization.

use std::collections::HashMap;

/// Node info for layout calculation.
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub layer: usize,
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
}

/// Simple hierarchical layout with barycenter ordering.
pub fn layout_graph(
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    config: LayoutConfig,
) -> LayoutResult {
    if nodes.is_empty() {
        return LayoutResult { positions: HashMap::new() };
    }

    // Group nodes by layer
    let mut layers: HashMap<usize, Vec<String>> = HashMap::new();
    let mut node_map: HashMap<String, LayoutNode> = HashMap::new();
    let mut max_layer = 0;
    
    for node in nodes {
        max_layer = max_layer.max(node.layer);
        layers.entry(node.layer).or_default().push(node.id.clone());
        node_map.insert(node.id.clone(), node);
    }

    // Build adjacency: which nodes connect to which
    let mut children: HashMap<String, Vec<String>> = HashMap::new();  // node -> nodes it depends on
    let mut parents: HashMap<String, Vec<String>> = HashMap::new();   // node -> nodes that depend on it
    
    for edge in &edges {
        children.entry(edge.to.clone()).or_default().push(edge.from.clone());
        parents.entry(edge.from.clone()).or_default().push(edge.to.clone());
    }

    // Sort each layer initially
    for layer in layers.values_mut() {
        layer.sort();
    }

    // Barycenter ordering: multiple passes to minimize crossings
    for _pass in 0..6 {
        // Forward pass (layer 0 -> max)
        for layer_idx in 1..=max_layer {
            order_layer_by_barycenter(&mut layers, layer_idx, &parents, true);
        }
        // Backward pass (max -> layer 0)
        for layer_idx in (0..max_layer).rev() {
            order_layer_by_barycenter(&mut layers, layer_idx, &children, false);
        }
    }

    // Calculate positions
    let mut positions = HashMap::new();
    
    for layer_idx in 0..=max_layer {
        if let Some(layer_nodes) = layers.get(&layer_idx) {
            let layer_count = layer_nodes.len();
            
            // Center the layer vertically
            let total_height = (layer_count as f32 - 1.0) * config.v_spacing;
            let start_y = -total_height / 2.0;
            
            for (slot, node_id) in layer_nodes.iter().enumerate() {
                // X: layer position (root on right, deps on left)
                let x = (max_layer - layer_idx) as f32 * config.h_spacing + 100.0;
                // Y: slot within layer, centered
                let y = start_y + slot as f32 * config.v_spacing + 300.0;
                
                positions.insert(node_id.clone(), (x, y));
            }
        }
    }

    LayoutResult { positions }
}

/// Order nodes in a layer by barycenter of connected nodes in adjacent layer.
fn order_layer_by_barycenter(
    layers: &mut HashMap<usize, Vec<String>>,
    layer_idx: usize,
    connections: &HashMap<String, Vec<String>>,
    use_prev_layer: bool,
) {
    let adj_layer_idx = if use_prev_layer {
        layer_idx.saturating_sub(1)
    } else {
        layer_idx + 1
    };
    
    // Get positions in adjacent layer
    let adj_positions: HashMap<String, f32> = layers
        .get(&adj_layer_idx)
        .map(|adj| adj.iter().enumerate().map(|(i, id)| (id.clone(), i as f32)).collect())
        .unwrap_or_default();
    
    if adj_positions.is_empty() {
        return;
    }

    if let Some(layer) = layers.get_mut(&layer_idx) {
        // Calculate barycenter for each node
        let mut barycenters: Vec<(String, f32)> = layer.iter().map(|node_id| {
            let connected = connections.get(node_id).cloned().unwrap_or_default();
            let positions: Vec<f32> = connected
                .iter()
                .filter_map(|c| adj_positions.get(c).copied())
                .collect();
            
            let bc = if positions.is_empty() {
                // Keep relative order for unconnected nodes
                layer.iter().position(|id| id == node_id).unwrap_or(0) as f32
            } else {
                positions.iter().sum::<f32>() / positions.len() as f32
            };
            
            (node_id.clone(), bc)
        }).collect();

        // Sort by barycenter
        barycenters.sort_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        *layer = barycenters.into_iter().map(|(id, _)| id).collect();
    }
}
