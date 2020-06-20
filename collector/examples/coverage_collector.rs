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

use std::sync::{Arc, Mutex};
use tonic::transport::Server;

struct Observer {
    nodes: Vec<bool>,
    covered_num: usize,
}

impl collector_service::Observer for Observer {
    fn create_fuzzer(&mut self, fuzzer_id: u64, graph: &[collector_service::Node]) {
        if fuzzer_id == 0 {
            self.nodes = vec![false; graph.len()];
        }
    }

    fn update_nodes(&mut self, _fuzzer_id: u64, bit_counters: &[(usize, u8)]) {
        for (node_index, _) in bit_counters {
            if !self.nodes[*node_index] {
                self.nodes[*node_index] = true;
                self.covered_num += 1;
            }
        }
        println!(
            "Covered: {} / Total: {}",
            self.covered_num,
            self.nodes.len()
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:2501".parse().unwrap();
    println!("CollectorService listening on {}.", addr);
    let observer_ptr = Arc::new(Mutex::new(Observer {
        nodes: Vec::new(),
        covered_num: 0,
    }));
    Server::builder()
        .add_service(collector_service::create_service(observer_ptr))
        .serve(addr)
        .await?;
    Ok(())
}
