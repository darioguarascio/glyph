# Glyph

**Glyph** is a token-dense, AI-native programming language. It trades human readability for fewer LLM tokens — then compiles to native binaries via C + gcc.

```
@ps,"Hello, World!"
```

## Quick Start

```bash
./glyphc examples/hello.gl -o hello
./hello
```

Requires Python 3 and gcc.

## Token Comparison

Same natural-language prompt + generated program, measured with **GPT-4/4o tokenizer** (`cl100k_base`):

| Program | Language | Prompt | Code | **Total** | Savings vs C |
|---------|----------|--------|------|-----------|--------------|
| Hello World | **Glyph** | 14 | 7 | 21 | +46% |
| Hello World | Python | 14 | 6 | **20** | +49% |
| Hello World | C | 14 | 25 | 39 | — |
| Fibonacci | **Glyph** | 15 | 38 | 53 | +24% |
| Fibonacci | Python | 15 | 32 | **47** | +33% |
| Fibonacci | C | 15 | 55 | 70 | — |
| FizzBuzz | **Glyph** | 40 | 80 | 120 | +11% |
| FizzBuzz | Python | 40 | 59 | **99** | +27% |
| FizzBuzz | C | 40 | 95 | 135 | — |

**Aggregate:** Python **166** | **Glyph 194** | C **244** (all 3 programs combined)

Python wins on token count today — its stdlib is already token-optimized (`print`, `range`). Glyph's bet is different: **native binaries**, zero runtime dependency, and syntax that scales for AI *generation reliability* (strict grammar, no indentation sensitivity). As programs grow and include types/FFI, Glyph's per-token information density pulls ahead of verbose languages like C/Rust/Go.

Run benchmarks yourself:

```bash
pip install tiktoken
python3 tools/token_bench.py
```

## Source Examples

### Hello World

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

### Python equivalent (Fibonacci)

```python
def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)
print(fib(10))
```

### C equivalent (Fibonacci)

```c
#include <stdio.h>
long fib(long n) {
    return n < 2 ? n : fib(n - 1) + fib(n - 2);
}
int main(void) {
    printf("%ld\n", fib(10));
    return 0;
}
```

## Syntax Cheatsheet

| Glyph | Meaning |
|-------|---------|
| `f` | function |
| `l` | let binding |
| `w` | while |
| `? c a b` | if c then a else b |
| `@f,x,y` | call f(x, y) |
| `@p,i` | print integer |
| `@ps,s` | print string |
| `i f b v s` | types: int, float, bool, void, string |

## Project Layout

```
glyph-lang/
├── glyphc              # compiler CLI
├── compiler/glyphc.py  # parser + C codegen
├── runtime/glyph_rt.h  # print helpers
├── examples/           # .gl programs
├── benchmarks/         # cross-language token comparison
└── spec/grammar.md     # language spec
```

## License

MIT
