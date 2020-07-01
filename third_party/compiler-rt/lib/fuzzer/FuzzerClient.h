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

#ifndef FUZZER_CLIENT_H_
#define FUZZER_CLIENT_H_

#include <cstddef>
#include <cstdint>

namespace fuzzer_client {

const size_t NO_CORPUS_INDEX = (~(size_t)0);

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

struct FuzzerClientParam {
  Module *Modules;
  size_t ModulesSize;
};

struct CorpusPriority {
  size_t Index;
  uint32_t Priority;
};

} // namespace fuzzer_client

extern "C" void
fuzzer_client_init(const fuzzer_client::FuzzerClientParam *Param);

extern "C" void fuzzer_client_update_features(const uint32_t *Features,
                                              size_t FeaturesSize,
                                              size_t CorpusIndex);

extern "C" size_t
fuzzer_client_get_corpus_priorities(fuzzer_client::CorpusPriority *Buffer,
                                    size_t BufferSize);

#endif // FUZZER_CLIENT_H_
