# Token Comparison: Glyph vs Traditional Languages

Measured with **cl100k_base (GPT-4/4o)**.

Each row = same natural-language prompt + generated program source.

### Hello

**Prompt:** Write a complete program that prints "Hello, World!" to stdout.

| Language | Prompt | Code | **Total** | Chars |
|----------|--------|------|-----------|-------|
| Glyph | 14 | 7 | **21** | 20 |
| Python | 14 | 6 | 20 | 23 |
| JavaScript | 14 | 7 | 21 | 30 |
| C | 14 | 25 | 39 | 83 |
| Rust | 14 | 12 | 26 | 45 |
| Go | 14 | 19 | 33 | 71 |

### Fib

**Prompt:** Write a complete program with a recursive fibonacci function and print fib(10).

| Language | Prompt | Code | **Total** | Chars |
|----------|--------|------|-----------|-------|
| Glyph | 15 | 38 | **53** | 73 |
| Python | 15 | 32 | 47 | 78 |
| JavaScript | 15 | 34 | 49 | 86 |
| C | 15 | 55 | 70 | 151 |
| Rust | 15 | 49 | 64 | 121 |
| Go | 15 | 52 | 67 | 148 |

### Fizzbuzz

**Prompt:** Write FizzBuzz: for i from 1 to 100, print Fizz if divisible by 3, Buzz if by 5, FizzBuzz if by both, else the number.

| Language | Prompt | Code | **Total** | Chars |
|----------|--------|------|-----------|-------|
| Glyph | 40 | 80 | **120** | 154 |
| Python | 40 | 59 | 99 | 151 |
| JavaScript | 40 | 74 | 114 | 201 |
| C | 40 | 95 | 135 | 268 |
| Rust | 40 | 73 | 113 | 244 |
| Go | 40 | 85 | 125 | 259 |

### Aggregate

| Language | Total Tokens | Savings vs C |
|----------|--------------|-------------|
| Glyph | 194 | +20.5% |
| Python | 166 | +32.0% |
| JavaScript | 184 | +24.6% |
| C | 244 | +0.0% |
| Rust | 203 | +16.8% |
| Go | 225 | +7.8% |
