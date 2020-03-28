add_definitions(${LLVM_DEFINITIONS})
include_directories(${LLVM_INCLUDE_DIRS})
add_compile_options(-fno-rtti)

set(CMAKE_INCLUDE_CURRENT_DIR TRUE)

include_directories(LLVMFuzzmonCollector "${PROTO_DIR}")
add_library(LLVMFuzzmonCollector MODULE
  CollectorPass.cpp
  "${PROTO_DIR}/control_flow_graph.pb.cc")
target_link_libraries(LLVMFuzzmonCollector protobuf::libprotobuf)
set_property(SOURCE
  "${PROTO_DIR}/control_flow_graph.pb.cc"
  PROPERTY GENERATED TRUE)
