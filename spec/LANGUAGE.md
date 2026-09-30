# Glyph Language Reference

> **Audience:** LLM agents generating or modifying Glyph code, and humans auditing that code.  
> **Extension:** `.gl`  
> **Compiler:** `glyphc` (`compiler/glyphc.py`) → C11 → native binary via `gcc`  
> **Version:** MVP (September 2026)

---

## 1. What Glyph Is

Glyph is a **token-dense, AI-native** language. Source is optimized for LLM generation and compilation, not human reading. Programs compile to native executables through a thin C backend.

**Design goals**

| Goal | Mechanism |
|------|-----------|
| Fewer tokens | Single-char keywords (`f`, `l`, `w`, `?`, `!`, `@`) |
| No ceremony | No semicolons, no comments, whitespace-separated |
| Predictable codegen | Subset maps cleanly to C (`int64_t`, `char*`, static arrays) |
| Runnable binaries | `glyph_rt.h` + `glyph_net.c` linked on every build |

**Non-goals (current MVP):** structs, modules, generics, exceptions, garbage collection, HTTPS, full curl parity.

---

## 2. Quick Reference

### Keywords & punctuation

| Token | Role |
|-------|------|
| `f` | function definition |
| `l` | local binding (`let`) |
| `w` | while loop |
| `!` | return |
| `?` | conditional (if/else or ternary) |
| `@` | call prefix (builtins and user functions) |
| `g` | global `int64_t` array |
| `gb` | global `char` byte buffer |
| `{` `}` | block |
| `[` `]` | array index |
| `:` | type annotation / global size |
| `,` | argument / parameter separator |

### Types

| Glyph | C type | Notes |
|-------|--------|-------|
| `i` | `int64_t` | default for locals, integers |
| `f` | `double` | float literals |
| `b` | `int` | 0/1 comparisons |
| `v` | `void` | no return value |
| `s` | `const char*` | string literal or argv pointer |

Omitted parameter/return types default to `i`. Locals are always emitted as `int64_t`.

### Call forms

```
@ps,"hello"           @ builtin: print string
@(fib,10)             @ user fn with parens
@fetch,@argv,1        @ user fn, comma args (preferred)
fib(10)               @ alternate user call (no @)
```

### Control flow idioms

```
? cond then_expr else_expr          @ expression ternary
? cond stmt_or_block else_branch    @ statement if/else (else may be block)
? a @ps,"x" ? b @ps,"y" @p,i        @ chained flat else-if (see §7)
```

---

## 3. Lexical Structure

### Whitespace

Spaces, tabs, and newlines separate tokens. Newlines are **not** statement terminators (there are no terminators); they are ordinary whitespace.

### Identifiers

`[a-zA-Z_][a-zA-Z0-9_]*` — except reserved words `f`, `g`, `gb`, `l`, `w`.

### Numbers

- Integer: `0`, `42`, `30000`
- Float: `3.14` (must contain `.`)

### Strings

Double-quoted. Escapes:

| Escape | Character |
|--------|-----------|
| `\n` | newline |
| `\r` | carriage return |
| `\t` | tab |
| `\\` | backslash |
| `\"` | double quote |

**Important:** HTTP and protocol strings need explicit `\r\n` — the compiler emits correct C escapes.

### Operators (by precedence, high → low)

1. Unary `-`, `!`
2. `*`, `/`, `%`
3. `+`, `-`
4. `<`, `>`, `<=`, `>=`, `==`, `!=`
5. `=` (assignment, right-associative)
6. `? … …` (ternary / conditional)

### Indexing & calls

- Index: `name[expr]` — postfix on identifiers only; **no nested index** (`a[b[c]]` invalid).
- Index assign: `name[idx]=expr` — parsed as postfix on call chain.
- User call: `name(arg, …)` or `@name,arg,…` or `@(name,arg,…)`.

---

## 4. Program Structure

A program is a sequence of:

1. **Global arrays** (`g` / `gb`)
2. **Functions** (`f`)
3. **Script statements** (optional top-level code)

If the file contains top-level statements and **no** `f main`, the compiler wraps them in `f main():v { … }`.

### Global arrays

```
g tape:30000       @ int64_t tape[30000]
gb host:256        @ char host[256]
```

Syntax: `(g | gb) IDENT : NUMBER`

- `g` → `static int64_t name[N]`
- `gb` → `static char name[N]` (byte buffer; index read returns unsigned byte as int)

### Functions

```
f NAME ( [ param [: type] [, …] ] ) [: ret_type] block
f NAME ( … ) [: ret_type] = expr
```

Examples:

```
f add(a:i,b:i):i = a+b

f main():v {
  @ps,"hi"
}
```

`main` is special: emitted as C `int main(int argc, char **argv)` with `glyph_argc_val` / `glyph_argv_val` initialized. Glyph return type `v` still yields C `return 0`.

---

## 5. Statements

Inside `{ … }`:

| Form | Meaning |
|------|---------|
| `l name = expr` | bind local (always `int64_t`) |
| `w cond { … }` | while loop |
| `! expr` | return (void functions wrap expr in `(void)(…); return;`) |
| `expr` | expression statement (calls, assignment, conditional) |

Assignment as statement reuses expression assign: `i=i+1` → `(i = (i + 1));`

---

## 6. Conditionals (`?`)

Two surface forms share one token:

### 6.1 Prefix form (statements and nested blocks)

```
? condition then_branch else_branch
```

Each branch is parsed with `branch()`:

- `{ … }` block
- nested `? …` 
- or any expression

Example — early exit with blocks:

```
? port<0 {
  @ps,"error: bad url"
} {
  l fd=@tcpconn,host,port
  …
}
```

### 6.2 Infix form (compact expression chains)

```
expr ? then_expr else_expr
```

Example:

```
f fib(n:i):i = ? n<2 n @(fib,n-1)+@(fib,n-2)
```

### 6.3 Chained else-if (flat chain)

Each condition consumes **one** else branch; chain by nesting ternaries in the else position:

```
? m==0 @ps,"FizzBuzz" ? a==0 @ps,"Fizz" ? b==0 @ps,"Buzz" @p,i
```

Equivalent to:

```
if (m==0) print FizzBuzz
else if (a==0) print Fizz
else if (b==0) print Buzz
else print i
```

**Do not** write `? a ? b c d` without a final else — the parser requires three operands per `?`.

**Do not** use `? 0 0` as a noop — invalid (needs condition + then + else). Use literal `0` as the else branch:

```
? ch==91 i=@sf,code,i,dp ? ch==93 i=@h93,code,i,dp 0
```

---

## 7. Arrays

### Declaration (globals only)

```
g mem:30000
gb req:4096
```

No local arrays in MVP.

### Access

```
mem[i]           @ read int64 cell
mem[i]=mem[i]+1  @ write
req[pos]=@ch,s,j @ byte buffer write (truncated to char)
```

---

## 8. Builtins

All builtins use `@name[, args…]` or `@(name,args…)`. Names are **case-sensitive**.

### 8.1 I/O & strings

| Call | Args | Returns | Effect |
|------|------|---------|--------|
| `@p,x` | int | — | print int + newline |
| `@pf,x` | float | — | print float + newline |
| `@ps,s` | string | — | print string + newline |
| `@o,c` | int (char code) | — | `putchar` |
| `@r` | — | int | `getchar` |
| `@len,s` | string | int | `strlen` |
| `@ch,s,i` | string, index | int | byte at index (0–255) |

### 8.2 Command line

| Call | Returns |
|------|---------|
| `@argc` | argument count (same as C argc) |
| `@argv,i` | `i`th argument as string |

### 8.3 TCP / HTTP (runtime: `glyph_net.c`)

| Call | Args | Returns | Notes |
|------|------|---------|-------|
| `@tcpconn,host,port` | string, int | fd or `-1` | `host` is `char*` or `gb` buffer name |
| `@tcpsend,fd,buf,len` | int, buffer, int | bytes sent or `-1` | |
| `@tcpread,fd,buf,max` | int, buffer, int | bytes read or `-1` | |
| `@tcpclose,fd` | int | — | |
| `@stdout,buf,len` | buffer, int | — | raw write to stdout |
| `@parseurl,url` | string | port or `-1` | **Requires** globals `gb host:N` and `gb path:M`; fills them |
| `@body,fd` | int | — | read HTTP response on `fd`, skip headers, print body |

**URL parsing:** only `http://` URLs. Default port 80. Host/port/path written into the `host` and `path` global buffers declared in the same file.

**HTTP client pattern:**

```
gb host:256
gb path:2048
gb req:4096

l port=@parseurl,url
? port<0 { @ps,"error: bad url" } {
  l fd=@tcpconn,host,port
  …
}
```

---

## 9. Compilation Model

```
source.gl  →  glyphc.py (parse + codegen)  →  out.c  →  gcc -O2 -std=c11  →  binary
                                                      ↘ glyph_net.c
```

- Every build links `runtime/glyph_net.c` (even programs that do not use networking).
- Runtime headers: `runtime/glyph_rt.h`, `runtime/glyph_net.h`.
- **Requirements:** Python 3, gcc.

```bash
./glyphc examples/hello.gl -o hello
./hello
```

---

## 10. Agent Guidelines (common mistakes)

| Mistake | Fix |
|---------|-----|
| `\r\n` in strings becomes literal `rn` | Use `"\\r\\n"` in Glyph source (compiler escapes correctly) |
| `@tcpsend,fd,req,n<0` | Parses as send length `(n<0)`. Use `l sent=@tcpsend,fd,req,n` then `? sent<0 …` |
| Ternary else branch is `{ … }` but parse fails | Supported after parser fix — both branches use `branch()` |
| Error path prints but continues | Use block form: `? err { @ps,"msg" } { … real work … }` |
| `@parseurl` without `gb host` / `gb path` | Codegen uses default sizes 256/2048 but buffers must exist |
| Nested ternary as then-branch without else | Add explicit final else (`0` or block) |
| `? 0 0` noop | Invalid — use `0` as expression |
| Local byte buffers | Not supported — use `gb` globals |
| HTTPS, redirects, POST | Not in MVP |

---

## 11. Complete Examples

### Hello World (script mode)

```
@ps,"Hello, World!"
```

### Fibonacci

```
f fib(n:i):i = ? n<2 n @(fib,n-1)+@(fib,n-2)
f main():v { @p,@(fib,10) }
```

### FizzBuzz

```
f main():v {
  l i=1
  w i<=100 {
    l m=i%15
    l a=i%3
    l b=i%5
    ? m==0 @ps,"FizzBuzz" ? a==0 @ps,"Fizz" ? b==0 @ps,"Buzz" @p,i
    i=i+1
  }
}
```

### Brainfuck interpreter (excerpt)

```
g mem:30000

f run(code:s):v {
  l dp=0
  l i=0
  w i<@len,code {
    l ch=@ch,code,i
    ? ch==62 dp=dp+1 ? ch==60 dp=dp-1 ? ch==43 mem[dp]=mem[dp]+1 … 0
    i=i+1
  }
}
```

See `examples/bf.gl`, `examples/httpget.gl`.

---

## 12. Formal Grammar (EBNF)

```ebnf
program       = { global | function | statement } ;

global        = "g" IDENT ":" NUMBER
              | "gb" IDENT ":" NUMBER ;

function      = "f" IDENT "(" [ params ] ")" [ ":" type ]
                ( "=" expr | block ) ;

params        = param { "," param } ;
param         = IDENT [ ":" type ] ;

block         = "{" { statement } "}" ;

statement     = "l" IDENT "=" expr
              | "w" expr block
              | "!" expr
              | expr ;

expr          = ternary ;
ternary       = [ "?" assign branch branch ]
              | assign [ "?" assign assign ] ;
branch        = block | ternary | assign ;

assign        = IDENT "=" assign | compare ;
compare       = add ( ( "<" | ">" | "<=" | ">=" | "==" | "!=" ) add )* ;
add           = mul ( ( "+" | "-" ) mul )* ;
mul           = unary ( ( "*" | "/" | "%" ) unary )* ;
unary         = ( "-" | "!" ) unary | postfix ;
postfix       = primary { "[" assign "]" [ "=" assign ] | "(" [ args ] ")" } ;
primary       = NUMBER | STRING | IDENT | at_call | "(" expr ")" ;

at_call       = "@" [ "(" IDENT [ "," args ] ")" | IDENT [ "," args ] ] ;
args          = expr { "," expr } ;

type          = "i" | "f" | "b" | "v" | "s" ;
```

---

## 13. File Index

| Path | Purpose |
|------|---------|
| `spec/LANGUAGE.md` | This document |
| `spec/grammar.md` | Short spec stub + EBNF pointer |
| `compiler/glyphc.py` | Reference implementation |
| `runtime/glyph_rt.h` | Core runtime |
| `runtime/glyph_net.c` | TCP/HTTP helpers |
| `examples/*.gl` | Runnable samples |
| `benchmarks/programs/*.gl` | Token-benchmark sources |

---

## 14. License

MIT (see repository root).
