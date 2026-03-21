//! Audio graph — re-exported from dhvani.

pub use dhvani::graph::{
    AudioNode, Connection, ExecutionPlan, Graph, GraphProcessor, GraphSwapHandle, NodeId,
};

#[cfg(test)]
mod tests {
    use super::*;
    use dhvani::buffer::AudioBuffer;

    struct SilenceNode;

    impl AudioNode for SilenceNode {
        fn name(&self) -> &str {
            "silence"
        }
        fn num_inputs(&self) -> usize {
            0
        }
        fn num_outputs(&self) -> usize {
            1
        }
        fn process(&mut self, _inputs: &[&AudioBuffer], _output: &mut AudioBuffer) {}
    }

    #[test]
    fn node_id_unique() {
        let a = NodeId::next();
        let b = NodeId::next();
        assert_ne!(a, b);
    }

    #[test]
    fn graph_add_node_and_compile() {
        let mut graph = Graph::new();
        let id = NodeId::next();
        graph.add_node(id, Box::new(SilenceNode));
        assert_eq!(graph.node_count(), 1);

        let plan = graph.compile().unwrap();
        assert_eq!(plan.order().len(), 1);
    }

    #[test]
    fn graph_connect_and_compile() {
        let mut graph = Graph::new();
        let a = NodeId::next();
        let b = NodeId::next();
        graph.add_node(a, Box::new(SilenceNode));
        graph.add_node(b, Box::new(SilenceNode));
        graph.connect(a, b);
        assert_eq!(graph.connection_count(), 1);

        let plan = graph.compile().unwrap();
        assert_eq!(plan.order().len(), 2);
        // a should come before b in topological order
        let order = plan.order();
        let pos_a = order.iter().position(|&id| id == a).unwrap();
        let pos_b = order.iter().position(|&id| id == b).unwrap();
        assert!(pos_a < pos_b);
    }

    #[test]
    fn graph_processor_creation() {
        let processor = GraphProcessor::new(2, 48000, 256);
        let _handle = processor.swap_handle();
    }
}
