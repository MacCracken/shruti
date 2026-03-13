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
    /// Pre-allocated node output buffers, reused across process() calls.
    /// Avoids HashMap + AudioBuffer heap allocation on every audio callback.
    node_outputs: HashMap<NodeId, AudioBuffer>,
}

impl GraphProcessor {
    pub fn new() -> Self {
        Self {
            plan: Arc::new(Mutex::new(None)),
            node_outputs: HashMap::new(),
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
            Err(std::sync::TryLockError::WouldBlock) => {
                // Mutex contended — fill with silence rather than block
                output.fill(0.0);
                return;
            }
            Err(std::sync::TryLockError::Poisoned(e)) => {
                eprintln!("shruti-engine: graph plan mutex poisoned, recovering: {e}");
                e.into_inner()
            }
        };

        let plan = match guard.as_mut() {
            Some(p) => p,
            None => {
                output.fill(0.0);
                return;
            }
        };

        // Reuse pre-allocated node output buffers (avoid HashMap + AudioBuffer
        // allocation on the RT thread). Only clear values, keep capacity.
        for buf in self.node_outputs.values_mut() {
            buf.clear();
        }
        for &node_id in &plan.order {
            self.node_outputs
                .entry(node_id)
                .or_insert_with(|| AudioBuffer::new(channels, buffer_frames));
            let entry = self.node_outputs.get_mut(&node_id).unwrap();
            if entry.channels() != channels || entry.frames() != buffer_frames {
                *entry = AudioBuffer::new(channels, buffer_frames);
            }
        }

        for idx in 0..plan.order.len() {
            let node_id = plan.order[idx];

            // Temporarily remove this node's buffer to get a mutable ref
            // while holding immutable refs to other buffers.
            let mut node_buf = self.node_outputs.remove(&node_id).unwrap();

            // Gather input references from already-processed nodes
            let inputs: Vec<&AudioBuffer> = plan
                .input_map
                .get(&node_id)
                .map(|sources| {
                    sources
                        .iter()
                        .filter_map(|src_id| self.node_outputs.get(src_id))
                        .collect()
                })
                .unwrap_or_default();

            if let Some(node) = plan.nodes.get_mut(&node_id) {
                node.process(&inputs, &mut node_buf);
            }

            // Put the buffer back
            self.node_outputs.insert(node_id, node_buf);
        }

        // The last node in the plan is the output — copy to device buffer
        if let Some(last_id) = plan.order.last() {
            if let Some(last_buf) = self.node_outputs.get(last_id) {
                let src = last_buf.as_interleaved();
                let len = output.len().min(src.len());
                output[..len].copy_from_slice(&src[..len]);
                if output.len() > len {
                    eprintln!(
                        "shruti-engine: interleaved buffer shorter than expected \
                         (got {} samples, need {}), zero-filling remainder",
                        src.len(),
                        output.len()
                    );
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
            Err(std::sync::TryLockError::WouldBlock) => return false,
            Err(std::sync::TryLockError::Poisoned(e)) => {
                eprintln!(
                    "shruti-engine: graph plan mutex poisoned in is_finished, recovering: {e}"
                );
                e.into_inner()
            }
        };
        match guard.as_ref() {
            Some(plan) => {
                if let Some(last_id) = plan.order.last() {
                    plan.nodes
                        .get(last_id)
                        .map(|n| n.is_finished())
                        .unwrap_or(true)
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
        match self.plan.lock() {
            Ok(mut guard) => {
                *guard = Some(new_plan);
            }
            Err(e) => {
                eprintln!("shruti-engine: graph plan mutex poisoned in swap, recovering: {e}");
                let mut guard = e.into_inner();
                *guard = Some(new_plan);
            }
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

        let connections = vec![Connection { from: a, to: b }, Connection { from: b, to: a }];

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

    #[test]
    fn test_empty_graph_compiles() {
        let graph = Graph::new();
        let plan = graph.compile().unwrap();
        assert!(plan.order.is_empty());
    }

    #[test]
    fn test_single_node_graph() {
        let mut graph = Graph::new();
        let id = NodeId::next();
        let src = AudioBuffer::from_interleaved(vec![0.5, -0.5], 2);
        graph.add_node(id, Box::new(FilePlayerNode::new(src, false)));

        let plan = graph.compile().unwrap();
        assert_eq!(plan.order.len(), 1);
        assert_eq!(plan.order[0], id);
    }

    #[test]
    fn test_disconnected_nodes_all_present() {
        let mut graph = Graph::new();
        let a = NodeId::next();
        let b = NodeId::next();
        let c = NodeId::next();
        graph.add_node(a, Box::new(GainNode::new(1.0)));
        graph.add_node(b, Box::new(GainNode::new(1.0)));
        graph.add_node(c, Box::new(GainNode::new(1.0)));
        // No connections

        let plan = graph.compile().unwrap();
        assert_eq!(plan.order.len(), 3);
        // All nodes present
        assert!(plan.order.contains(&a));
        assert!(plan.order.contains(&b));
        assert!(plan.order.contains(&c));
    }

    #[test]
    fn test_self_loop_detected() {
        let mut nodes: HashMap<NodeId, Box<dyn AudioNode>> = HashMap::new();
        let a = NodeId::next();
        nodes.insert(a, Box::new(GainNode::new(1.0)));

        let connections = vec![Connection { from: a, to: a }];
        let result = topological_sort(&nodes, &connections);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "cycle detected in audio graph");
    }

    #[test]
    fn test_three_node_chain_order() {
        let mut nodes: HashMap<NodeId, Box<dyn AudioNode>> = HashMap::new();
        let a = NodeId::next();
        let b = NodeId::next();
        let c = NodeId::next();
        nodes.insert(a, Box::new(GainNode::new(1.0)));
        nodes.insert(b, Box::new(GainNode::new(1.0)));
        nodes.insert(c, Box::new(GainNode::new(1.0)));

        let connections = vec![Connection { from: a, to: b }, Connection { from: b, to: c }];
        let order = topological_sort(&nodes, &connections).unwrap();
        // a must come before b, b before c
        let pos_a = order.iter().position(|&x| x == a).unwrap();
        let pos_b = order.iter().position(|&x| x == b).unwrap();
        let pos_c = order.iter().position(|&x| x == c).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_diamond_graph() {
        // A -> B, A -> C, B -> D, C -> D
        let mut nodes: HashMap<NodeId, Box<dyn AudioNode>> = HashMap::new();
        let a = NodeId::next();
        let b = NodeId::next();
        let c = NodeId::next();
        let d = NodeId::next();
        nodes.insert(a, Box::new(GainNode::new(1.0)));
        nodes.insert(b, Box::new(GainNode::new(1.0)));
        nodes.insert(c, Box::new(GainNode::new(1.0)));
        nodes.insert(d, Box::new(GainNode::new(1.0)));

        let connections = vec![
            Connection { from: a, to: b },
            Connection { from: a, to: c },
            Connection { from: b, to: d },
            Connection { from: c, to: d },
        ];
        let order = topological_sort(&nodes, &connections).unwrap();
        let pos = |id: NodeId| order.iter().position(|&x| x == id).unwrap();
        assert!(pos(a) < pos(b));
        assert!(pos(a) < pos(c));
        assert!(pos(b) < pos(d));
        assert!(pos(c) < pos(d));
    }

    #[test]
    fn test_processor_with_none_plan() {
        let mut processor = GraphProcessor::new();
        let mut output = vec![1.0f32; 8];
        processor.process(&mut output, 2, 4);
        // No plan means silence
        for &s in &output {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn test_processor_is_finished_no_plan() {
        let processor = GraphProcessor::new();
        // No plan => is_finished returns true
        assert!(processor.is_finished());
    }

    #[test]
    fn test_processor_is_finished_with_active_player() {
        let src = AudioBuffer::from_interleaved(vec![1.0, 1.0, 1.0, 1.0], 2);
        let player_id = NodeId::next();

        let mut graph = Graph::new();
        graph.add_node(player_id, Box::new(FilePlayerNode::new(src, false)));

        let plan = graph.compile().unwrap();
        let mut processor = GraphProcessor::new();
        let handle = processor.swap_handle();
        handle.swap(plan);

        // Player has 2 frames, not yet processed
        assert!(!processor.is_finished());

        // Process enough to finish
        let mut output = vec![0.0f32; 4];
        processor.process(&mut output, 2, 2);
        assert!(processor.is_finished());
    }

    #[test]
    fn test_plan_swap_replaces_plan() {
        let mut processor = GraphProcessor::new();
        let handle = processor.swap_handle();

        // First plan: gain=0.5
        let src1 = AudioBuffer::from_interleaved(vec![1.0, 1.0], 1);
        let p1 = NodeId::next();
        let g1 = NodeId::next();
        let mut graph1 = Graph::new();
        graph1.add_node(p1, Box::new(FilePlayerNode::new(src1, true)));
        graph1.add_node(g1, Box::new(GainNode::new(0.5)));
        graph1.connect(p1, g1);
        handle.swap(graph1.compile().unwrap());

        let mut output = vec![0.0f32; 1];
        processor.process(&mut output, 1, 1);
        assert!((output[0] - 0.5).abs() < 1e-6);

        // Second plan: gain=0.25
        let src2 = AudioBuffer::from_interleaved(vec![1.0, 1.0], 1);
        let p2 = NodeId::next();
        let g2 = NodeId::next();
        let mut graph2 = Graph::new();
        graph2.add_node(p2, Box::new(FilePlayerNode::new(src2, true)));
        graph2.add_node(g2, Box::new(GainNode::new(0.25)));
        graph2.connect(p2, g2);
        handle.swap(graph2.compile().unwrap());

        let mut output2 = vec![0.0f32; 1];
        processor.process(&mut output2, 1, 1);
        assert!((output2[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_processor_empty_plan_fills_silence() {
        let graph = Graph::new();
        let plan = graph.compile().unwrap();

        let mut processor = GraphProcessor::new();
        let handle = processor.swap_handle();
        handle.swap(plan);

        let mut output = vec![1.0f32; 4];
        processor.process(&mut output, 2, 2);
        for &s in &output {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn test_graph_default() {
        let graph = Graph::default();
        let plan = graph.compile().unwrap();
        assert!(plan.order.is_empty());
    }

    #[test]
    fn test_processor_default() {
        let processor = GraphProcessor::default();
        assert!(processor.is_finished());
    }

    #[test]
    fn test_connection_debug() {
        let a = NodeId::next();
        let b = NodeId::next();
        let conn = Connection { from: a, to: b };
        let cloned = conn.clone();
        assert_eq!(format!("{:?}", conn), format!("{:?}", cloned));
    }

    // --- Poisoned mutex recovery tests ---

    #[test]
    fn test_processor_recovers_from_poisoned_mutex() {
        let processor = GraphProcessor::new();
        let plan_arc = Arc::clone(&processor.plan);

        // Poison the mutex by panicking in a thread that holds the lock.
        let plan_arc2 = Arc::clone(&plan_arc);
        let _ = std::thread::spawn(move || {
            let _guard = plan_arc2.lock().unwrap();
            panic!("intentional panic to poison mutex");
        })
        .join();

        // Verify mutex is poisoned.
        assert!(plan_arc.lock().is_err());

        // is_finished should recover gracefully (not panic).
        // With poisoned mutex, it recovers and reads None => true.
        let result = processor.is_finished();
        assert!(result);
    }

    #[test]
    fn test_swap_handle_recovers_from_poisoned_mutex() {
        let processor = GraphProcessor::new();
        let handle = processor.swap_handle();
        let plan_arc = Arc::clone(&processor.plan);

        // Poison the mutex.
        let plan_arc2 = Arc::clone(&plan_arc);
        let _ = std::thread::spawn(move || {
            let _guard = plan_arc2.lock().unwrap();
            panic!("intentional panic to poison mutex");
        })
        .join();

        // swap should recover and install the new plan.
        let graph = Graph::new();
        let plan = graph.compile().unwrap();
        handle.swap(plan); // Should not panic.

        // Verify the plan was actually installed by reading through the
        // (still poisoned) mutex.
        let inner = plan_arc.lock().unwrap_or_else(|e| e.into_inner());
        assert!(inner.is_some());
    }

    // --- Render failure logging tests ---

    #[test]
    fn test_short_interleaved_buffer_zero_fills() {
        // Create a player with 1 frame of 1-channel audio.
        // Request a 2-channel, 2-frame output => interleaved buffer will be
        // shorter than the output slice.
        let src = AudioBuffer::from_interleaved(vec![0.5], 1);
        let player_id = NodeId::next();

        let mut graph = Graph::new();
        graph.add_node(player_id, Box::new(FilePlayerNode::new(src, false)));

        let plan = graph.compile().unwrap();
        let mut processor = GraphProcessor::new();
        let handle = processor.swap_handle();
        handle.swap(plan);

        // Output is 2ch * 2frames = 4 samples, but the node produces 1ch * 1frame = 1 sample.
        // Actually the node produces output matching the AudioBuffer::new(channels, frames)
        // passed to it, so let's verify the behaviour is correct even when the last
        // node's output matches the requested size.
        let mut output = vec![1.0f32; 4];
        processor.process(&mut output, 2, 2);

        // The node produces a 2ch*2frame buffer. Frame 0 gets the mono sample
        // upmixed to both channels, frame 1 gets silence (past end).
        // So output should be [0.5, 0.5, 0.0, 0.0].
        assert!((output[0] - 0.5).abs() < 1e-6);
        assert!((output[1] - 0.5).abs() < 1e-6);
        assert_eq!(output[2], 0.0);
        assert_eq!(output[3], 0.0);
    }

    #[test]
    fn test_output_larger_than_rendered_fills_silence() {
        // Even if the output slice is larger than what any node produces,
        // the remainder should be zero-filled.
        let src = AudioBuffer::from_interleaved(vec![0.3, -0.3], 2);
        let player_id = NodeId::next();

        let mut graph = Graph::new();
        graph.add_node(player_id, Box::new(FilePlayerNode::new(src, false)));

        let plan = graph.compile().unwrap();
        let mut processor = GraphProcessor::new();
        let handle = processor.swap_handle();
        handle.swap(plan);

        // Process with buffer_frames=1 (matches source), but output is 6 samples
        // (larger than 2ch*1frame=2).
        let mut output = vec![9.9f32; 6];
        processor.process(&mut output, 2, 1);

        assert!((output[0] - 0.3).abs() < 1e-6);
        assert!((output[1] - -0.3).abs() < 1e-6);
        // Remaining should be zero-filled.
        for &s in &output[2..] {
            assert_eq!(s, 0.0);
        }
    }
}
