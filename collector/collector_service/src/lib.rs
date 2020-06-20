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
use common::collector_proto::{
    collector_service_server::CollectorService, collector_service_server::CollectorServiceServer,
    CreateFuzzerRequest, CreateFuzzerResponse, UpdateFeaturesRequest, UpdateFeaturesResponse,
};
use fuzzer::Fuzzer;
pub use fuzzer::Node;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tonic::{Request, Response, Status};

pub trait Observer {
    fn create_fuzzer(&mut self, fuzzer_id: u64, graph: &[Node]);

    fn update_nodes(&mut self, fuzzer_id: u64, bit_counters: &[(usize, u8)]);
}

pub type ObserverPtr = Arc<Mutex<dyn Observer + Send>>;

pub struct CollectorServiceImpl {
    fuzzer_map: Mutex<HashMap<u64, Fuzzer>>,
    observer: ObserverPtr,
}

#[tonic::async_trait]
impl CollectorService for CollectorServiceImpl {
    async fn create_fuzzer(
        &self,
        req: Request<CreateFuzzerRequest>,
    ) -> Result<Response<CreateFuzzerResponse>, Status> {
        let create_fuzzer_req = req.into_inner();
        let fuzzer = Fuzzer::new(create_fuzzer_req.cfg.unwrap());
        let mut fuzzer_map = self.fuzzer_map.lock().unwrap();
        let fuzzer_id = fuzzer_map.len() as u64;
        self.observer
            .lock()
            .unwrap()
            .create_fuzzer(fuzzer_id, fuzzer.get_nodes());
        fuzzer_map.insert(fuzzer_id, fuzzer);
        Ok(Response::new(CreateFuzzerResponse { id: fuzzer_id }))
    }

    async fn update_features(
        &self,
        req: Request<UpdateFeaturesRequest>,
    ) -> Result<Response<UpdateFeaturesResponse>, Status> {
        let update_feature_req = req.into_inner();
        let fuzzer_id = update_feature_req.id;
        let features = update_feature_req.features;
        let hit_bit_counters = self
            .fuzzer_map
            .lock()
            .unwrap()
            .get_mut(&fuzzer_id)
            .unwrap()
            .update_features(&features);
        self.observer
            .lock()
            .unwrap()
            .update_nodes(fuzzer_id, &hit_bit_counters);
        Ok(Response::new(UpdateFeaturesResponse {}))
    }
}

pub fn create_service(observer: ObserverPtr) -> CollectorServiceServer<CollectorServiceImpl> {
    CollectorServiceServer::new(CollectorServiceImpl {
        fuzzer_map: Mutex::new(HashMap::new()),
        observer,
    })
}
