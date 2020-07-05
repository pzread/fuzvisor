// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use common::{
    collector_proto::ControlFlowGraph,
    observer_proto::{structure_graph::Node as GraphNode, StructureGraph},
    NO_SANCOV_INDEX,
};
use std::{cmp, collections::HashMap, iter};

#[derive(Clone)]
struct Node {
    bit_counter: u8,
}

pub struct Fuzzer {
    nodes: Vec<Node>,
    sancov_index_map: HashMap<u32, usize>,
    sancov_edge_dict: HashMap<u32, Vec<(u32, Vec<usize>)>>,
}

impl Fuzzer {
    pub fn new(struct_graph: &StructureGraph, cfg: ControlFlowGraph) -> Self {
        let node_sancov_map: HashMap<usize, u32> = cfg
            .functions
            .iter()
            .map(|function| function.basic_blocks.iter())
            .flatten()
            .filter_map(|block| match block.sancov_index {
                NO_SANCOV_INDEX => None,
                sancov_index => Some((block.id as usize, sancov_index as u32)),
            })
            .collect();
        Self {
            nodes: iter::repeat(Node { bit_counter: 0 })
                .take(struct_graph.nodes.len())
                .collect(),
            sancov_index_map: node_sancov_map
                .iter()
                .map(|(node_index, sancov_index)| (*sancov_index, *node_index))
                .collect(),
            sancov_edge_dict: Self::build_sancov_edge_dict(&struct_graph.nodes, &node_sancov_map),
        }
    }

    pub fn update_features(&mut self, features: &[u32]) -> Vec<(usize, u8)> {
        let mut covered_sancov_indices: HashMap<u32, u8> = HashMap::new();
        for feature in features {
            let sancov_index = feature / 8;
            if self.sancov_index_map.contains_key(&sancov_index) {
                let bit_counter = covered_sancov_indices.entry(sancov_index).or_default();
                *bit_counter |= 1 << (feature % 8);
            }
        }
        let mut hit_bit_counters: HashMap<usize, u8> = HashMap::new();
        for (sancov_index, bit_counter) in covered_sancov_indices.iter() {
            if let Some(edges) = self.sancov_edge_dict.get(&sancov_index) {
                for (dst, covered_nodes) in edges {
                    if !covered_sancov_indices.contains_key(dst) {
                        continue;
                    }
                    for node_index in covered_nodes {
                        let updated_bit_counter = self.nodes[*node_index].bit_counter | bit_counter;
                        self.nodes[*node_index].bit_counter = updated_bit_counter;
                        hit_bit_counters.insert(*node_index, updated_bit_counter);
                    }
                }
            }
        }
        hit_bit_counters.into_iter().collect()
    }

    fn build_sancov_edge_dict(
        graph_nodes: &[GraphNode],
        node_sancov_map: &HashMap<usize, u32>,
    ) -> HashMap<u32, Vec<(u32, Vec<usize>)>> {
        let mut sancov_edge_map: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        let mut visiting_map = vec![NO_SANCOV_INDEX; graph_nodes.len()];
        for node_index in 0..graph_nodes.len() {
            let source_sancov_index = match node_sancov_map.get(&node_index) {
                Some(sancov_index) => sancov_index,
                None => continue,
            };
            let mut path = Vec::new();
            Self::path_traverse(
                graph_nodes,
                node_sancov_map,
                node_index,
                *source_sancov_index,
                &mut path,
                &mut sancov_edge_map,
                &mut visiting_map,
            );
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
        graph_nodes: &[GraphNode],
        node_sancov_map: &HashMap<usize, u32>,
        node_index: usize,
        source_sancov_index: u32,
        path: &mut Vec<usize>,
        sancov_edge_map: &mut HashMap<(u32, u32), Vec<usize>>,
        visiting_map: &mut Vec<u64>,
    ) {
        path.push(node_index);
        let sancov_index = node_sancov_map.get(&node_index).copied();
        if (path.len() > 1 && sancov_index.is_some())
            || graph_nodes[node_index].successors.is_empty()
            || visiting_map[node_index] == source_sancov_index as u64
        {
            let sancov_index = sancov_index.unwrap_or(source_sancov_index);
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
            for successor in graph_nodes[node_index].successors.iter() {
                Self::path_traverse(
                    graph_nodes,
                    node_sancov_map,
                    *successor as usize,
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
