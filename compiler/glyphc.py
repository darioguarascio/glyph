#!/usr/bin/env python3
"""Glyph compiler: .gl -> C -> native binary via gcc."""

import re
import sys
import subprocess
import tempfile
import os
from dataclasses import dataclass, field
from typing import Optional, List, Union
from enum import Enum, auto

# --- Lexer ---

class TT(Enum):
    F = auto(); L = auto(); W = auto(); RET = auto()
    Q = auto(); AT = auto(); COLON = auto()
    LP = auto(); RP = auto(); LB = auto(); RB = auto()
    COMMA = auto(); EQ = auto()
    PLUS = auto(); MINUS = auto(); STAR = auto(); SLASH = auto(); PERCENT = auto()
    LT = auto(); GT = auto(); LE = auto(); GE = auto(); EQEQ = auto(); NE = auto()
    BANG = auto()
    IDENT = auto(); NUMBER = auto(); STRING = auto()
    EOF = auto()

@dataclass
class Tok:
    kind: TT
    val: str = ""
    line: int = 0

KEYWORDS = {"f": TT.F, "l": TT.L, "w": TT.W}

class Lexer:
    def __init__(self, src: str):
        self.src = src
        self.i = 0
        self.line = 1

    def peek(self) -> str:
        return self.src[self.i] if self.i < len(self.src) else ""

    def advance(self) -> str:
        c = self.peek()
        if c == "\n":
            self.line += 1
        self.i += 1
        return c

    def skip_ws(self):
        while self.peek() and self.peek() in " \t\r\n":
            self.advance()

    def read_ident(self, first: str) -> str:
        s = first
        while self.peek().isalnum() or self.peek() == "_":
            s += self.advance()
        return s

    def read_number(self, first: str) -> str:
        s = first
        while self.peek().isdigit():
            s += self.advance()
        if self.peek() == ".":
            s += self.advance()
            while self.peek().isdigit():
                s += self.advance()
        return s

    def read_string(self) -> str:
        self.advance()  # skip opening "
        s = ""
        while self.peek() and self.peek() != '"':
            if self.peek() == "\\":
                self.advance()
                s += self.advance()
            else:
                s += self.advance()
        self.advance()  # closing "
        return s

    def next(self) -> Tok:
        self.skip_ws()
        line = self.line
        c = self.peek()
        if not c:
            return Tok(TT.EOF, line=line)

        if c.isalpha() or c == "_":
            ident = self.read_ident(self.advance())
            if ident in KEYWORDS:
                return Tok(KEYWORDS[ident], ident, line)
            return Tok(TT.IDENT, ident, line)

        if c.isdigit():
            return Tok(TT.NUMBER, self.read_number(self.advance()), line)

        if c == '"':
            return Tok(TT.STRING, self.read_string(), line)

        two = c + self.src[self.i + 1] if self.i + 1 < len(self.src) else ""
        if two in ("<=", ">=", "==", "!="):
            self.advance(); self.advance()
            return Tok({ "<=": TT.LE, ">=": TT.GE, "==": TT.EQEQ, "!=": TT.NE }[two], two, line)

        self.advance()
        return Tok({
            "f": TT.F, "(": TT.LP, ")": TT.RP, "{": TT.LB, "}": TT.RB,
            ",": TT.COMMA, "=": TT.EQ, "+": TT.PLUS, "-": TT.MINUS,
            "*": TT.STAR, "/": TT.SLASH, "%": TT.PERCENT, "<": TT.LT,
            ">": TT.GT, ":": TT.COLON, "?": TT.Q, "@": TT.AT, "!": TT.RET,
        }[c], c, line)


# --- AST ---

@dataclass
class Param:
    name: str
    typ: Optional[str] = None

@dataclass
class Func:
    name: str
    params: List[Param]
    ret: Optional[str]
    body: Union["Expr", "Block"]

@dataclass
class Block:
    stmts: List["Stmt"]

@dataclass
class Let:
    name: str
    expr: "Expr"

@dataclass
class While:
    cond: "Expr"
    body: Block

@dataclass
class Return:
    expr: "Expr"

Stmt = Union[Let, While, Return, "Expr"]

@dataclass
class Num:
    val: str

@dataclass
class Str:
    val: str

@dataclass
class Var:
    name: str

@dataclass
class BinOp:
    op: str
    left: "Expr"
    right: "Expr"

@dataclass
class Unary:
    op: str
    expr: "Expr"

@dataclass
class Ternary:
    cond: "Expr"
    then: "Expr"
    else_: "Expr"

@dataclass
class Call:
    fn: str
    args: List["Expr"]

@dataclass
class Assign:
    name: str
    expr: "Expr"

Expr = Union[Num, Str, Var, BinOp, Unary, Ternary, Call, Assign]

@dataclass
class Program:
    funcs: List[Func]


# --- Parser ---

class ParseError(Exception):
    def __init__(self, msg: str, line: int):
        super().__init__(f"line {line}: {msg}")
        self.line = line


class Parser:
    def __init__(self, src: str):
        self.lex = Lexer(src)
        self._peek: Optional[Tok] = None
        self.cur = self._next()

    def _next(self) -> Tok:
        if self._peek:
            t = self._peek
            self._peek = None
            return t
        return self.lex.next()

    def _lookahead(self) -> Tok:
        if not self._peek:
            self._peek = self.lex.next()
        return self._peek

    def eat(self, kind: TT) -> Tok:
        if self.cur.kind != kind:
            raise ParseError(f"expected {kind}, got {self.cur.kind}", self.cur.line)
        t = self.cur
        self.cur = self._next()
        return t

    def match(self, kind: TT) -> bool:
        if self.cur.kind == kind:
            self.cur = self._next()
            return True
        return False

    def parse(self) -> Program:
        funcs = []
        script = []
        while self.cur.kind != TT.EOF:
            if self.cur.kind == TT.F:
                funcs.append(self.func())
            else:
                script.append(self.stmt())
        if script and not any(f.name == "main" for f in funcs):
            funcs.append(Func("main", [], "v", Block(script)))
        return Program(funcs)

    def func(self) -> Func:
        self.eat(TT.F)
        name = self.eat(TT.IDENT).val
        self.eat(TT.LP)
        params = []
        if self.cur.kind == TT.IDENT:
            params = self.params()
        self.eat(TT.RP)
        ret = None
        if self.match(TT.COLON):
            ret = self.eat(TT.IDENT).val
        if self.match(TT.EQ):
            body = self.ternary()
        else:
            body = self.block()
        return Func(name, params, ret, body)

    def params(self) -> List[Param]:
        ps = [Param(self.eat(TT.IDENT).val)]
        if self.match(TT.COLON):
            ps[-1].typ = self.eat(TT.IDENT).val
        while self.match(TT.COMMA):
            p = Param(self.eat(TT.IDENT).val)
            if self.match(TT.COLON):
                p.typ = self.eat(TT.IDENT).val
            ps.append(p)
        return ps

    def block(self) -> Block:
        self.eat(TT.LB)
        stmts = []
        while self.cur.kind != TT.RB:
            stmts.append(self.stmt())
        self.eat(TT.RB)
        return Block(stmts)

    def stmt(self) -> Stmt:
        if self.cur.kind == TT.L:
            self.eat(TT.L)
            name = self.eat(TT.IDENT).val
            self.eat(TT.EQ)
            return Let(name, self.let_rhs())
        if self.cur.kind == TT.W:
            self.eat(TT.W)
            cond = self.ternary()
            return While(cond, self.block())
        if self.cur.kind == TT.RET:
            self.eat(TT.RET)
            return Return(self.ternary())
        return self.ternary()

    def let_rhs(self) -> Expr:
        if self.match(TT.Q):
            cond = self.assign()
            then = self.assign()
            else_ = self.ternary()
            return Ternary(cond, then, else_)
        return self.assign()

    def ternary(self) -> Expr:
        if self.match(TT.Q):
            cond = self.assign()
            then = self.assign()
            else_ = self.ternary()
            return Ternary(cond, then, else_)
        e = self.assign()
        if self.match(TT.Q):
            then = self.assign()
            else_ = self.assign()
            return Ternary(e, then, else_)
        return e

    def assign(self) -> Expr:
        if self.cur.kind == TT.IDENT and self._lookahead().kind == TT.EQ:
            name = self.eat(TT.IDENT).val
            self.eat(TT.EQ)
            return Assign(name, self.assign())
        return self.compare()

    def _compare_from(self, left: Expr) -> Expr:
        ops = {TT.LT: "<", TT.GT: ">", TT.LE: "<=", TT.GE: ">=", TT.EQEQ: "==", TT.NE: "!="}
        e = left
        while self.cur.kind in ops:
            op = ops[self.cur.kind]
            self.cur = self._next()
            e = BinOp(op, e, self.add())
        return e

    def compare(self) -> Expr:
        return self._compare_from(self.add())

    def add(self) -> Expr:
        e = self.mul()
        while self.cur.kind in (TT.PLUS, TT.MINUS):
            op = "+" if self.cur.kind == TT.PLUS else "-"
            self.cur = self._next()
            e = BinOp(op, e, self.mul())
        return e

    def mul(self) -> Expr:
        e = self.unary()
        while self.cur.kind in (TT.STAR, TT.SLASH, TT.PERCENT):
            op = {TT.STAR: "*", TT.SLASH: "/", TT.PERCENT: "%"}[self.cur.kind]
            self.cur = self._next()
            e = BinOp(op, e, self.unary())
        return e

    def unary(self) -> Expr:
        if self.cur.kind == TT.MINUS:
            self.cur = self._next()
            return Unary("-", self.unary())
        if self.cur.kind == TT.BANG:
            self.cur = self._next()
            return Unary("!", self.unary())
        return self.call()

    def call(self) -> Expr:
        e = self.primary()
        while True:
            if self.match(TT.LP):
                args = []
                if self.cur.kind != TT.RP:
                    args.append(self.assign())
                    while self.match(TT.COMMA):
                        args.append(self.assign())
                self.eat(TT.RP)
                if isinstance(e, Var):
                    e = Call(e.name, args)
                else:
                    raise ParseError("call on non-identifier", self.cur.line)
            elif self.cur.kind == TT.AT and isinstance(e, type) and False:
                pass
            else:
                break
        return e

    def _parse_at_call(self) -> Call:
        """Parse @(fn, args...) or @fn, arg [, arg...]"""
        self.eat(TT.AT)
        if self.match(TT.LP):
            fn = self.eat(TT.IDENT).val
            args = []
            if self.match(TT.COMMA):
                if self.cur.kind != TT.RP:
                    args.append(self.assign())
                    while self.match(TT.COMMA):
                        args.append(self.assign())
            self.eat(TT.RP)
            return Call(fn, args)
        fn = self.eat(TT.IDENT).val
        args = []
        if self.match(TT.COMMA):
            args.append(self.assign())
            while self.match(TT.COMMA):
                args.append(self.assign())
        return Call(fn, args)

    def primary(self) -> Expr:
        if self.cur.kind == TT.AT:
            return self._parse_at_call()

        if self.cur.kind == TT.NUMBER:
            v = self.cur.val
            self.cur = self._next()
            return Num(v)

        if self.cur.kind == TT.STRING:
            v = self.cur.val
            self.cur = self._next()
            return Str(v)

        if self.cur.kind == TT.IDENT:
            v = Var(self.cur.val)
            self.cur = self._next()
            return v

        if self.match(TT.LP):
            e = self.ternary()
            self.eat(TT.RP)
            return e

        raise ParseError(f"unexpected token {self.cur.kind}", self.cur.line)


# --- Codegen ---

TYPE_MAP = {"i": "int64_t", "f": "double", "b": "int", "v": "void", "s": "char*"}

BUILTINS = {
    "p": ("glyph_print_i", "i"),
    "pf": ("glyph_print_f", "f"),
    "ps": ("glyph_print_s", "s"),
}


class Codegen:
    def __init__(self, prog: Program):
        self.prog = prog
        self.func_types = {}
        for fn in prog.funcs:
            ps = [(p.name, TYPE_MAP.get(p.typ or "i", "int64_t")) for p in fn.params]
            rt = TYPE_MAP.get(fn.ret or "i", "int64_t") if fn.ret != "v" else "void"
            if fn.ret == "v" or (isinstance(fn.body, Block) and fn.name == "main"):
                rt = "void" if fn.name == "main" or fn.ret == "v" else rt
            self.func_types[fn.name] = (rt, ps)

    def gen(self) -> str:
        lines = [
            '#include "../runtime/glyph_rt.h"',
            '#include <stdint.h>',
            '',
        ]
        for fn in self.prog.funcs:
            lines.append(self.gen_func(fn))
        if not any(f.name == "main" for f in self.prog.funcs):
            lines.append("int main(void) { return 0; }")
        return "\n".join(lines) + "\n"

    def gen_func(self, fn: Func) -> str:
        rt, _ = self.func_types[fn.name]
        if fn.name == "main" and fn.ret is None:
            rt = "int"
        params = ", ".join(f"{TYPE_MAP.get(p.typ or 'i', 'int64_t')} {p.name}" for p in fn.params)
        sig = f"{rt} {fn.name}({params})"
        if isinstance(fn.body, Block):
            body = self.gen_block(fn.body, indent=1, ret=rt)
            if rt == "int" and fn.name == "main":
                body = body.rstrip() + "\n    return 0;\n"
            return f"{sig} {{\n{body}}}\n"
        expr = self.gen_expr(fn.body)
        if rt == "void":
            return f"{sig} {{ {expr}; }}\n"
        return f"{rt} {fn.name}({params}) {{ return {expr}; }}\n"

    def gen_block(self, block: Block, indent: int = 0, ret: str = "int64_t") -> str:
        pad = "    " * indent
        lines = []
        for s in block.stmts:
            if isinstance(s, Let):
                lines.append(f"{pad}int64_t {s.name} = {self.gen_expr(s.expr)};")
            elif isinstance(s, While):
                lines.append(f"{pad}while ({self.gen_expr(s.cond)}) {{")
                lines.append(self.gen_block(s.body, indent + 1, ret))
                lines.append(f"{pad}}}")
            elif isinstance(s, Return):
                e = self.gen_expr(s.expr)
                if ret == "void":
                    lines.append(f"{pad}(void)({e}); return;")
                else:
                    lines.append(f"{pad}return {e};")
            elif isinstance(s, Ternary):
                lines.extend(self.gen_if_else(s, indent))
            else:
                e = self.gen_expr(s)
                if ret != "void":
                    lines.append(f"{pad}(void)({e});")
                else:
                    lines.append(f"{pad}{e};")
        return "\n".join(lines) + ("\n" if lines else "")

    def gen_if_else(self, t: Ternary, indent: int) -> List[str]:
        pad = "    " * indent
        lines = [f"{pad}if ({self.gen_expr(t.cond)}) {{"]
        lines.extend(self.gen_stmt_lines(t.then, indent + 1))
        lines.append(f"{pad}}} else {{")
        lines.extend(self.gen_stmt_lines(t.else_, indent + 1))
        lines.append(f"{pad}}}")
        return lines

    def gen_stmt_lines(self, e: Expr, indent: int) -> List[str]:
        pad = "    " * indent
        if isinstance(e, Ternary):
            return self.gen_if_else(e, indent)
        return [f"{pad}{self.gen_expr(e)};"]

    def gen_expr(self, e: Expr) -> str:
        if isinstance(e, Num):
            return e.val + (".0" if "." in e.val else "")
        if isinstance(e, Str):
            return f'"{e.val}"'
        if isinstance(e, Var):
            return e.name
        if isinstance(e, BinOp):
            l, r = self.gen_expr(e.left), self.gen_expr(e.right)
            if e.op in ("==", "!=", "<", ">", "<=", ">="):
                return f"({l} {e.op} {r})"
            return f"({l} {e.op} {r})"
        if isinstance(e, Unary):
            if e.op == "!":
                return f"(!{self.gen_expr(e.expr)})"
            return f"(-{self.gen_expr(e.expr)})"
        if isinstance(e, Ternary):
            c, t, el = self.gen_expr(e.cond), self.gen_expr(e.then), self.gen_expr(e.else_)
            return f"({c} ? {t} : {el})"
        if isinstance(e, Call):
            if e.fn in BUILTINS:
                c_fn, _ = BUILTINS[e.fn]
                if e.fn == "p":
                    return f"glyph_print_i({self.gen_expr(e.args[0])})"
                if e.fn == "pf":
                    return f"glyph_print_f({self.gen_expr(e.args[0])})"
                if e.fn == "ps":
                    return f'glyph_print_s({self.gen_expr(e.args[0])})'
            args = ", ".join(self.gen_expr(a) for a in e.args)
            return f"{e.fn}({args})"
        if isinstance(e, Assign):
            return f"({e.name} = {self.gen_expr(e.expr)})"
        return "0"


def compile_source(src: str, out_path: str, rt_dir: str) -> None:
    prog = Parser(src).parse()
    c_code = Codegen(prog).gen()
    rt_header = os.path.join(rt_dir, "glyph_rt.h")
    with tempfile.TemporaryDirectory() as tmp:
        c_path = os.path.join(tmp, "out.c")
        with open(c_path, "w") as f:
            f.write(c_code.replace('"../runtime/glyph_rt.h"', f'"{rt_header}"'))
        cmd = ["gcc", "-O2", "-std=c11", "-o", out_path, c_path]
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode != 0:
            print(r.stderr, file=sys.stderr)
            sys.exit(1)


def main():
    if len(sys.argv) < 3:
        print("usage: glyphc.py <input.gl> -o <output>", file=sys.stderr)
        sys.exit(1)
    in_path = sys.argv[1]
    out_path = None
    for i, a in enumerate(sys.argv):
        if a == "-o" and i + 1 < len(sys.argv):
            out_path = sys.argv[i + 1]
    if not out_path:
        print("usage: glyphc.py <input.gl> -o <output>", file=sys.stderr)
        sys.exit(1)
    rt_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "runtime")
    with open(in_path) as f:
        src = f.read()
    try:
        compile_source(src, out_path, rt_dir)
    except ParseError as e:
        print(f"parse error: {e}", file=sys.stderr)
        sys.exit(1)
    print(f"compiled {in_path} -> {out_path}")


if __name__ == "__main__":
    main()
