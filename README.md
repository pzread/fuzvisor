Fuzvisor
=======
A framwork provides an interface to monitor and control fuzzers.

**DISCLAIMER:** This is not an officially supported Google product.

Fuzvisor dumps the static prgoram structures (e.g. control flow graph) and loads them from a separated collecting server for analysis during fuzzing. The collecting server collects the coverage and performance from multiple fuzzing workers through lightweight and high-throughput gRPC protocol.

This project is still under heavy development.

Build
-----
**Prerequisites**

+ CMake >= 3.10
+ Toolchain to build Clang and LLVM
+ Latest Rust toolchain

**Build the modified Clang and LLVM**
```sh
mkdir fuzvisor-build && cd fuzvisor-build
cmake ../fuzvisor
cmake --build .
```

Usage
-----
**Prepare the fuzzing target**

Use the LLVM toolchain at `fuzvisor-build/toolchain/llvm-prefix/src/llvm-build/bin/` to compile your target with `libfuzzer`. For example:
```sh
fuzvisor-build/toolchain/llvm-prefix/src/llvm-build/bin/clang -fsanitize=fuzzer -O a.out target.cpp
```

**Start the collecting server**
```sh
cd fuzvisor/collector
cargo run --release
```
**Start the fuzzing target**

Same as running a `libfuzzer` target. For example:
```sh
./a.out -use_value_profile=1 -jobs=16
```

Developer Guides (WIP)
---------------
