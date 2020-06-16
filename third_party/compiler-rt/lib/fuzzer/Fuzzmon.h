/*
 * Copyright 2020 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#ifndef FUZZMON_H_
#define FUZZMON_H_

#include <stdint.h>

namespace fuzzmon {

struct CfgPayloadData {
  const uint8_t *Buffer;
  size_t Size;
};

struct CfgRemapData {
  const uint64_t *Starts;
  const uint64_t *Offsets;
  size_t Size;
};

struct Module {
  CfgPayloadData CfgPayload;
  CfgRemapData CfgRemap;
};

struct LibCollectorParam {
  Module *Modules;
  size_t ModulesSize;
};

} // namespace fuzzmon

extern "C" void
fuzzmon_libcollector_init(const fuzzmon::LibCollectorParam *Param);

extern "C" void fuzzmon_libcollector_update_features(const uint32_t *Features,
                                                     size_t FeaturesSize);

#endif // FUZZMON_H_
