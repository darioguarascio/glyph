# Glyph Grammar

The canonical language reference is **[LANGUAGE.md](./LANGUAGE.md)** — syntax, semantics, builtins, compilation, and agent guidelines.

This file is a minimal index for tools that expect `grammar.md`.

## Summary

- **Extension:** `.gl`
- **Keywords:** `f` `g` `gb` `l` `w` `!` `?` `@`
- **Calls:** `@name,arg,…` or `@(name,arg,…)` or `name(arg,…)`
- **Conditionals:** `? cond then else` (prefix or infix); blocks allowed in branches
- **Globals:** `g name:N` (int64 array), `gb name:N` (char buffer)
- **Types:** `i` `f` `b` `v` `s` (optional on params/returns)

## EBNF

See §12 in [LANGUAGE.md](./LANGUAGE.md#12-formal-grammar-ebnf).
