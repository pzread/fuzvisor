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

#[path = "../common/lib.rs"]
mod common;
mod fuzzer;
use common::collector_proto::{
    collector_service_server::{CollectorService, CollectorServiceServer},
    CreateFuzzerRequest, CreateFuzzerResponse, UpdateFeaturesRequest, UpdateFeaturesResponse,
};
use fuzzer::Fuzzer;
use std::{collections::HashMap, sync::Mutex};
use tonic::{transport::Server, Request, Response, Status};

struct CollectorServiceImpl {
    fuzzer_map: Mutex<HashMap<u64, Fuzzer>>,
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

        self.fuzzer_map
            .lock()
            .unwrap()
            .get_mut(&fuzzer_id)
            .unwrap()
            .update_features(&features);

        Ok(Response::new(UpdateFeaturesResponse {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:2501".parse().unwrap();
    println!("CollectorService listening on {}.", addr);
    Server::builder()
        .add_service(CollectorServiceServer::new(CollectorServiceImpl {
            fuzzer_map: Mutex::new(HashMap::new()),
        }))
        .serve(addr)
        .await?;
    Ok(())
}
