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
