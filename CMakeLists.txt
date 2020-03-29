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
