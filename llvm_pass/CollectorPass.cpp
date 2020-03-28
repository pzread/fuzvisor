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

using namespace llvm;

namespace {
static constexpr char kInitFuncName[] = "__fuzzmon_collector_init";
static constexpr char kCtorFuncName[] = "fuzzmon.collector_ctor";
static const int kCtorPriority = 573;

static void ScanAndMarkSingleSanCov8bitCounter(
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
  }
}

static const std::unordered_map<const BasicBlock *, uint64_t>
ScanAndMarkSanCov8bitCounter(const Module &M) {
  std::unordered_map<const BasicBlock *, uint64_t> MarkMap;
  for (const GlobalVariable &GV : M.globals()) {
    if (GV.getSection() != "__sancov_cntrs") {
      continue;
    }
    ScanAndMarkSingleSanCov8bitCounter(GV, MarkMap.size(), &MarkMap);
  }
  return std::move(MarkMap);
}

static void AddCtorAndCallInit(Module *M, const ControlFlowGraph &Cfg) {
  auto &C = M->getContext();
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
  auto *InitFuncType = FunctionType::get(
      Type::getVoidTy(C), {CfgPayload->getType()}, /*isVarArg=*/false);
  auto *InitFunc =
      Function::Create(InitFuncType, GlobalValue::LinkageTypes::ExternalLinkage,
                       kInitFuncName, M);
  IRB.CreateCall(InitFunc, {CfgPayload});

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
    const auto MarkI = MarkMap.find(&BB);
    if (MarkI == MarkMap.end()) {
      continue;
    }
    auto *CfgBB = CfgF.add_basic_blocks();
    CfgBB->set_id(MarkI->second);
    BBMap.emplace(&BB, CfgBB);
  }

  for (const BasicBlock &BB : F) {
    const auto CfgBBI = BBMap.find(&BB);
    if (CfgBBI == BBMap.end()) {
      continue;
    }
    auto *CfgBB = CfgBBI->second;
    for (auto BI = succ_begin(&BB), BE = succ_end(&BB); BI != BE; ++BI) {
      const auto CfgSBBI = BBMap.find(*BI);
      if (CfgSBBI == BBMap.end()) {
        continue;
      }
      CfgBB->add_successors(CfgSBBI->second->id());
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
  const std::unordered_map<const BasicBlock *, uint64_t> MarkMap =
      ScanAndMarkSanCov8bitCounter(M);

  const ControlFlowGraph Cfg = BuildCFG(M, MarkMap);

  AddCtorAndCallInit(&M, Cfg);
  return true;
}
