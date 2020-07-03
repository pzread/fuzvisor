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

use common::collector_proto::{structure_graph::Node as GraphNode, StructureGraph};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tonic::transport::Server;

struct Node {
    graph_node: GraphNode,
    function_index: usize,
    covered: bool,
    uncovered_successors: usize,
}

struct Function {
    name: String,
    covered_nodes: usize,
}

struct Observer {
    nodes: Vec<Node>,
    freqs: Vec<usize>,
    functions: Vec<Function>,
    frontiers: HashMap<usize, Vec<(u64, u64)>>,
    covered_nodes: usize,
    covered_functions: usize,
    prio_corpuses: HashMap<u64, HashSet<u64>>,
}

impl collector_service::Observer for Observer {
    fn create_fuzzer(&mut self, fuzzer_id: u64, struct_graph: &StructureGraph) {
        if fuzzer_id == 0 {
            self.functions = struct_graph
                .functions
                .iter()
                .map(|graph_function| Function {
                    name: graph_function.name.clone(),
                    covered_nodes: 0,
                })
                .collect();
            let mut nodes: Vec<Node> = struct_graph
                .nodes
                .iter()
                .map(|graph_node| Node {
                    graph_node: graph_node.clone(),
                    function_index: 0,
                    covered: false,
                    uncovered_successors: graph_node.successors.len(),
                })
                .collect();
            for (function_index, graph_function) in struct_graph.functions.iter().enumerate() {
                for node_index in graph_function.node_indices.iter() {
                    nodes[*node_index as usize].function_index = function_index;
                }
            }
            self.nodes = nodes;
            self.freqs = vec![0; struct_graph.nodes.len()];
        }
    }

    fn update_nodes(
        &mut self,
        fuzzer_id: u64,
        bit_counters: &[(usize, u8)],
        corpus_id: Option<u64>,
    ) -> Vec<(u64, u32)> {
        let mut new_update = false;
        let mut new_function_names = Vec::new();
        for &(node_index, _) in bit_counters {
            self.freqs[node_index] += 1;

            let node = &mut self.nodes[node_index];
            if node.covered {
                continue;
            }
            node.covered = true;
            self.covered_nodes += 1;

            let function = &mut self.functions[node.function_index];
            function.covered_nodes += 1;
            if function.covered_nodes == 1 {
                self.covered_functions += 1;
                new_function_names.push(function.name.clone());
            }

            if node.uncovered_successors > 0 {
                self.frontiers.insert(node_index, Vec::new());
            }
            for pred_index in 0..node.graph_node.predecessors.len() {
                let predecessor =
                    self.nodes[node_index].graph_node.predecessors[pred_index] as usize;
                let pred_node = &mut self.nodes[predecessor];
                pred_node.uncovered_successors -= 1;
                if pred_node.uncovered_successors == 0 {
                    self.frontiers.remove(&predecessor);
                }
            }
            new_update = true;
        }
        if let Some(corpus_id) = corpus_id {
            for &(node_index, _) in bit_counters {
                let frontier = match self.frontiers.get_mut(&node_index) {
                    Some(frontier) => frontier,
                    None => continue,
                };
                frontier.push((fuzzer_id, corpus_id));
            }
        }

        let mut hist = Vec::new();
        let mut freq_denominator = 1;
        for (&node_index, frontier) in self.frontiers.iter() {
            hist.push((self.freqs[node_index], frontier.as_slice()));
            freq_denominator = freq_denominator.max(self.freqs[node_index]);
        }
        hist.sort_by_key(|&(freq, _)| freq);
        let hist: Vec<(f64, &[(u64, u64)])> = hist
            .into_iter()
            .map(|(freq, frontier)| (freq as f64 / freq_denominator as f64, frontier))
            .collect();
        let hist_list: Vec<f64> = hist
            .chunks((hist.len() + 9) / 10)
            .map(|chunk| chunk.last().unwrap().0)
            .collect();

        let prio_corpus_ids = self.prio_corpuses.entry(fuzzer_id).or_default();
        let mut new_prio_corpuses: HashMap<u64, u32> = prio_corpus_ids
            .iter()
            .map(|&corpus_id| (corpus_id, 1))
            .collect();
        for (freq, frontier) in hist {
            for &(corpus_fuzzer_id, corpus_id) in frontier {
                if corpus_fuzzer_id != fuzzer_id {
                    continue;
                }
                let weight = new_prio_corpuses.entry(corpus_id).or_insert(1);
                *weight = (*weight).max((1.0 / freq.min(0.001)) as u32);
            }
        }
        *prio_corpus_ids = new_prio_corpuses
            .iter()
            .filter_map(|(&corpus_id, &weight)| if weight != 1 { Some(corpus_id) } else { None })
            .collect();

        if new_update || !new_prio_corpuses.is_empty() {
            println!("{:.3?}", hist_list);
            println!(
                "Covered Nodes: {} ({}) / Total Nodes: {} ({}) / Frontiers: {} / Update Prios: {}",
                self.covered_nodes,
                self.covered_functions,
                self.nodes.len(),
                self.functions.len(),
                self.frontiers.len(),
                new_prio_corpuses.len(),
            );
        }
        if !new_function_names.is_empty() {
            println!("New Functions: {:?}", new_function_names);
        }
        new_prio_corpuses.into_iter().collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:2501".parse().unwrap();
    println!("CollectorService listening on {}.", addr);
    let observer_ptr = Arc::new(Mutex::new(Observer {
        nodes: Vec::new(),
        freqs: Vec::new(),
        functions: Vec::new(),
        frontiers: HashMap::new(),
        covered_nodes: 0,
        covered_functions: 0,
        prio_corpuses: HashMap::new(),
    }));
    Server::builder()
        .add_service(collector_service::create_service(observer_ptr))
        .serve(addr)
        .await?;
    Ok(())
}
