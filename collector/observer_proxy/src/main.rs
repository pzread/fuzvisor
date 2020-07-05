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
use clap::Arg;
use common::observer_proto::{
    observer_service_client::ObserverServiceClient, update_features_request::BitCounter,
    CreateFuzzerRequest, StructureGraph, UpdateFeaturesRequest,
};
use std::u64;
use tokio::sync::Mutex;
use tonic::transport::Server;

struct Proxy {
    client: Mutex<ObserverServiceClient<tonic::transport::channel::Channel>>,
}

#[async_trait]
impl collector_service::Observer for Proxy {
    async fn create_fuzzer(&self, fuzzer_id: u64, struct_graph: &StructureGraph) {
        let req = CreateFuzzerRequest {
            fuzzer_id,
            structure_graph: Some(struct_graph.clone()),
        };
        self.client.lock().await.create_fuzzer(req).await.unwrap();
    }

    async fn update_features(
        &self,
        fuzzer_id: u64,
        bit_counters: &[(usize, u8)],
        corpus_id: Option<u64>,
    ) -> Vec<(u64, u32)> {
        let req = UpdateFeaturesRequest {
            fuzzer_id,
            bit_counters: bit_counters
                .iter()
                .map(|&(node_index, counter)| BitCounter {
                    node_index: node_index as u64,
                    counter: counter as u32,
                })
                .collect(),
            corpus_id: corpus_id.unwrap_or(u64::MAX),
        };
        let res = self
            .client
            .lock()
            .await
            .update_features(req)
            .await
            .unwrap()
            .into_inner();
        res.corpus_priorities
            .into_iter()
            .map(|corpus_priority| (corpus_priority.id, corpus_priority.priority))
            .collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = clap::App::new("Observer Proxy")
        .arg(
            Arg::with_name("listen_addr")
                .takes_value(true)
                .required(true)
                .long("listen_addr")
                .help("Set collector listening address:port"),
        )
        .arg(
            Arg::with_name("observer_url")
                .takes_value(true)
                .required(true)
                .long("observer_url")
                .help("Set observer server url"),
        )
        .get_matches();

    let observer_url = args.value_of("observer_url").unwrap().to_owned();
    let client = ObserverServiceClient::connect(observer_url).await.unwrap();

    let observer_ptr = Box::new(Proxy {
        client: Mutex::new(client),
    });

    let addr = args.value_of("listen_addr").unwrap().parse().unwrap();
    println!("Observer Proxy listening on {}.", addr);
    Server::builder()
        .add_service(collector_service::create_service(observer_ptr))
        .serve(addr)
        .await?;
    Ok(())
}
