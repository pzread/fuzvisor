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

project(fuzzmon_collector)

add_custom_target(collector_lib
  COMMAND cargo build --release --lib --target-dir ${CMAKE_CURRENT_BINARY_DIR}
  WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
set_property(TARGET collector_lib PROPERTY
  FUZZMON_COLLECTOR_LIB_PATH ${CMAKE_CURRENT_BINARY_DIR}/release/libcollector.so)
