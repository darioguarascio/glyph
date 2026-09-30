# Legacy transpiler (deprecated)

`compiler/glyphc.py` translates `.gl` text to C and invokes `gcc`. This path exists for early token benchmarks only.

**Do not extend.** New language work belongs in:

- `spec/BYTECODE.md` — canonical GBC IR
- `native/` — Rust VM and (future) native codegen

The transpiler will be removed once the GBC frontend reaches parity on examples.
