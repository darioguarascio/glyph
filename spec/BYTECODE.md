# GBC — Glyph Bytecode

GBC is the **canonical** program representation. Agents should target GBC (or a token stream that maps 1:1 to these opcodes). Text `.gl` is sugar.

## File format

```
GBC1 magic[4]        "GBC1"
version      u8       1
flags        u8       0 = exec, 1 = debug symbols
str_count    u16      string pool entries
glob_count   u16      global segments
fn_count     u16      functions
str_pool     ...      length-prefixed UTF-8 blobs
glob_table   ...      per segment: kind u8, name_idx u16, size u32
fn_table     ...      per fn: name_idx u16, arity u8, slot_count u16, code_len u32
code         ...      concatenated function bodies
```

All integers little-endian unless noted.

## Global kinds

| Kind | Value | Runtime |
|------|-------|---------|
| I64_ARRAY | 0 | `int64_t name[size]` in `.bss` |
| BYTE_ARRAY | 1 | `uint8_t name[size]` in `.bss` |

## Value model

- Slot values are **i64** on the stack and in `g` arrays.
- `gb` load/store truncates/extends to byte.
- No float in GBC v1 (add `F64` ops in v2 if needed).

## Opcode map (v1)

One-byte opcode, operands follow immediately.

| Op | Hex | Operands | Effect |
|----|-----|----------|--------|
| NOP | 00 | — | |
| HALT | 01 | — | stop (main return) |
| CONST_I64 | 02 | i64 | push imm |
| CONST_STR | 03 | u16 idx | push str pool index as i64 handle |
| POP | 04 | — | drop TOS |
| DUP | 05 | — | duplicate TOS |
| LOAD_LOCAL | 10 | u16 slot | push local |
| STORE_LOCAL | 11 | u16 slot | pop → local |
| LOAD_GLOB | 12 | u16 g | u32 off | push glob[g][off] |
| STORE_GLOB | 13 | u16 g | u32 off | pop → glob[g][off] |
| LOAD_GLOB_IDX | 14 | u16 g | pop idx → push glob[g][idx] |
| STORE_GLOB_IDX | 15 | u16 g | pop idx, pop val → glob[g][idx] |
| ADD | 20 | — | pop b,a; push a+b |
| SUB | 21 | — | |
| MUL | 22 | — | |
| DIV | 23 | — | signed div |
| MOD | 24 | — | |
| NEG | 25 | — | unary minus |
| NOT | 26 | — | logical not (0/1) |
| CMP_LT | 30 | — | push a<b |
| CMP_GT | 31 | — | |
| CMP_LE | 32 | — | |
| CMP_GE | 33 | — | |
| CMP_EQ | 34 | — | |
| CMP_NE | 35 | — | |
| JMP | 40 | i32 rel | ip += rel |
| JMP_IF | 41 | i32 rel | pop; if nonzero ip += rel |
| JMP_IFNOT | 42 | i32 rel | pop; if zero ip += rel |
| CALL | 50 | u16 fn | u8 argc | call user fn |
| RET | 51 | — | return to caller |
| CALL_BUILTIN | 52 | u8 id | u8 argc | see builtins |
| SYSCALL | 53 | u8 id | u8 argc | OS/runtime |

### Builtin IDs (CALL_BUILTIN)

| ID | Name | Args | Returns |
|----|------|------|---------|
| 0 | print_i64 | 1 | |
| 1 | print_str | 1 str_idx | |
| 2 | putchar | 1 | |
| 3 | getchar | 0 | i64 |
| 4 | strlen | 1 str_idx | i64 |
| 5 | char_at | 1 str_idx, 1 i64 | i64 byte |
| 6 | argc | 0 | i64 |
| 7 | argv | 1 i64 | str_idx |

Network builtins (TCP, parseurl, http body) are **syscalls** in the runtime, not language core—keeps GBC small.

## Calling convention (v1)

- Args pushed left-to-right; callee receives in slots 0..arity-1.
- Return value in slot 0 or TOS (TBD in codegen; VM uses TOS).
- Caller cleans arguments (CALL pops argc from conceptual stack).

## Example: hello (conceptual)

```
CONST_STR 0          ; "Hello, World!"
CALL_BUILTIN 1, 1    ; print_str
HALT
```

String pool[0] = `Hello, World!\n`

## AI token mapping

Each opcode fits one tokenizer token when using a dedicated vocabulary:

```
| OP name | suggested token ID | ASCII fallback |
| CONST_I64 | `<i64:imm>` | `02` + bytes |
| ADD | `+` or `ADD` | `20` |
```

Long-term: models fine-tuned on GBC emit **binary token IDs**; the assembler writes `.gbc` without a text stage.

## Versioning

- Bump `version` byte on breaking opcode changes.
- Verifier rejects unknown opcodes and out-of-range global indices before codegen.

See `vm/` reference interpreter and `compiler/glyph-ir/` for the implementation track.
