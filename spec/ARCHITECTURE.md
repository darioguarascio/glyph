# Glyph Architecture (Native Compiler Track)

## Current state (legacy)

The Python `compiler/glyphc.py` is a **transpiler**: `.gl` text → C source → `gcc`. It proved token-density benchmarks but is **not** the long-term compiler. It remains for comparison and migration tests only.

## Target

Glyph is a language whose **primary representation is bytecode**, not text. Text (`.gl`) is an optional human/agent-readable surface syntax. The compiler produces native machine code without delegating semantics to C.

```
┌─────────────┐     ┌──────────────┐     ┌─────────┐     ┌──────────────┐
│ .gl (text)  │────▶│   Frontend   │────▶│  GBC    │────▶│   Backend    │
│ token stream│     │ parse+lower  │     │ bytecode│     │ Cranelift/   │
└─────────────┘     └──────────────┘     └─────────┘     │ LLVM → ELF   │
       ▲ optional                          ▲ canonical   └──────────────┘
       │                                   │
       └──────── LLM may emit GBC directly ┘
```

## Design principles

1. **Bytecode is truth** — tools, debuggers, and optimizers operate on `.gbc`, not C.
2. **AI-first encoding** — fixed-width opcodes, integer operands, string table indices; minimal ambiguity for generation and parsing.
3. **Low-level semantics** — explicit memory (globals, stack slots), no hidden GC, no C ABI leakage.
4. **Native codegen** — Cranelift (phase 1 backend) or LLVM (phase 2); no `-o out.c` step.
5. **Incremental delivery** — each phase produces a runnable artifact; no “big bang” rewrite.

## Compiler phases

| Phase | Input | Output | Validates |
|-------|-------|--------|-----------|
| Lex | `.gl` bytes | token stream | surface syntax |
| Parse | tokens | AST | grammar |
| Lower | AST | GBC module | semantics |
| Verify | GBC | GBC or error | types, bounds |
| Opt | GBC | GBC | peephole, const fold |
| Codegen | GBC | object / ELF | native execution |

Optional fast path: **token stream → GBC** skipping text AST when the model emits opcodes directly.

## Runtime model

- **Stack slots** — locals (`l`), fixed at compile time per function.
- **Global segments** — `g` (i64 array), `gb` (byte array), linked as `.data` / `.bss`.
- **Calls** — direct call opcodes; builtins are well-known indices (not libc names).
- **Syscalls / OS** — thin platform layer (`glyph_rt`) written in C or asm for I/O only; language semantics do not pass through it.

## Repository layout (target)

```
glyph/
├── spec/
│   ├── ARCHITECTURE.md    ← this file
│   ├── BYTECODE.md        ← GBC opcode reference
│   └── LANGUAGE.md        ← surface syntax (legacy frontend)
├── compiler/              ← Rust: glyph-compiler workspace
│   ├── glyph-frontend/    ← .gl → AST
│   ├── glyph-ir/          ← AST → GBC, verifier
│   ├── glyph-codegen/     ← GBC → native (Cranelift)
│   └── glyph-cli/         ← `glyph build`, `glyph run`
├── vm/                    ← reference interpreter for GBC (tests, debug)
├── runtime/               ← minimal OS glue (write, exit, mmap)
└── legacy/                ← Python transpiler (frozen, deprecated)
```

## Milestones

### M0 — Specification (now)
- GBC opcode set and file format
- Architecture document
- Legacy transpiler labeled deprecated

### M1 — Reference VM ✓
- Rust interpreter for GBC (`native/glyph-vm`)
- `glyph run hello.gbc` works
- Unit tests for core opcodes

### M2 — Frontend ✓ (partial)
- `.gl` parser + lowering in Rust (`native/glyph-frontend`)
- `glyph compile hello.gl -o hello.gbc` / `glyph run hello.gl`
- Parity: hello, fib, fizzbuzz (VM execution, no gcc)
- Remaining: bf, httpget, full builtin/syscall surface

### M3 — Native codegen
- Cranelift backend: GBC → ELF object → executable
- Drop gcc from default pipeline

### M4 — AI direct emission
- Document token→GBC mapping for LLM prompts
- Benchmark: tokens to runnable binary without text surface

### M5 — Optimizer
- Constant fold, dead slot elimination, peephole on GBC
- Measure codegen quality vs `-O2` gcc transpile path

## What we explicitly reject

- C as intermediate representation
- “Native binary” meaning “gcc links our C”
- Growing the Python transpiler with new features
- Optimizing for demo speed over IR/codegen correctness

## Backend choice

**Cranelift** first: embeddable, Rust-native, good for a new language, fast enough for AOT.

**LLVM** later if we need mature optimization passes and multi-arch support at scale.

Custom x64 emitter only for hot paths or freestanding targets—not as the main compiler.
