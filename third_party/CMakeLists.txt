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

ExternalProject_Add(clang
  URL "https://github.com/llvm/llvm-project/releases/download/llvmorg-10.0.0/clang-10.0.0.src.tar.xz"
  PATCH_COMMAND patch -p1 -i ${CMAKE_CURRENT_SOURCE_DIR}/clang.patch
  BUILD_COMMAND ""
  CONFIGURE_COMMAND ""
  INSTALL_COMMAND ""
)

get_target_property(FUZZMON_COLLECTOR_LIB_PATH collector_lib FUZZMON_COLLECTOR_LIB_PATH)
ExternalProject_Add(llvm
  URL "https://github.com/llvm/llvm-project/releases/download/llvmorg-10.0.0/llvm-10.0.0.src.tar.xz"
  CMAKE_ARGS
    -DFUZZMON_COLLECTOR_LIB_PATH=${FUZZMON_COLLECTOR_LIB_PATH}
    -DFUZZMON_COLLECTOR_PASS_PATH=${FUZZMON_COLLECTOR_PASS_PATH}
    -DLLVM_EXTERNAL_CLANG_SOURCE_DIR=${CMAKE_CURRENT_BINARY_DIR}/clang-prefix/src/clang
    -DLLVM_EXTERNAL_COMPILER_RT_SOURCE_DIR=${CMAKE_CURRENT_SOURCE_DIR}/compiler-rt
    -DLLVM_EXTERNAL_PROJECTS=clang;compiler-rt
  BUILD_COMMAND cmake --build .
  BUILD_ALWAYS ON
  INSTALL_COMMAND ""
  DEPENDS clang collector_lib
)
set_property(TARGET llvm
  PROPERTY BUILD_DIR
    ${CMAKE_CURRENT_BINARY_DIR}/llvm-prefix/src/llvm-build)

ExternalProject_Add(protobuf
  URL "https://github.com/protocolbuffers/protobuf/releases/download/v3.11.4/protobuf-cpp-3.11.4.tar.gz"
  SOURCE_SUBDIR cmake
  CMAKE_ARGS
    -DCMAKE_CXX_FLAGS=-DGOOGLE_PROTOBUF_NO_RTTI
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON
    -Dprotobuf_WITH_ZLIB=OFF
    -Dprotobuf_BUILD_SHARED_LIBS=OFF
    -Dprotobuf_BUILD_TESTS=OFF
  INSTALL_COMMAND ""
)
set_property(TARGET protobuf
  PROPERTY BUILD_DIR
    ${CMAKE_CURRENT_BINARY_DIR}/protobuf-prefix/src/protobuf-build)
