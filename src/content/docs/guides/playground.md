---
title: Playground & WebAssembly
description: Run Marrow in your browser, and compile it to WebAssembly yourself.
---

The [playground](/marrow/playground/) compiles and runs Marrow programs entirely in your browser. Nothing is sent to a server: the site is static, and the compiler itself runs as WebAssembly.

## How it works

The Marrow compiler is written in Marrow, so it can compile itself to WebAssembly. The playground loads that build in a Web Worker and runs three steps:

1. `marrow --emit-il` compiles your program to QBE's intermediate language, without calling `qbe` or a C toolchain.
2. A small translator (`il2wasm.mjs`) turns that IL into a WebAssembly module.
3. A second worker instantiates the module. Its only import is the raw system call, which `runtime.mjs` emulates: standard streams, an in-memory file system, `mmap`, clocks, random numbers and `exit`.

Programs can use the whole standard library that does not need threads or the network. Sockets, `fork`, `exec` and threads answer `ENOSYS`.

A run is stopped after 20 seconds, so an endless loop does not freeze the page. **Stop** ends it earlier.

## Sharing

**Share** copies a link that holds the program itself in the URL fragment, so it is never sent anywhere either.

## Doing it yourself

```bash
marrow --emit-il --nostdlib --target x86_64-linux main.mw main.ssa
node tools/wasm/build.mjs build/marrow out/   # compiler.wasm, std.json, translator, runtime
node tests/wasm.mjs                           # the runnable test programs, in WebAssembly
```

`--emit-il` stops after code generation and writes the IL to the output path.
