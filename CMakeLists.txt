cmake_minimum_required(VERSION 3.11)
find_package(LLVM 9.0 REQUIRED CONFIG)

project(fuzzmon)

set(CMAKE_POSITION_INDEPENDENT_CODE ON)

include(FetchContent)
FetchContent_Declare(
  protobuf
  GIT_REPOSITORY https://github.com/protocolbuffers/protobuf
  GIT_TAG        v3.11.4
)
FetchContent_GetProperties(protobuf)
if(NOT protobuf_POPULATED)
  FetchContent_Populate(protobuf)
  # Top level doesn't contain the CMakeLists.txt, it is in the "cmake" subdirectory
  add_compile_definitions(GOOGLE_PROTOBUF_NO_RTTI)
  add_subdirectory(${protobuf_SOURCE_DIR}/cmake ${protobuf_BINARY_DIR})
endif()

add_subdirectory(proto proto)
add_subdirectory(llvm_pass llvm_pass)
