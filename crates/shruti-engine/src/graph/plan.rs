use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use shruti_dsp::AudioBuffer;

use super::node::{AudioNode, NodeId};

/// A connection between two nodes in the graph.
#[derive(Debug, Clone)]
pub struct Connection {
    pub from: NodeId,
    pub to: NodeId,
}

/// The audio graph — built on the non-RT thread.
pub struct Graph {
    nodes: HashMap<NodeId, Box<dyn AudioNode>>,
    connections: Vec<Connection>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: NodeId, node: Box<dyn AudioNode>) {
        self.nodes.insert(id, node);
    }

    pub fn connect(&mut self, from: NodeId, to: NodeId) {
        self.connections.push(Connection { from, to });
    }

    /// Compile the graph into a topologically sorted execution plan.
    pub fn compile(self) -> Result<ExecutionPlan, &'static str> {
        let order = topological_sort(&self.nodes, &self.connections)?;

        // Build input map: for each node, which nodes feed into it
        let mut input_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for conn in &self.connections {
            input_map.entry(conn.to).or_default().push(conn.from);
        }

        Ok(ExecutionPlan {
            order,
            nodes: self.nodes,
            input_map,
        })
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// A compiled, ready-to-execute plan for the audio thread.
pub struct ExecutionPlan {
    order: Vec<NodeId>,
    nodes: HashMap<NodeId, Box<dyn AudioNode>>,
    input_map: HashMap<NodeId, Vec<NodeId>>,
}

/// Processes an audio graph on the RT thread.
///
/// Holds the current execution plan behind an Arc<Mutex<>>.
/// The mutex is only locked briefly to swap plans; the RT thread
/// processes with a local reference.
pub struct GraphProcessor {
    plan: Arc<Mutex<Option<ExecutionPlan>>>,
}

impl GraphProcessor {
    pub fn new() -> Self {
        Self {
            plan: Arc::new(Mutex::new(None)),
        }
    }

    /// Get a handle for swapping in new plans from the non-RT thread.
    pub fn swap_handle(&self) -> GraphSwapHandle {
        GraphSwapHandle {
            plan: Arc::clone(&self.plan),
        }
    }

    /// Process one buffer cycle. Writes interleaved output into `output`.
    /// Called from the audio callback.
    pub fn process(&mut self, output: &mut [f32], channels: u16, buffer_frames: u32) {
        let mut guard = match self.plan.try_lock() {
            Ok(g) => g,
            Err(_) => {
                // Mutex contended — fill with silence rather than block
                output.fill(0.0);
                return;
            }
        };

        let plan = match guard.as_mut() {
            Some(p) => p,
            None => {
                output.fill(0.0);
                return;
            }
        };

        // Pre-allocate output buffers for each node
        let mut node_outputs: HashMap<NodeId, AudioBuffer> = HashMap::new();

        for &node_id in &plan.order {
            let mut node_buf = AudioBuffer::new(channels, buffer_frames);

            // Gather inputs
            let inputs: Vec<&AudioBuffer> = plan
                .input_map
                .get(&node_id)
                .map(|sources| {
                    sources
                        .iter()
                        .filter_map(|src_id| node_outputs.get(src_id))
                        .collect()
                })
                .unwrap_or_default();

            if let Some(node) = plan.nodes.get_mut(&node_id) {
                node.process(&inputs, &mut node_buf);
            }

            node_outputs.insert(node_id, node_buf);
        }

        // The last node in the plan is the output — copy to device buffer
        if let Some(last_id) = plan.order.last() {
            if let Some(last_buf) = node_outputs.get(last_id) {
                let src = last_buf.as_interleaved();
                let len = output.len().min(src.len());
                output[..len].copy_from_slice(&src[..len]);
                if output.len() > len {
                    output[len..].fill(0.0);
                }
            }
        } else {
            output.fill(0.0);
        }
    }

    /// Check if the last node in the plan is finished.
    pub fn is_finished(&self) -> bool {
        let guard = match self.plan.try_lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        match guard.as_ref() {
            Some(plan) => {
                if let Some(last_id) = plan.order.last() {
                    plan.nodes.get(last_id).map(|n| n.is_finished()).unwrap_or(true)
                } else {
                    true
                }
            }
            None => true,
        }
    }
}

impl Default for GraphProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for swapping execution plans from the non-RT thread.
pub struct GraphSwapHandle {
    plan: Arc<Mutex<Option<ExecutionPlan>>>,
}

impl GraphSwapHandle {
    pub fn swap(&self, new_plan: ExecutionPlan) {
        if let Ok(mut guard) = self.plan.lock() {
            *guard = Some(new_plan);
        }
    }
}

/// Topological sort using Kahn's algorithm.
fn topological_sort(
    nodes: &HashMap<NodeId, Box<dyn AudioNode>>,
    connections: &[Connection],
) -> Result<Vec<NodeId>, &'static str> {
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for &id in nodes.keys() {
        in_degree.entry(id).or_insert(0);
        adjacency.entry(id).or_default();
    }

    for conn in connections {
        *in_degree.entry(conn.to).or_insert(0) += 1;
        adjacency.entry(conn.from).or_default().push(conn.to);
    }

    let mut queue: Vec<NodeId> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort_by_key(|id| id.0); // deterministic order

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node);
        if let Some(neighbors) = adjacency.get(&node) {
            for &neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(&neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }
    }

    if result.len() != nodes.len() {
        return Err("cycle detected in audio graph");
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::node::{FilePlayerNode, GainNode};

    #[test]
    fn test_topological_sort_simple() {
        let mut nodes: HashMap<NodeId, Box<dyn AudioNode>> = HashMap::new();
        let a = NodeId::next();
        let b = NodeId::next();
        nodes.insert(a, Box::new(GainNode::new(1.0)));
        nodes.insert(b, Box::new(GainNode::new(1.0)));

        let connections = vec![Connection { from: a, to: b }];
        let order = topological_sort(&nodes, &connections).unwrap();

        assert_eq!(order[0], a);
        assert_eq!(order[1], b);
    }

    #[test]
    fn test_cycle_detection() {
        let mut nodes: HashMap<NodeId, Box<dyn AudioNode>> = HashMap::new();
        let a = NodeId::next();
        let b = NodeId::next();
        nodes.insert(a, Box::new(GainNode::new(1.0)));
        nodes.insert(b, Box::new(GainNode::new(1.0)));

        let connections = vec![
            Connection { from: a, to: b },
            Connection { from: b, to: a },
        ];

        assert!(topological_sort(&nodes, &connections).is_err());
    }

    #[test]
    fn test_graph_compile_and_process() {
        let src = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.5, -0.5], 2);
        let player_id = NodeId::next();
        let gain_id = NodeId::next();

        let mut graph = Graph::new();
        graph.add_node(player_id, Box::new(FilePlayerNode::new(src, false)));
        graph.add_node(gain_id, Box::new(GainNode::new(0.5)));
        graph.connect(player_id, gain_id);

        let plan = graph.compile().unwrap();

        let mut processor = GraphProcessor::new();
        let handle = processor.swap_handle();
        handle.swap(plan);

        let mut output = vec![0.0f32; 4];
        processor.process(&mut output, 2, 2);

        assert!((output[0] - 0.25).abs() < 1e-6);
        assert!((output[1] - -0.25).abs() < 1e-6);
    }
}
