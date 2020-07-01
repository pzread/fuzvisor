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

use async_trait::async_trait;
use common::observer_proto::{structure_graph::Node as GraphNode, StructureGraph};
use std::{collections::HashSet, sync::Mutex};
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

struct ObserverInner {
    nodes: Vec<Node>,
    functions: Vec<Function>,
    frontiers: HashSet<usize>,
    covered_nodes: usize,
    covered_functions: usize,
}

struct Observer {
    inner: Mutex<ObserverInner>,
}

impl ObserverInner {
    fn create_fuzzer(&mut self, fuzzer_id: u64, struct_graph: &StructureGraph) {
        if fuzzer_id != 0 {
            return;
        }
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
    }

    fn update_features(
        &mut self,
        bit_counters: &[(usize, u8)],
        corpus_id: Option<u64>,
    ) -> Vec<(u64, u32)> {
        let mut new_update = false;
        let mut new_function_names = Vec::new();
        for &(node_index, _) in bit_counters {
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
                self.frontiers.insert(node_index);
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
        if new_update {
            println!(
                "Covered Nodes: {} ({}) / Total Nodes: {} ({}) / Frontiers: {} / Corpus ID: {}",
                self.covered_nodes,
                self.covered_functions,
                self.nodes.len(),
                self.functions.len(),
                self.frontiers.len(),
                corpus_id.unwrap(),
            );
        }
        if !new_function_names.is_empty() {
            println!("New Functions: {:?}", new_function_names);
        }
        Vec::new()
    }
}

#[async_trait]
impl collector_service::Observer for Observer {
    async fn create_fuzzer(&self, fuzzer_id: u64, struct_graph: &StructureGraph) {
        self.inner
            .lock()
            .unwrap()
            .create_fuzzer(fuzzer_id, struct_graph);
    }

    async fn update_features(
        &self,
        _fuzzer_id: u64,
        bit_counters: &[(usize, u8)],
        corpus_id: Option<u64>,
    ) -> Vec<(u64, u32)> {
        self.inner
            .lock()
            .unwrap()
            .update_features(bit_counters, corpus_id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:2501".parse().unwrap();
    println!("Collector Service listening on {}.", addr);
    let observer_ptr = Box::new(Observer {
        inner: Mutex::new(ObserverInner {
            nodes: Vec::new(),
            functions: Vec::new(),
            frontiers: HashSet::new(),
            covered_nodes: 0,
            covered_functions: 0,
        }),
    });
    Server::builder()
        .add_service(collector_service::create_service(observer_ptr))
        .serve(addr)
        .await?;
    Ok(())
}
