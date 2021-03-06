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

mod client;
use client::Client;
use common::{
    collector_proto::{
        control_flow_graph::{BasicBlock, Function},
        ControlFlowGraph, CreateFuzzerRequest, UpdateFeaturesRequest,
    },
    NO_SANCOV_INDEX,
};
use lazy_static::lazy_static;
use prost::Message;
use std::{
    collections::HashMap,
    env,
    sync::atomic::{AtomicU64, Ordering},
    sync::Mutex,
};

const SERVER_URL_ENV: &str = "FUZVISOR_SERVER_URL";
const DEFAULT_SERVER_URL: &str = "http://[::1]:2501";

#[repr(C)]
struct fuzzer_client_param_cfg_payload_data {
    buffer: *const u8,
    size: usize,
}

#[repr(C)]
struct fuzzer_client_param_cfg_remap_data {
    starts: *const u64,
    offsets: *const u64,
    size: usize,
}
#[repr(C)]
struct fuzzer_client_param_module {
    cfg_payload: fuzzer_client_param_cfg_payload_data,
    cfg_remap: fuzzer_client_param_cfg_remap_data,
}

#[repr(C)]
pub struct fuzzer_client_param {
    modules: *const fuzzer_client_param_module,
    modules_size: usize,
}

static FUZZER_ID: AtomicU64 = AtomicU64::new(std::u64::MAX);
lazy_static! {
    static ref SERVICE_CLIENT: Mutex<Client> = Mutex::new(Client::new(
        &env::var(SERVER_URL_ENV.to_owned()).unwrap_or(DEFAULT_SERVER_URL.to_owned())
    ));
}

#[no_mangle]
pub extern "C" fn fuzzer_client_init(param_ptr: *const fuzzer_client_param) {
    initialize_service_client();

    let modules = unsafe {
        let param = &*param_ptr;
        std::slice::from_raw_parts(param.modules, param.modules_size)
    };

    let cfgs: Vec<ControlFlowGraph> = modules
        .iter()
        .map(|module| unsafe {
            let remap_starts =
                std::slice::from_raw_parts(module.cfg_remap.starts, module.cfg_remap.size);
            let remap_offsets =
                std::slice::from_raw_parts(module.cfg_remap.offsets, module.cfg_remap.size);
            let cfg_payload =
                std::slice::from_raw_parts(module.cfg_payload.buffer, module.cfg_payload.size);
            let mut cfg = ControlFlowGraph::decode(cfg_payload).unwrap();
            remap_sancov_index(&mut cfg, &remap_starts, &remap_offsets);
            cfg
        })
        .collect();
    let concat_cfg = concat_control_flow_graph(cfgs);

    let id = SERVICE_CLIENT
        .lock()
        .unwrap()
        .call(|client| {
            client.create_fuzzer(CreateFuzzerRequest {
                cfg: Some(concat_cfg),
            })
        })
        .unwrap()
        .into_inner()
        .id;
    FUZZER_ID.store(id, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn fuzzer_client_update_features(features_ptr: *const u32, features_size: usize) {
    let features = unsafe { std::slice::from_raw_parts(features_ptr, features_size).to_vec() };
    SERVICE_CLIENT
        .lock()
        .unwrap()
        .call(|client| {
            client.update_features(UpdateFeaturesRequest {
                id: FUZZER_ID.load(Ordering::SeqCst),
                features,
            })
        })
        .unwrap();
}

fn initialize_service_client() {
    SERVICE_CLIENT.lock().unwrap().connect();
}

fn remap_sancov_index(cfg: &mut ControlFlowGraph, remap_starts: &[u64], remap_offsets: &[u64]) {
    for function in cfg.functions.iter_mut() {
        for basic_block in function.basic_blocks.iter_mut() {
            if basic_block.sancov_index == NO_SANCOV_INDEX {
                continue;
            }
            let sancov_index = basic_block.sancov_index;
            let index = match remap_starts.binary_search(&sancov_index) {
                Ok(index) => index,
                Err(index) => index - 1,
            };
            basic_block.sancov_index = remap_offsets[index] + (sancov_index - remap_starts[index]);
        }
    }
}

fn concat_control_flow_graph(cfgs: Vec<ControlFlowGraph>) -> ControlFlowGraph {
    let mut new_functions = Vec::new();
    let mut next_block_id = 0;
    for cfg in cfgs {
        let mut block_id_map = HashMap::new();
        let mut block_id_mapper = |id: &u64| {
            *block_id_map.entry(*id).or_insert_with(|| {
                let mapped_id = next_block_id;
                next_block_id += 1;
                mapped_id
            })
        };
        for function in cfg.functions {
            let mut new_basic_blocks = Vec::new();
            for basic_block in function.basic_blocks {
                new_basic_blocks.push(BasicBlock {
                    id: block_id_mapper(&basic_block.id),
                    successors: basic_block
                        .successors
                        .iter()
                        .map(|successor| block_id_mapper(successor))
                        .collect(),
                    sancov_index: basic_block.sancov_index,
                });
            }
            new_functions.push(Function {
                id: new_functions.len() as u64,
                name: function.name,
                basic_blocks: new_basic_blocks,
            });
        }
    }
    ControlFlowGraph {
        functions: new_functions,
    }
}
