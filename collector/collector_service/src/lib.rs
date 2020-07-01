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

mod fuzzer;
use async_trait::async_trait;
use common::{
    collector_proto::{
        collector_service_server::CollectorService,
        collector_service_server::CollectorServiceServer, ControlFlowGraph, CreateFuzzerRequest,
        CreateFuzzerResponse, UpdateFeaturesRequest, UpdateFeaturesResponse,
    },
    observer_proto::{
        structure_graph::Function as GraphFunction, structure_graph::Node as GraphNode,
        StructureGraph,
    },
    NO_CORPUS_ID,
};
use fuzzer::Fuzzer;
use std::{collections::HashMap, sync::Mutex};
use tonic::{Request, Response, Status};

#[async_trait]
pub trait Observer {
    async fn create_fuzzer(&self, fuzzer_id: u64, struct_graph: &StructureGraph);

    async fn update_features(
        &self,
        fuzzer_id: u64,
        bit_counters: &[(usize, u8)],
        corpus_id: Option<u64>,
    );
}

pub type ObserverPtr = Box<dyn Observer + Sync + Send>;

pub struct CollectorServiceImpl {
    fuzzer_map: Mutex<HashMap<u64, Fuzzer>>,
    observer: ObserverPtr,
}

#[async_trait]
impl CollectorService for CollectorServiceImpl {
    async fn create_fuzzer(
        &self,
        req: Request<CreateFuzzerRequest>,
    ) -> Result<Response<CreateFuzzerResponse>, Status> {
        let create_fuzzer_req = req.into_inner();
        let cfg = create_fuzzer_req.cfg.unwrap();

        let struct_graph = build_structure_graph(&cfg);
        let fuzzer = Fuzzer::new(&struct_graph, cfg);
        let fuzzer_id = {
            let mut fuzzer_map = self.fuzzer_map.lock().unwrap();
            let fuzzer_id = fuzzer_map.len() as u64;
            fuzzer_map.insert(fuzzer_id, fuzzer);
            fuzzer_id
        };
        self.observer.create_fuzzer(fuzzer_id, &struct_graph).await;

        Ok(Response::new(CreateFuzzerResponse { id: fuzzer_id }))
    }

    async fn update_features(
        &self,
        req: Request<UpdateFeaturesRequest>,
    ) -> Result<Response<UpdateFeaturesResponse>, Status> {
        let update_feature_req = req.into_inner();
        let fuzzer_id = update_feature_req.id;
        let features = update_feature_req.features;
        let corpus_id = match update_feature_req.corpus_id {
            NO_CORPUS_ID => None,
            corpus_id => Some(corpus_id),
        };

        let hit_bit_counters = self
            .fuzzer_map
            .lock()
            .unwrap()
            .get_mut(&fuzzer_id)
            .unwrap()
            .update_features(&features);
        self.observer
            .update_features(fuzzer_id, &hit_bit_counters, corpus_id)
            .await;

        Ok(Response::new(UpdateFeaturesResponse {
            corpus_priorities: Vec::new(),
        }))
    }
}

pub fn create_service(observer: ObserverPtr) -> CollectorServiceServer<CollectorServiceImpl> {
    CollectorServiceServer::new(CollectorServiceImpl {
        fuzzer_map: Mutex::new(HashMap::new()),
        observer,
    })
}

fn build_structure_graph(cfg: &ControlFlowGraph) -> StructureGraph {
    let mut node_pairs = Vec::new();
    let mut functions = Vec::new();
    for cfg_function in cfg.functions.iter() {
        let mut node_indices = Vec::new();
        for cfg_block in cfg_function.basic_blocks.iter() {
            let node_index = cfg_block.id;
            node_indices.push(node_index);
            node_pairs.push((
                node_index,
                GraphNode {
                    predecessors: Vec::new(),
                    successors: {
                        let mut successors = cfg_block.successors.clone();
                        successors.dedup();
                        successors
                    },
                },
            ))
        }
        functions.push(GraphFunction {
            name: cfg_function.name.clone(),
            node_indices,
        })
    }
    node_pairs.sort_by_key(|(node_index, _)| *node_index);
    let mut nodes: Vec<GraphNode> = node_pairs
        .into_iter()
        .map(|(_, graph_node)| graph_node)
        .collect();
    for node_index in 0..nodes.len() {
        for successor in nodes[node_index].successors.clone() {
            nodes[successor as usize]
                .predecessors
                .push(node_index as u64);
        }
    }
    StructureGraph { nodes, functions }
}
