# Glyph Language Specification

Glyph is a token-dense, AI-native language. Humans need not read it; machines compile it.

## Design Principles

1. Single-character keywords where possible
2. Heavy type inference (types optional)
3. Prefix `@` for calls: `@(fn, arg1, arg2)`
4. Ternary `? cond then else` (no `:` needed)
5. No comments, no semicolons, newline-terminated statements

## Tokens

| Token | Meaning |
|-------|---------|
| `f` | function definition |
| `l` | let binding |
| `w` | while loop |
| `!` | return |
| `?` | ternary conditional |
| `@` | call |
| `p` | builtin print (via `@p`) |

## Grammar (EBNF)

```ebnf
program     = { function } ;
function    = "f" IDENT "(" [ params ] ")" [ ":" type ] ( "=" expr | block ) ;
params      = param { "," param } ;
param       = IDENT [ ":" type ] ;
block       = "{" { statement } "}" ;
statement   = let | while | return | expr ;
let         = "l" IDENT "=" expr ;
while       = "w" expr block ;
return      = "!" expr ;
expr        = ternary ;
ternary     = assign [ "?" assign assign ] ;
assign      = IDENT "=" assign | compare ;
compare     = add ( ( "<" | ">" | "<=" | ">=" | "==" | "!=" ) add )* ;
add         = mul ( ( "+" | "-" ) mul )* ;
mul         = unary ( ( "*" | "/" | "%" ) unary )* ;
unary       = ( "-" | "!" ) unary | call ;
call        = primary { "(" [ args ] ")" | "@" "(" IDENT "," args ")" } ;
primary     = NUMBER | STRING | IDENT | "@" "(" IDENT "," [ args ] ")" | "(" expr ")" ;
args        = expr { "," expr } ;
type        = "i" | "f" | "b" | "v" | "s" ;
```

## Types

| Code | C type |
|------|--------|
| `i` | `int64_t` |
| `f` | `double` |
| `b` | `int` (0/1) |
| `v` | `void` |
| `s` | `char*` |

Inference: integer literals → `i`, float → `f`, comparisons → `b`.

## Builtins

| Call | Semantics |
|------|-----------|
| `@p, x` | print integer |
| `@pf, x` | print float |
| `@ps, x` | print string |

## Example

```
f fib(n:i):i = ? n<2 n @(fib,n-1)+@(fib,n-2)
f main():v {
  @p,@(fib,10)
}
```
