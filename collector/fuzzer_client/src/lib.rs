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
        update_features_response::CorpusPriority,
        ControlFlowGraph, CreateFuzzerRequest, UpdateFeaturesRequest,
    },
    NO_CORPUS_ID, NO_SANCOV_INDEX,
};
use lazy_static::lazy_static;
use prost::Message;
use std::{
    cell::RefCell,
    collections::HashMap,
    env, slice,
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

#[repr(C)]
pub struct fuzzer_client_corpus_priority {
    index: usize,
    priority: u32,
}

static FUZZER_ID: AtomicU64 = AtomicU64::new(std::u64::MAX);
lazy_static! {
    static ref SERVICE_CLIENT: Mutex<Client> = Mutex::new(Client::new(
        &env::var(SERVER_URL_ENV.to_owned()).unwrap_or(DEFAULT_SERVER_URL.to_owned())
    ));
}

thread_local!(static PENDING_CORPUS_PRIORITIES: RefCell<Vec<CorpusPriority>> = RefCell::new(Vec::new()));

#[no_mangle]
pub extern "C" fn fuzzer_client_init(param_ptr: *const fuzzer_client_param) {
    initialize_service_client();

    let modules = unsafe {
        let param = &*param_ptr;
        slice::from_raw_parts(param.modules, param.modules_size)
    };

    let cfgs: Vec<ControlFlowGraph> = modules
        .iter()
        .map(|module| unsafe {
            let remap_starts =
                slice::from_raw_parts(module.cfg_remap.starts, module.cfg_remap.size);
            let remap_offsets =
                slice::from_raw_parts(module.cfg_remap.offsets, module.cfg_remap.size);
            let cfg_payload =
                slice::from_raw_parts(module.cfg_payload.buffer, module.cfg_payload.size);
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
pub extern "C" fn fuzzer_client_update_features(
    features_ptr: *const u32,
    features_size: usize,
    corpus_index: usize,
) {
    let features = unsafe { slice::from_raw_parts(features_ptr, features_size).to_vec() };
    let response = SERVICE_CLIENT
        .lock()
        .unwrap()
        .call(|client| {
            client.update_features(UpdateFeaturesRequest {
                id: FUZZER_ID.load(Ordering::SeqCst),
                features,
                corpus_id: match corpus_index {
                    std::usize::MAX => NO_CORPUS_ID,
                    index => index as u64,
                },
            })
        })
        .unwrap()
        .into_inner();
    PENDING_CORPUS_PRIORITIES.with(|pending_corpus_priorities| {
        *pending_corpus_priorities.borrow_mut() = response.corpus_priorities;
    });
}

#[no_mangle]
pub extern "C" fn fuzzer_client_get_corpus_priorities(
    buffer_ptr: *mut fuzzer_client_corpus_priority,
    buffer_size: usize,
) -> usize {
    PENDING_CORPUS_PRIORITIES.with(|pending_corpus_priorities| {
        let mut pending_corpus_priorities = pending_corpus_priorities.borrow_mut();
        let priority_length = pending_corpus_priorities.len();
        if buffer_size < priority_length {
            return priority_length;
        }

        let buffer = unsafe { slice::from_raw_parts_mut(buffer_ptr, priority_length) };
        for (slot, corpus_priority) in buffer.iter_mut().zip(pending_corpus_priorities.iter()) {
            slot.index = corpus_priority.id as usize;
            slot.priority = corpus_priority.priority;
        }
        pending_corpus_priorities.clear();
        priority_length
    })
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
