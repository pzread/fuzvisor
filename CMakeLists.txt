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
include(ExternalProject)

project(fuzzmon)

set(LLVM_PASS_BUILD_DIR
  ${CMAKE_CURRENT_BINARY_DIR}/llvm_pass/src/llvm_pass-build)
set(FUZZMON_COLLECTOR_PASS_PATH
  ${LLVM_PASS_BUILD_DIR}/libLLVMFuzzmonCollector.so)

add_subdirectory(collector)
add_subdirectory(third_party)

get_property(LLVM_BUILD_DIR TARGET llvm PROPERTY BUILD_DIR)
get_property(PROTOBUF_BUILD_DIR TARGET protobuf PROPERTY BUILD_DIR)

ExternalProject_Add(llvm_pass
  PREFIX llvm_pass
  SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/llvm_pass
  BINARY_DIR ${LLVM_PASS_BUILD_DIR}
  CMAKE_ARGS
    -DLLVM_DIR=${LLVM_BUILD_DIR}/lib/cmake/llvm
    -DProtobuf_DIR=${PROTOBUF_BUILD_DIR}/lib/cmake/protobuf
  INSTALL_COMMAND ""
  DEPENDS llvm protobuf
)
