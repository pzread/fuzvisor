cmake_minimum_required(VERSION 3.10)
find_package(Protobuf CONFIG REQUIRED)
find_package(LLVM 10.0 CONFIG REQUIRED)

project(fuzzmon_llvm_pass)

add_definitions(${LLVM_DEFINITIONS})
include_directories(${LLVM_INCLUDE_DIRS})
add_compile_options(-DGOOGLE_PROTOBUF_NO_RTTI -fno-rtti)

add_library(LLVMFuzzmonCollector MODULE
  CollectorPass.cpp
  ../proto/control_flow_graph.proto)
include_directories(LLVMFuzzmonCollector ${CMAKE_CURRENT_BINARY_DIR}/proto)
target_link_libraries(LLVMFuzzmonCollector protobuf::libprotobuf)
protobuf_generate(TARGET LLVMFuzzmonCollector
  APPEND_PATH
  PROTOC_OUT_DIR ${CMAKE_CURRENT_BINARY_DIR}/proto
)
