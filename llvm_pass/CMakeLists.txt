# Copyright 2020 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

cmake_minimum_required(VERSION 3.10)
find_package(Protobuf CONFIG REQUIRED)
find_package(LLVM 10.0 CONFIG REQUIRED)

project(llvm_pass)

add_definitions(${LLVM_DEFINITIONS})
include_directories(${LLVM_INCLUDE_DIRS})
add_compile_options(-DGOOGLE_PROTOBUF_NO_RTTI -fno-rtti)

add_library(LLVMCollector MODULE
  CollectorPass.cpp
  ../proto/control_flow_graph.proto)
file(MAKE_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/proto)
include_directories(LLVMCollector ${CMAKE_CURRENT_BINARY_DIR}/proto)
target_link_libraries(LLVMCollector protobuf::libprotobuf)
protobuf_generate(TARGET LLVMCollector
  APPEND_PATH
  PROTOC_OUT_DIR ${CMAKE_CURRENT_BINARY_DIR}/proto
)
