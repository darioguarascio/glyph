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

### Bf

**Prompt:** Write a Brainfuck interpreter: takes BF source as argv[1], 30000-cell tape, all 8 commands, matching bracket jumps.

| Language | Prompt | Code | **Total** | Chars |
|----------|--------|------|-----------|-------|
| Glyph | 29 | 353 | **382** | 687 |
| Python | 29 | 341 | 370 | 923 |
| JavaScript | 29 | 363 | 392 | 1117 |
| C | 29 | 411 | 440 | 1201 |
| Rust | 29 | 473 | 502 | 1341 |
| Go | 29 | 379 | 408 | 926 |

### Httpget

**Prompt:** Write an HTTP GET client: argv[1] is http://HOST/PATH, raw TCP sockets only (no curl/libcurl), parse URL, send HTTP/1.1 GET, print response body.

| Language | Prompt | Code | **Total** | Chars |
|----------|--------|------|-----------|-------|
| Glyph | 42 | 312 | **354** | 767 |
| Python | 42 | 526 | 568 | 1883 |
| JavaScript | 42 | 458 | 500 | 1610 |
| C | 42 | 1225 | 1267 | 3676 |
| Rust | 42 | 576 | 618 | 2016 |
| Go | 42 | 568 | 610 | 1745 |

### Aggregate

| Language | Total Tokens | Savings vs C |
|----------|--------------|-------------|
| Glyph | 930 | +52.3% |
| Python | 1104 | +43.4% |
| JavaScript | 1076 | +44.8% |
| C | 1951 | +0.0% |
| Rust | 1323 | +32.2% |
| Go | 1243 | +36.3% |
