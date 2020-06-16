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

use super::common::fuzzmon_proto::collector_service_client::CollectorServiceClient;
use std::{future::Future, mem::MaybeUninit};
use tokio::runtime;

pub struct Client {
    runtime: runtime::Runtime,
    client: MaybeUninit<CollectorServiceClient<tonic::transport::channel::Channel>>,
}

impl Client {
    pub fn new() -> Self {
        let runtime = runtime::Builder::new()
            .basic_scheduler()
            .core_threads(1)
            .enable_all()
            .build()
            .unwrap();
        Self {
            runtime,
            client: MaybeUninit::zeroed(),
        }
    }

    pub fn connect(&mut self) {
        let client = self
            .runtime
            .block_on(async { CollectorServiceClient::connect("http://[::1]:2501").await })
            .unwrap();
        self.client = MaybeUninit::new(client);
    }

    pub fn call<'a, T, F>(&'a mut self, f: T) -> F::Output
    where
        F: Future + 'a,
        T: FnOnce(&'a mut CollectorServiceClient<tonic::transport::channel::Channel>) -> F,
    {
        self.runtime
            .block_on(f(unsafe { &mut *self.client.as_mut_ptr() }))
    }
}
