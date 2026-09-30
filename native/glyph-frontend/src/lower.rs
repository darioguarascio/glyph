use crate::ast::*;
use glyph_ir::{Builtin, Function, GlobalKind, GlobalSeg, Module, Op};
use std::collections::HashMap;

#[derive(Debug)]
pub enum LowerError {
    UnknownVar(String),
    UnknownFn(String),
    UnknownGlobal(String),
    Unsupported(String),
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVar(n) => write!(f, "unknown variable {n}"),
            Self::UnknownFn(n) => write!(f, "unknown function {n}"),
            Self::UnknownGlobal(n) => write!(f, "unknown global {n}"),
            Self::Unsupported(m) => write!(f, "unsupported: {m}"),
        }
    }
}

impl std::error::Error for LowerError {}

struct Emitter {
    code: Vec<u8>,
}

impl Emitter {
    fn new() -> Self {
        Self { code: Vec::new() }
    }

    fn pos(&self) -> usize {
        self.code.len()
    }

    fn op(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    fn u8(&mut self, v: u8) {
        self.code.push(v);
    }

    fn u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn u32(&mut self, v: u32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn i32_placeholder(&mut self) -> usize {
        let p = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        p
    }

    fn patch_i32(&mut self, at: usize, target: usize) {
        let rel = (target as i64 - (at as i64 + 4)) as i32;
        self.code[at..at + 4].copy_from_slice(&rel.to_le_bytes());
    }

    fn const_i64(&mut self, v: i64) {
        self.op(Op::ConstI64);
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn const_str(&mut self, idx: u16) {
        self.op(Op::ConstStr);
        self.u16(idx);
    }

    fn jmp_ifnot(&mut self) -> usize {
        self.op(Op::JmpIfNot);
        self.i32_placeholder()
    }

    fn jmp(&mut self) -> usize {
        self.op(Op::Jmp);
        self.i32_placeholder()
    }

    fn finish(self) -> Vec<u8> {
        self.code
    }
}

struct FnCtx {
    slots: HashMap<String, u16>,
    next: u16,
}

impl FnCtx {
    fn new(params: &[Param]) -> Self {
        let mut slots = HashMap::new();
        for (i, p) in params.iter().enumerate() {
            slots.insert(p.name.clone(), i as u16);
        }
        Self {
            slots,
            next: params.len() as u16,
        }
    }

    fn alloc(&mut self, name: &str) -> u16 {
        let s = self.next;
        self.next += 1;
        self.slots.insert(name.to_string(), s);
        s
    }

    fn slot(&self, name: &str) -> Option<u16> {
        self.slots.get(name).copied()
    }
}

pub struct Lowerer {
    strings: Vec<String>,
    str_map: HashMap<String, u16>,
    globals: Vec<GlobalSeg>,
    glob_map: HashMap<String, u16>,
    fn_map: HashMap<String, u16>,
    prog: Program,
}

impl Lowerer {
    pub fn lower(prog: Program) -> Result<Module, LowerError> {
        let mut lo = Lowerer {
            strings: Vec::new(),
            str_map: HashMap::new(),
            globals: Vec::new(),
            glob_map: HashMap::new(),
            fn_map: HashMap::new(),
            prog,
        };
        lo.prepare_globals();
        lo.prepare_functions();
        lo.lower_all_functions()
    }

    fn intern(&mut self, s: &str) -> u16 {
        if let Some(&i) = self.str_map.get(s) {
            return i;
        }
        let i = self.strings.len() as u16;
        self.strings.push(s.to_string());
        self.str_map.insert(s.to_string(), i);
        i
    }

    fn prepare_globals(&mut self) {
        let globals = self.prog.globals.clone();
        for g in &globals {
            let name_idx = self.intern(&g.name);
            let idx = self.globals.len() as u16;
            self.glob_map.insert(g.name.clone(), idx);
            self.globals.push(GlobalSeg {
                kind: if g.byte {
                    GlobalKind::ByteArray
                } else {
                    GlobalKind::I64Array
                },
                name_idx,
                size: g.size,
            });
        }
    }

    fn prepare_functions(&mut self) {
        let funcs = self.prog.funcs.clone();
        for f in &funcs {
            self.intern(&f.name);
        }
        for (i, f) in funcs.iter().enumerate() {
            self.fn_map.insert(f.name.clone(), i as u16);
        }
    }

    fn lower_all_functions(mut self) -> Result<Module, LowerError> {
        let mut functions = Vec::new();
        for f in self.prog.funcs.clone() {
            functions.push(self.lower_function(&f)?);
        }
        Ok(Module {
            flags: 0,
            strings: self.strings,
            globals: self.globals,
            functions,
        })
    }

    fn lower_function(&mut self, f: &Func) -> Result<Function, LowerError> {
        let name_idx = self.intern(&f.name);
        let mut ctx = FnCtx::new(&f.params);
        let mut em = Emitter::new();

        match &f.body {
            Body::Expr(e) => {
                self.lower_expr(&mut em, &mut ctx, e)?;
                em.op(Op::Ret);
            }
            Body::Block(b) => {
                self.lower_block(&mut em, &mut ctx, b, false)?;
                em.const_i64(0);
                em.op(Op::Ret);
            }
        }

        Ok(Function {
            name_idx,
            arity: f.params.len() as u8,
            slot_count: ctx.next,
            code: em.finish(),
        })
    }

    fn lower_block(
        &mut self,
        em: &mut Emitter,
        ctx: &mut FnCtx,
        block: &Block,
        as_expr: bool,
    ) -> Result<(), LowerError> {
        let n = block.stmts.len();
        for (i, stmt) in block.stmts.iter().enumerate() {
            let keep = as_expr && i + 1 == n;
            self.lower_stmt(em, ctx, stmt, keep)?;
        }
        if as_expr && block.stmts.is_empty() {
            em.const_i64(0);
        }
        Ok(())
    }

    fn lower_stmt(
        &mut self,
        em: &mut Emitter,
        ctx: &mut FnCtx,
        stmt: &Stmt,
        keep_value: bool,
    ) -> Result<(), LowerError> {
        match stmt {
            Stmt::Let { name, expr } => {
                self.lower_expr(em, ctx, expr)?;
                let slot = ctx.alloc(name);
                em.op(Op::StoreLocal);
                em.u16(slot);
                if keep_value {
                    em.op(Op::LoadLocal);
                    em.u16(slot);
                }
            }
            Stmt::While { cond, body } => {
                let loop_start = em.pos();
                self.lower_expr(em, ctx, cond)?;
                let exit_patch = em.jmp_ifnot();
                self.lower_block(em, ctx, body, false)?;
                let back_patch = em.jmp();
                em.patch_i32(back_patch, loop_start);
                em.patch_i32(exit_patch, em.pos());
            }
            Stmt::Return(expr) => {
                self.lower_expr(em, ctx, expr)?;
                em.op(Op::Ret);
            }
            Stmt::Expr(expr) => {
                self.lower_ternary_stmt(em, ctx, expr, keep_value)?;
            }
        }
        Ok(())
    }

    fn lower_ternary_stmt(
        &mut self,
        em: &mut Emitter,
        ctx: &mut FnCtx,
        expr: &Expr,
        keep_value: bool,
    ) -> Result<(), LowerError> {
        if let Expr::Ternary { cond, then_, else_ } = expr {
            self.lower_expr(em, ctx, cond)?;
            let else_patch = em.jmp_ifnot();
            self.lower_branch(em, ctx, then_, keep_value)?;
            let end_patch = em.jmp();
            em.patch_i32(else_patch, em.pos());
            self.lower_branch(em, ctx, else_, keep_value)?;
            em.patch_i32(end_patch, em.pos());
            return Ok(());
        }
        self.lower_expr(em, ctx, expr)?;
        if !keep_value && stmt_needs_pop(expr) {
            em.op(Op::Pop);
        }
        Ok(())
    }

    fn lower_branch(
        &mut self,
        em: &mut Emitter,
        ctx: &mut FnCtx,
        expr: &Expr,
        keep_value: bool,
    ) -> Result<(), LowerError> {
        match expr {
            Expr::Block(b) => self.lower_block(em, ctx, b, keep_value),
            Expr::Ternary { .. } => self.lower_ternary_stmt(em, ctx, expr, keep_value),
            other => {
                self.lower_expr(em, ctx, other)?;
                if !keep_value && stmt_needs_pop(other) {
                    em.op(Op::Pop);
                }
                Ok(())
            }
        }
    }

    fn lower_expr(&mut self, em: &mut Emitter, ctx: &mut FnCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Num(n) => {
                let v: i64 = if n.contains('.') {
                    n.parse::<f64>().unwrap_or(0.0) as i64
                } else {
                    n.parse().unwrap_or(0)
                };
                em.const_i64(v);
            }
            Expr::Str(s) => {
                let idx = self.intern(s);
                em.const_str(idx);
            }
            Expr::Var(name) => {
                let slot = ctx.slot(name).ok_or_else(|| LowerError::UnknownVar(name.clone()))?;
                em.op(Op::LoadLocal);
                em.u16(slot);
            }
            Expr::BinOp { op, left, right } => {
                self.lower_expr(em, ctx, left)?;
                self.lower_expr(em, ctx, right)?;
                em.op(match op.as_str() {
                    "+" => Op::Add,
                    "-" => Op::Sub,
                    "*" => Op::Mul,
                    "/" => Op::Div,
                    "%" => Op::Mod,
                    "<" => Op::CmpLt,
                    ">" => Op::CmpGt,
                    "<=" => Op::CmpLe,
                    ">=" => Op::CmpGe,
                    "==" => Op::CmpEq,
                    "!=" => Op::CmpNe,
                    _ => return Err(LowerError::Unsupported(op.clone())),
                });
            }
            Expr::Unary { op, expr } => {
                self.lower_expr(em, ctx, expr)?;
                em.op(match op.as_str() {
                    "-" => Op::Neg,
                    "!" => Op::Not,
                    _ => return Err(LowerError::Unsupported(op.clone())),
                });
            }
            Expr::Ternary { cond, then_, else_ } => {
                self.lower_expr(em, ctx, cond)?;
                let else_patch = em.jmp_ifnot();
                self.lower_expr(em, ctx, then_)?;
                let end_patch = em.jmp();
                em.patch_i32(else_patch, em.pos());
                self.lower_expr(em, ctx, else_)?;
                em.patch_i32(end_patch, em.pos());
            }
            Expr::Call { name, args } => self.lower_call(em, ctx, name, args)?,
            Expr::Assign { name, expr } => {
                self.lower_expr(em, ctx, expr)?;
                let slot = ctx.slot(name).ok_or_else(|| LowerError::UnknownVar(name.clone()))?;
                em.op(Op::StoreLocal);
                em.u16(slot);
                em.op(Op::LoadLocal);
                em.u16(slot);
            }
            Expr::Index { name, idx } => {
                let g = self
                    .glob_map
                    .get(name)
                    .copied()
                    .ok_or_else(|| LowerError::UnknownGlobal(name.clone()))?;
                self.lower_expr(em, ctx, idx)?;
                em.op(Op::LoadGlobIdx);
                em.u16(g);
            }
            Expr::IndexAssign { name, idx, expr } => {
                let g = self
                    .glob_map
                    .get(name)
                    .copied()
                    .ok_or_else(|| LowerError::UnknownGlobal(name.clone()))?;
                self.lower_expr(em, ctx, expr)?;
                self.lower_expr(em, ctx, idx)?;
                em.op(Op::StoreGlobIdx);
                em.u16(g);
            }
            Expr::Block(b) => self.lower_block(em, ctx, b, true)?,
        }
        Ok(())
    }

    fn lower_call(
        &mut self,
        em: &mut Emitter,
        ctx: &mut FnCtx,
        name: &str,
        args: &[Expr],
    ) -> Result<(), LowerError> {
        if let Some(b) = builtin_id(name) {
            for arg in args {
                self.lower_builtin_arg(em, ctx, arg)?;
            }
            em.op(Op::CallBuiltin);
            em.u8(b);
            em.u8(args.len() as u8);
            return Ok(());
        }
        for arg in args {
            self.lower_expr(em, ctx, arg)?;
        }
        let fn_idx = self
            .fn_map
            .get(name)
            .copied()
            .ok_or_else(|| LowerError::UnknownFn(name.to_string()))?;
        em.op(Op::Call);
        em.u16(fn_idx);
        em.u8(args.len() as u8);
        Ok(())
    }

    fn lower_builtin_arg(
        &mut self,
        em: &mut Emitter,
        ctx: &mut FnCtx,
        arg: &Expr,
    ) -> Result<(), LowerError> {
        if let Expr::Str(s) = arg {
            let idx = self.intern(s);
            em.const_str(idx);
        } else {
            self.lower_expr(em, ctx, arg)?;
        }
        Ok(())
    }
}

fn is_void_builtin(name: &str) -> bool {
    matches!(name, "p" | "ps" | "o" | "pf")
}

fn stmt_needs_pop(expr: &Expr) -> bool {
    match expr {
        Expr::Call { name, .. } => !is_void_builtin(name),
        Expr::Assign { .. } | Expr::BinOp { .. } | Expr::Unary { .. } | Expr::Var(_)
        | Expr::Num(_) | Expr::Str(_) | Expr::Index { .. } => true,
        Expr::Ternary { .. } | Expr::Block(_) | Expr::IndexAssign { .. } => false,
    }
}

fn builtin_id(name: &str) -> Option<u8> {
    match name {
        "p" => Some(Builtin::PrintI64 as u8),
        "ps" => Some(Builtin::PrintStr as u8),
        "o" => Some(Builtin::PutChar as u8),
        "r" => Some(Builtin::GetChar as u8),
        "len" => Some(Builtin::StrLen as u8),
        "ch" => Some(Builtin::CharAt as u8),
        "argc" => Some(Builtin::Argc as u8),
        "argv" => Some(Builtin::Argv as u8),
        _ => None,
    }
}
