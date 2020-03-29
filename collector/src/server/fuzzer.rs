use super::common::{
    fuzzmon_proto::{control_flow_graph::BasicBlock, ControlFlowGraph},
    NO_SANCOV_INDEX,
};
use std::{cmp, collections::HashMap};

struct Node {
    predecessors: Vec<usize>,
    successors: Vec<usize>,
    bit_counter: u8,
    sancov_index: Option<u32>,
}

pub struct Fuzzer {
    nodes: Vec<Node>,
    sancov_index_map: HashMap<u32, usize>,
    sancov_edge_dict: HashMap<u32, Vec<(u32, Vec<usize>)>>,
}

impl Fuzzer {
    pub fn new(cfg: ControlFlowGraph) -> Self {
        let mut blocks: Vec<&BasicBlock> = cfg
            .functions
            .iter()
            .flat_map(|function| function.basic_blocks.iter())
            .collect();
        blocks.sort_by_key(|block| block.id);

        let mut nodes: Vec<Node> = blocks
            .iter()
            .map(|block| Node {
                predecessors: Vec::new(),
                successors: {
                    let mut successors: Vec<usize> =
                        block.successors.iter().map(|succ| *succ as usize).collect();
                    successors.dedup();
                    successors
                },
                bit_counter: 0,
                sancov_index: match block.sancov_index {
                    NO_SANCOV_INDEX => None,
                    sancov_index => Some(sancov_index as u32),
                },
            })
            .collect();
        for node_index in 0..nodes.len() {
            let successors = nodes[node_index].successors.clone();
            for successor in successors {
                nodes[successor].predecessors.push(node_index);
            }
        }

        let sancov_index_map: HashMap<u32, usize> = nodes
            .iter()
            .enumerate()
            .filter_map(|(node_index, node)| {
                node.sancov_index
                    .map(|sancov_index| (sancov_index, node_index))
            })
            .collect();
        let sancov_edge_dict = Self::build_sancov_edge_dict(&nodes);

        Self {
            nodes,
            sancov_index_map,
            sancov_edge_dict,
        }
    }

    pub fn update_features(&mut self, features: &[u32]) {
        let mut covered_sancov_indices: HashMap<u32, u8> = HashMap::new();
        for feature in features {
            let sancov_index = feature / 8;
            if self.sancov_index_map.contains_key(&sancov_index) {
                let bit_counter = covered_sancov_indices.entry(sancov_index).or_default();
                *bit_counter |= 1 << (feature % 8);
            }
        }
        for (sancov_index, bit_counter) in covered_sancov_indices.iter() {
            if let Some(edges) = self.sancov_edge_dict.get(&sancov_index) {
                for (dst, covered_nodes) in edges {
                    if !covered_sancov_indices.contains_key(dst) {
                        continue;
                    }
                    for node_index in covered_nodes {
                        self.nodes[*node_index].bit_counter |= bit_counter;
                    }
                }
            }
        }
    }

    fn build_sancov_edge_dict(nodes: &[Node]) -> HashMap<u32, Vec<(u32, Vec<usize>)>> {
        let mut sancov_edge_map: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        let mut visiting_map = vec![NO_SANCOV_INDEX; nodes.len()];
        for (node_index, node) in nodes.iter().enumerate() {
            if let Some(source_sancov_index) = node.sancov_index {
                let mut path = Vec::new();
                Self::path_traverse(
                    nodes,
                    node_index,
                    source_sancov_index,
                    &mut path,
                    &mut sancov_edge_map,
                    &mut visiting_map,
                );
            } else {
                assert!(!node.predecessors.is_empty());
            }
        }
        let mut sancov_edge_dict: HashMap<u32, Vec<(u32, Vec<usize>)>> = HashMap::new();
        for ((src, dst), covered_nodes) in sancov_edge_map {
            sancov_edge_dict
                .entry(src)
                .or_default()
                .push((dst, covered_nodes));
        }
        sancov_edge_dict
    }

    fn path_traverse(
        nodes: &[Node],
        node_index: usize,
        source_sancov_index: u32,
        path: &mut Vec<usize>,
        sancov_edge_map: &mut HashMap<(u32, u32), Vec<usize>>,
        visiting_map: &mut Vec<u64>,
    ) {
        let node = &nodes[node_index];
        path.push(node_index);
        if (path.len() > 1 && node.sancov_index.is_some())
            || node.successors.is_empty()
            || visiting_map[node_index] == source_sancov_index as u64
        {
            let sancov_index = node.sancov_index.unwrap_or(source_sancov_index);
            let edge_path = sancov_edge_map
                .entry((
                    cmp::min(source_sancov_index, sancov_index),
                    cmp::max(source_sancov_index, sancov_index),
                ))
                .or_default();
            edge_path.extend_from_slice(&path);
            edge_path.dedup();
        } else {
            visiting_map[node_index] = source_sancov_index as u64;
            for successor in node.successors.iter() {
                Self::path_traverse(
                    nodes,
                    *successor,
                    source_sancov_index,
                    path,
                    sancov_edge_map,
                    visiting_map,
                );
            }
            visiting_map[node_index] = NO_SANCOV_INDEX;
        }
        path.pop();
    }
}
