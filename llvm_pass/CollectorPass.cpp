// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#include "control_flow_graph.pb.h"

#include "llvm/Analysis/LoopInfo.h"
#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/Instruction.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/Type.h"
#include "llvm/Pass.h"
#include "llvm/PassSupport.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Transforms/IPO/PassManagerBuilder.h"
#include "llvm/Transforms/Instrumentation.h"
#include "llvm/Transforms/Utils/ModuleUtils.h"

#include <string>
#include <unordered_map>

using namespace fuzzmon;
using namespace llvm;

namespace {
static constexpr char kStartSanCovCntrsSymbol[] = "__start___sancov_cntrs";
static constexpr char kSanCovCntrsSectionName[] = "__sancov_cntrs";
static constexpr char kInitFuncName[] = "__fuzzmon_collector_init";
static constexpr char kCtorFuncName[] = "fuzzmon.collector_ctor";
static constexpr uint64_t kNoSancovIndex = std::numeric_limits<uint64_t>::max();
static const int kCtorPriority = 573;

static uint64_t ScanAndMarkSingleSanCov8bitCounter(
    const GlobalVariable &GV, const uint64_t StartMark,
    std::unordered_map<const BasicBlock *, uint64_t> *MarkMap) {
  uint64_t MaxMark = StartMark;
  for (const Use &U : GV.uses()) {
    const auto *CE = dyn_cast<ConstantExpr>(U.getUser());
    if (CE == nullptr || CE->getOpcode() != Instruction::GetElementPtr) {
      continue;
    }
    const auto *Index = dyn_cast<ConstantInt>(CE->getOperand(2));
    if (Index == nullptr) {
      continue;
    }
    uint64_t CounterIndex = Index->getZExtValue();

    const auto *SI = dyn_cast<StoreInst>(CE->user_back());
    if (SI == nullptr) {
      continue;
    }
    const auto *BB = SI->getParent();

    uint64_t Mark = StartMark + CounterIndex;
    MarkMap->emplace(BB, Mark);
    MaxMark = std::max(MaxMark, Mark);
  }
  return MaxMark;
}

static void ScanAndMarkSanCov8bitCounter(
    Module &M, std::unordered_map<const BasicBlock *, uint64_t> *MarkMap,
    std::vector<std::pair<uint64_t, GlobalVariable *>> *RemapPoints) {
  MarkMap->clear();
  RemapPoints->clear();
  uint64_t NextStartMark = 0;
  for (GlobalVariable &GV : M.globals()) {
    if (GV.getSection() != kSanCovCntrsSectionName) {
      continue;
    }
    RemapPoints->emplace_back(NextStartMark, &GV);
    NextStartMark =
        ScanAndMarkSingleSanCov8bitCounter(GV, NextStartMark, MarkMap) + 1;
  }
}

static void AddRemapArray(
    Module *M,
    const std::vector<std::pair<uint64_t, GlobalVariable *>> &RemapPoints,
    GlobalVariable **RemapStartGV, GlobalVariable **RemapAddressGV) {
  auto &C = M->getContext();

  std::vector<uint64_t> RemapStarts;
  std::vector<Constant *> RemapAddresses;
  Constant *Zero = ConstantInt::get(Type::getInt32Ty(C), 0);
  for (const auto RemapPoint : RemapPoints) {
    GlobalVariable *RegionGV = RemapPoint.second;
    auto RegionAddress = ConstantExpr::getInBoundsGetElementPtr(
        RegionGV->getValueType(), RegionGV, ArrayRef<Constant *>({Zero, Zero}));
    RemapStarts.push_back(RemapPoint.first);
    RemapAddresses.push_back(RegionAddress);
  }
  auto *RemapStartArray = ConstantDataArray::get(
      C, ArrayRef<uint64_t>(RemapStarts.data(), RemapStarts.size()));
  *RemapStartGV =
      new GlobalVariable(*M, RemapStartArray->getType(), true,
                         GlobalValue::PrivateLinkage, RemapStartArray);
  auto *RemapAddressArray = ConstantArray::get(
      ArrayType::get(Type::getInt8PtrTy(C), RemapAddresses.size()),
      ArrayRef<Constant *>(RemapAddresses.data(), RemapAddresses.size()));
  *RemapAddressGV =
      new GlobalVariable(*M, RemapAddressArray->getType(), true,
                         GlobalValue::PrivateLinkage, RemapAddressArray);
}

static void BuildVoidFunction(LLVMContext &C, Function *F) {
  auto *EntryBlock = BasicBlock::Create(C, /*Name=*/"", F);
  IRBuilder<> IRB(EntryBlock, EntryBlock->getFirstInsertionPt());
  IRB.CreateRetVoid();
}

static void AddCtorAndCallInit(
    Module *M, const ControlFlowGraph &Cfg,
    const std::vector<std::pair<uint64_t, GlobalVariable *>> &RemapPoints) {
  auto &C = M->getContext();

  GlobalVariable *RemapStartGV;
  GlobalVariable *RemapAddressGV;
  AddRemapArray(M, RemapPoints, &RemapStartGV, &RemapAddressGV);
  auto RemapBaseAddress = ConstantExpr::getBitCast(
      M->getGlobalVariable(kStartSanCovCntrsSymbol), Type::getInt8PtrTy(C));

  auto *CtorFuncType =
      FunctionType::get(Type::getVoidTy(C), /*isVarArg=*/false);
  auto *CtorFunc =
      Function::Create(CtorFuncType, GlobalValue::LinkageTypes::PrivateLinkage,
                       kCtorFuncName, M);

  auto *EntryBlock = BasicBlock::Create(C, /*Name=*/"", CtorFunc);
  IRBuilder<> IRB(EntryBlock, EntryBlock->getFirstInsertionPt());

  std::string CfgByteString;
  Cfg.SerializeToString(&CfgByteString);
  auto *CfgPayload = ConstantDataArray::get(
      C,
      ArrayRef<uint8_t>(reinterpret_cast<const uint8_t *>(CfgByteString.data()),
                        CfgByteString.size()));
  auto *CfgPayloadGV = new GlobalVariable(
      *M, CfgPayload->getType(), true, GlobalValue::PrivateLinkage, CfgPayload);
  Constant *Zero = ConstantInt::get(Type::getInt32Ty(C), 0);
  Constant *CfgPayloadPtr = ConstantExpr::getInBoundsGetElementPtr(
      CfgPayloadGV->getValueType(), CfgPayloadGV,
      ArrayRef<Constant *>({Zero, Zero}));
  Constant *RemapStartPtr = ConstantExpr::getInBoundsGetElementPtr(
      RemapStartGV->getValueType(), RemapStartGV,
      ArrayRef<Constant *>({Zero, Zero}));
  Constant *RemapAddressPtr = ConstantExpr::getInBoundsGetElementPtr(
      RemapAddressGV->getValueType(), RemapAddressGV,
      ArrayRef<Constant *>({Zero, Zero}));
  auto *InitFuncType =
      FunctionType::get(Type::getVoidTy(C),
                        {CfgPayloadPtr->getType(), Type::getInt64Ty(C),
                         RemapStartPtr->getType(), RemapAddressPtr->getType(),
                         Type::getInt64Ty(C), RemapBaseAddress->getType()},
                        /*isVarArg=*/false);
  auto *InitFunc =
      Function::Create(InitFuncType, GlobalValue::LinkageTypes::WeakAnyLinkage,
                       kInitFuncName, M);
  BuildVoidFunction(C, InitFunc);
  IRB.CreateCall(InitFunc,
                 {CfgPayloadPtr,
                  ConstantInt::get(Type::getInt64Ty(C), CfgByteString.size()),
                  RemapStartPtr, RemapAddressPtr,
                  ConstantInt::get(Type::getInt64Ty(C), RemapPoints.size()),
                  RemapBaseAddress});

  IRB.CreateRetVoid();

  llvm::appendToGlobalCtors(*M, CtorFunc, /*Priority=*/kCtorPriority);
}
} // namespace

namespace {
class CollectorPass : public ModulePass {
public:
  static char ID;
  CollectorPass() : ModulePass(ID) {}

  bool runOnModule(Module &M) override;

private:
  uint64_t NextUniqueID = 1;

  uint64_t GenerateID();

  ControlFlowGraph::Function BuildFunctionCFG(
      const Function &F,
      const std::unordered_map<const BasicBlock *, uint64_t> &MarkMap);

  ControlFlowGraph
  BuildCFG(const Module &M,
           const std::unordered_map<const BasicBlock *, uint64_t> &MarkMap);
};
} // namespace

char CollectorPass::ID = 0;
static RegisterPass<CollectorPass> X("fuzzmon-collector", "fuzzmon-collector",
                                     false /* Only looks at CFG */,
                                     false /* Analysis Pass */);

uint64_t CollectorPass::GenerateID() { return NextUniqueID++; }

ControlFlowGraph::Function CollectorPass::BuildFunctionCFG(
    const Function &F,
    const std::unordered_map<const BasicBlock *, uint64_t> &MarkMap) {
  ControlFlowGraph::Function CfgF;
  CfgF.set_id(GenerateID());
  CfgF.set_name(F.getName());

  std::unordered_map<const BasicBlock *, ControlFlowGraph::BasicBlock *> BBMap;
  for (const BasicBlock &BB : F) {
    auto *CfgBB = CfgF.add_basic_blocks();
    CfgBB->set_id(GenerateID());

    const auto MarkI = MarkMap.find(&BB);
    if (MarkI != MarkMap.end()) {
      CfgBB->set_sancov_index(MarkI->second);
    } else {
      CfgBB->set_sancov_index(kNoSancovIndex);
    }

    BBMap.emplace(&BB, CfgBB);
  }

  for (const BasicBlock &BB : F) {
    auto *CfgBB = BBMap.find(&BB)->second;
    for (auto BI = succ_begin(&BB), BE = succ_end(&BB); BI != BE; ++BI) {
      const auto *CfgSBB = BBMap.find(*BI)->second;
      CfgBB->add_successors(CfgSBB->id());
    }
  }

  return std::move(CfgF);
}

ControlFlowGraph CollectorPass::BuildCFG(
    const Module &M,
    const std::unordered_map<const BasicBlock *, uint64_t> &MarkMap) {
  ControlFlowGraph Cfg;

  for (const Function &F : M) {
    const std::string FuncName = F.getName();
    if (F.size() == 0 || FuncName.find("sancov.") == 0) {
      continue;
    }
    *Cfg.add_functions() = BuildFunctionCFG(F, MarkMap);
  }

  return std::move(Cfg);
}

bool CollectorPass::runOnModule(Module &M) {
  if (M.getFunction(kCtorFuncName) != nullptr ||
      M.getGlobalVariable(kStartSanCovCntrsSymbol) == nullptr) {
    return false;
  }

  std::unordered_map<const BasicBlock *, uint64_t> MarkMap;
  std::vector<std::pair<uint64_t, GlobalVariable *>> RemapPoints;
  ScanAndMarkSanCov8bitCounter(M, &MarkMap, &RemapPoints);

  const ControlFlowGraph Cfg = BuildCFG(M, MarkMap);

  AddCtorAndCallInit(&M, Cfg, RemapPoints);
  return true;
}
