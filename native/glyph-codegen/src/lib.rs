//! Compile GBC modules to native object files via Cranelift.

mod link;

pub use link::{build_executable, link_executable};

use anyhow::{anyhow, Result};
use cranelift::codegen::ir::condcodes::IntCC;
use cranelift::codegen::ir::{types, AbiParam, FuncRef, InstBuilder, MemFlags, StackSlot, Value};
use cranelift::codegen::verify_function;
use cranelift::codegen::settings::{self, Configurable};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use glyph_ir::{Builtin, Decoder, Function as GbcFn, GlobalKind, Module as GbcModule, Op};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct NativeObject {
    pub bytes: Vec<u8>,
}

pub fn compile_module(gbc: &GbcModule) -> Result<NativeObject> {
    gbc.verify().map_err(|e| anyhow!("{e}"))?;
    let mut cg = Codegen::new()?;
    cg.emit(gbc)?;
    Ok(NativeObject {
        bytes: cg.module.finish().emit()?,
    })
}

struct Codegen {
    module: ObjectModule,
    func_ids: Vec<FuncId>,
    global_ids: Vec<cranelift_module::DataId>,
    str_ids: Vec<cranelift_module::DataId>,
    rt: RuntimeFuncs,
}

struct RuntimeFuncs {
    print_i: FuncId,
    print_str: FuncId,
    putchar: FuncId,
    getchar: FuncId,
    strlen: FuncId,
    char_at: FuncId,
    argc: FuncId,
    argv: FuncId,
}

struct RuntimeRefs {
    print_i: FuncRef,
    print_str: FuncRef,
    putchar: FuncRef,
    getchar: FuncRef,
    strlen: FuncRef,
    char_at: FuncRef,
    argc: FuncRef,
    argv: FuncRef,
}

impl Codegen {
    fn new() -> Result<Self> {
        let mut flagbuilder = settings::builder();
        flagbuilder.set("is_pic", "false").ok();
        flagbuilder.set("opt_level", "speed").ok();
        let isa = cranelift_native::builder()
            .map_err(|e| anyhow!("host ISA: {e}"))?
            .finish(settings::Flags::new(flagbuilder))?;
        let builder = ObjectBuilder::new(isa, "glyph", cranelift_module::default_libcall_names())?;
        let mut module = ObjectModule::new(builder);
        let rt = declare_runtime(&mut module)?;
        Ok(Self {
            module,
            func_ids: Vec::new(),
            global_ids: Vec::new(),
            str_ids: Vec::new(),
            rt,
        })
    }

    fn emit(&mut self, gbc: &GbcModule) -> Result<()> {
        for (i, s) in gbc.strings.iter().enumerate() {
            let name = format!("_gstr_{i}");
            let id = self
                .module
                .declare_data(&name, Linkage::Local, false, false)?;
            let mut data = DataDescription::new();
            let mut bytes = s.as_bytes().to_vec();
            bytes.push(0);
            data.define(bytes.into_boxed_slice());
            self.module.define_data(id, &data)?;
            self.str_ids.push(id);
        }

        for (i, g) in gbc.globals.iter().enumerate() {
            let name = format!("_gglob_{i}");
            let id = self
                .module
                .declare_data(&name, Linkage::Local, true, false)?;
            let mut data = DataDescription::new();
            let size = g.size as usize * if g.kind == GlobalKind::ByteArray { 1 } else { 8 };
            data.define_zeroinit(size);
            self.module.define_data(id, &data)?;
            self.global_ids.push(id);
        }

        for (i, f) in gbc.functions.iter().enumerate() {
            let name = format!("_gfn_{i}");
            let mut sig = self.module.make_signature();
            for _ in 0..f.arity {
                sig.params.push(AbiParam::new(types::I64));
            }
            sig.returns.push(AbiParam::new(types::I64));
            let id = self
                .module
                .declare_function(&name, Linkage::Export, &sig)?;
            self.func_ids.push(id);
        }

        for (i, f) in gbc.functions.iter().enumerate() {
            self.emit_function(gbc, i, f)?;
        }
        self.emit_entry(gbc)?;
        Ok(())
    }

    fn emit_entry(&mut self, gbc: &GbcModule) -> Result<()> {
        let entry = gbc.entry_fn();
        let target = self.func_ids[entry];
        let mut sig = self.module.make_signature();
        sig.returns.push(AbiParam::new(types::I64));
        let entry_id = self
            .module
            .declare_function("_glyph_entry", Linkage::Export, &sig)?;
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        let mut bctx = cranelift_frontend::FunctionBuilderContext::new();
        let mut builder = cranelift_frontend::FunctionBuilder::new(&mut ctx.func, &mut bctx);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);
        let fref = self.module.declare_func_in_func(target, &mut builder.func);
        let call = builder.ins().call(fref, &[]);
        let rv = builder.inst_results(call)[0];
        builder.ins().return_(&[rv]);
        builder.finalize();
        self.module
            .define_function(entry_id, &mut ctx)
            .map_err(|e| anyhow!("entry: {e}"))?;
        self.module.clear_context(&mut ctx);
        Ok(())
    }

    fn emit_function(&mut self, _gbc: &GbcModule, fn_idx: usize, f: &GbcFn) -> Result<()> {
        let func_id = self.func_ids[fn_idx];
        let mut ctx = self.module.make_context();
        ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();

        let mut builder_ctx = cranelift_frontend::FunctionBuilderContext::new();
        let mut builder = cranelift_frontend::FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);

        let mut locals = Vec::new();
        for _ in 0..f.slot_count {
            locals.push(builder.create_sized_stack_slot(
                cranelift::codegen::ir::StackSlotData::new(
                    cranelift::codegen::ir::StackSlotKind::ExplicitSlot,
                    8,
                    0,
                ),
            ));
        }

        let params: Vec<Value> = builder.block_params(entry).to_vec();
        for (i, p) in params.iter().enumerate() {
            if i < locals.len() {
                builder.ins().stack_store(*p, locals[i], 0);
            }
        }

        let global_ptrs: Vec<Value> = self
            .global_ids
            .iter()
            .map(|id| {
                let gv = self.module.declare_data_in_func(*id, &mut builder.func);
                builder.ins().symbol_value(types::I64, gv)
            })
            .collect();

        let str_ptrs: Vec<Value> = self
            .str_ids
            .iter()
            .map(|id| {
                let gv = self.module.declare_data_in_func(*id, &mut builder.func);
                builder.ins().symbol_value(types::I64, gv)
            })
            .collect();

        let fn_refs: Vec<FuncRef> = self
            .func_ids
            .iter()
            .map(|id| self.module.declare_func_in_func(*id, &mut builder.func))
            .collect();

        let rt_refs = RuntimeRefs {
            print_i: self.module.declare_func_in_func(self.rt.print_i, &mut builder.func),
            print_str: self.module.declare_func_in_func(self.rt.print_str, &mut builder.func),
            putchar: self.module.declare_func_in_func(self.rt.putchar, &mut builder.func),
            getchar: self.module.declare_func_in_func(self.rt.getchar, &mut builder.func),
            strlen: self.module.declare_func_in_func(self.rt.strlen, &mut builder.func),
            char_at: self.module.declare_func_in_func(self.rt.char_at, &mut builder.func),
            argc: self.module.declare_func_in_func(self.rt.argc, &mut builder.func),
            argv: self.module.declare_func_in_func(self.rt.argv, &mut builder.func),
        };

        let max_stack = max_stack_depth(&f.code).max(8);
        let stack_data = builder.create_sized_stack_slot(
            cranelift::codegen::ir::StackSlotData::new(
                cranelift::codegen::ir::StackSlotKind::ExplicitSlot,
                (max_stack * 8) as u32,
                0,
            ),
        );
        let sp_slot = builder.create_sized_stack_slot(
            cranelift::codegen::ir::StackSlotData::new(
                cranelift::codegen::ir::StackSlotKind::ExplicitSlot,
                8,
                0,
            ),
        );
        let sp_zero = builder.ins().iconst(types::I64, 0);
        builder.ins().stack_store(sp_zero, sp_slot, 0);

        let mut block_map: HashMap<usize, cranelift::codegen::ir::Block> = HashMap::new();
        let first_bb = block_map
            .entry(0)
            .or_insert_with(|| builder.create_block());
        builder.ins().jump(*first_bb, &[]);

        let mut gen = FuncGen {
            builder: &mut builder,
            fn_refs: &fn_refs,
            rt: &rt_refs,
            str_ptrs: &str_ptrs,
            global_ptrs: &global_ptrs,
            locals: &locals,
            stack_data,
            sp_slot,
        };
        let _returned = gen.emit_code(&f.code, &mut block_map)?;
        builder.finalize();

        if let Err(errors) = verify_function(&ctx.func, self.module.isa()) {
            return Err(anyhow!(
                "define fn {fn_idx} verifier:\n{errors}\n{}",
                ctx.func.display()
            ));
        }
        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| anyhow!("define fn {fn_idx}: {e}"))?;
        self.module.clear_context(&mut ctx);
        Ok(())
    }
}

struct FuncGen<'f, 'b> {
    builder: &'b mut cranelift_frontend::FunctionBuilder<'f>,
    fn_refs: &'b [FuncRef],
    rt: &'b RuntimeRefs,
    str_ptrs: &'b [Value],
    global_ptrs: &'b [Value],
    locals: &'b [StackSlot],
    stack_data: StackSlot,
    sp_slot: StackSlot,
}

impl<'f, 'b> FuncGen<'f, 'b> {
    fn emit_code(
        &mut self,
        code: &[u8],
        block_map: &mut HashMap<usize, cranelift::codegen::ir::Block>,
    ) -> Result<bool> {
        let mut returned = false;
        let mut emitted = HashSet::new();
        let mut pending: VecDeque<usize> = VecDeque::new();
        pending.push_back(0);

        while let Some(start) = pending.pop_front() {
            if emitted.contains(&start) {
                continue;
            }
            emitted.insert(start);
            let bb = *block_map
                .entry(start)
                .or_insert_with(|| self.builder.create_block());
            self.builder.switch_to_block(bb);

            let mut dec = Decoder::new(&code[start..]);
            let base = start;
            loop {
                if dec.remaining() == 0 {
                    let tail = self.peek_or_zero();
                    self.builder.ins().return_(&[tail]);
                    returned = true;
                    break;
                }
                let op_ip = base + dec.ip;
                let op = dec.read_op().map_err(|e| anyhow!("{e}"))?;
                match op {
                    Op::Nop => {}
                    Op::Halt => {
                        let zero = self.builder.ins().iconst(types::I64, 0);
                        self.push(zero);
                        returned = true;
                        break;
                    }
                    Op::ConstI64 => {
                        let v = dec.read_i64().map_err(|e| anyhow!("{e}"))?;
                        let cv = self.builder.ins().iconst(types::I64, v);
                        self.push(cv);
                    }
                    Op::ConstStr => {
                        let idx = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        self.push(self.str_ptrs[idx]);
                    }
                    Op::Pop => {
                        self.pop()?;
                    }
                    Op::Dup => {
                        let v = self.peek()?;
                        self.push(v);
                    }
                    Op::LoadLocal => {
                        let slot = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let v = self.load_local(slot)?;
                        self.push(v);
                    }
                    Op::StoreLocal => {
                        let slot = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let v = self.pop()?;
                        self.store_local(slot, v)?;
                    }
                    Op::LoadGlob => {
                        let g = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let off = dec.read_u32().map_err(|e| anyhow!("{e}"))? as i64;
                        let v = self.load_global(g, off)?;
                        self.push(v);
                    }
                    Op::StoreGlob => {
                        let g = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let off = dec.read_u32().map_err(|e| anyhow!("{e}"))? as i64;
                        let v = self.pop()?;
                        self.store_global(g, off, v)?;
                    }
                    Op::LoadGlobIdx => {
                        let g = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let idx = self.pop()?;
                        let v = self.load_global_idx(g, idx)?;
                        self.push(v);
                    }
                    Op::StoreGlobIdx => {
                        let g = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let idx = self.pop()?;
                        let v = self.pop()?;
                        self.store_global_idx(g, idx, v)?;
                    }
                    Op::Add => self.binop(|b, a, c| b.ins().iadd(a, c))?,
                    Op::Sub => self.binop(|b, a, c| b.ins().isub(a, c))?,
                    Op::Mul => self.binop(|b, a, c| b.ins().imul(a, c))?,
                    Op::Div => self.binop_div()?,
                    Op::Mod => self.binop_mod()?,
                    Op::Neg => {
                        let a = self.pop()?;
                        let nv = self.builder.ins().ineg(a);
                        self.push(nv);
                    }
                    Op::Not => {
                        let a = self.pop()?;
                        let zero = self.builder.ins().iconst(types::I64, 0);
                        let eq = self.builder.ins().icmp(IntCC::Equal, a, zero);
                        let rv = self.builder.ins().uextend(types::I64, eq);
                        self.push(rv);
                    }
                    Op::CmpLt => self.cmp(IntCC::SignedLessThan)?,
                    Op::CmpGt => self.cmp(IntCC::SignedGreaterThan)?,
                    Op::CmpLe => self.cmp(IntCC::SignedLessThanOrEqual)?,
                    Op::CmpGe => self.cmp(IntCC::SignedGreaterThanOrEqual)?,
                    Op::CmpEq => self.cmp(IntCC::Equal)?,
                    Op::CmpNe => self.cmp(IntCC::NotEqual)?,
                    Op::Jmp => {
                        let rel = dec.read_i32().map_err(|e| anyhow!("{e}"))?;
                        let target = (op_ip + 5) as i32 + rel;
                        self.br_to(block_map, target as usize, &mut pending)?;
                        break;
                    }
                    Op::JmpIf => {
                        let rel = dec.read_i32().map_err(|e| anyhow!("{e}"))?;
                        let cond = self.pop()?;
                        let zero = self.builder.ins().iconst(types::I64, 0);
                        let t = self.builder.ins().icmp(IntCC::NotEqual, cond, zero);
                        let target = (op_ip + 5) as i32 + rel;
                        let fb = self.ensure_block(block_map, op_ip + 5, &mut pending);
                        let tb = self.ensure_block(block_map, target as usize, &mut pending);
                        self.builder.ins().brif(t, tb, &[], fb, &[]);
                        break;
                    }
                    Op::JmpIfNot => {
                        let rel = dec.read_i32().map_err(|e| anyhow!("{e}"))?;
                        let cond = self.pop()?;
                        let zero = self.builder.ins().iconst(types::I64, 0);
                        let t = self.builder.ins().icmp(IntCC::Equal, cond, zero);
                        let target = (op_ip + 5) as i32 + rel;
                        let fb = self.ensure_block(block_map, op_ip + 5, &mut pending);
                        let tb = self.ensure_block(block_map, target as usize, &mut pending);
                        self.builder.ins().brif(t, tb, &[], fb, &[]);
                        break;
                    }
                    Op::Call => {
                        let fn_idx = dec.read_u16().map_err(|e| anyhow!("{e}"))? as usize;
                        let argc = dec.read_u8().map_err(|e| anyhow!("{e}"))? as usize;
                        let mut args: Vec<Value> = (0..argc).map(|_| self.pop()).collect::<Result<_>>()?;
                        args.reverse();
                        let call = self.builder.ins().call(self.fn_refs[fn_idx], &args);
                        let rv = self.builder.inst_results(call)[0];
                        self.push(rv);
                    }
                    Op::Ret => {
                        let v = self.peek_or_zero();
                        self.builder.ins().return_(&[v]);
                        returned = true;
                        break;
                    }
                    Op::CallBuiltin => {
                        let id = dec.read_u8().map_err(|e| anyhow!("{e}"))?;
                        let _argc = dec.read_u8().map_err(|e| anyhow!("{e}"))?;
                        self.emit_builtin(id)?;
                    }
                    Op::Syscall => {
                        return Err(anyhow!("syscalls not supported in native codegen yet"));
                    }
                }
            }
        }
        self.builder.seal_all_blocks();
        Ok(returned)
    }

    fn ensure_block(
        &mut self,
        block_map: &mut HashMap<usize, cranelift::codegen::ir::Block>,
        start: usize,
        pending: &mut VecDeque<usize>,
    ) -> cranelift::codegen::ir::Block {
        if !block_map.contains_key(&start) {
            pending.push_back(start);
        }
        *block_map
            .entry(start)
            .or_insert_with(|| self.builder.create_block())
    }

    fn br_to(
        &mut self,
        block_map: &mut HashMap<usize, cranelift::codegen::ir::Block>,
        target: usize,
        pending: &mut VecDeque<usize>,
    ) -> Result<()> {
        let b = self.ensure_block(block_map, target, pending);
        self.builder.ins().jump(b, &[]);
        Ok(())
    }

    fn push(&mut self, v: Value) {
        let sp = self
            .builder
            .ins()
            .stack_load(types::I64, self.sp_slot, 0);
        let addr = self.stack_addr(sp);
        self.builder
            .ins()
            .store(MemFlags::trusted(), v, addr, 0);
        let sp1 = self.builder.ins().iadd_imm(sp, 1);
        self.builder.ins().stack_store(sp1, self.sp_slot, 0);
    }

    fn pop(&mut self) -> Result<Value> {
        let sp = self
            .builder
            .ins()
            .stack_load(types::I64, self.sp_slot, 0);
        let sp1 = self.builder.ins().iadd_imm(sp, -1);
        self.builder.ins().stack_store(sp1, self.sp_slot, 0);
        let addr = self.stack_addr(sp1);
        Ok(self
            .builder
            .ins()
            .load(types::I64, MemFlags::trusted(), addr, 0))
    }

    fn peek(&mut self) -> Result<Value> {
        let sp = self
            .builder
            .ins()
            .stack_load(types::I64, self.sp_slot, 0);
        let sp1 = self.builder.ins().iadd_imm(sp, -1);
        let addr = self.stack_addr(sp1);
        Ok(self
            .builder
            .ins()
            .load(types::I64, MemFlags::trusted(), addr, 0))
    }

    fn peek_or_zero(&mut self) -> Value {
        let sp = self
            .builder
            .ins()
            .stack_load(types::I64, self.sp_slot, 0);
        let zero_v = self.builder.ins().iconst(types::I64, 0);
        let is_empty = self.builder.ins().icmp(IntCC::Equal, sp, zero_v);
        let sp1 = self.builder.ins().iadd_imm(sp, -1);
        let addr = self.stack_addr(sp1);
        let top = self
            .builder
            .ins()
            .load(types::I64, MemFlags::trusted(), addr, 0);
        self.builder.ins().select(is_empty, zero_v, top)
    }

    fn stack_addr(&mut self, sp: Value) -> Value {
        let base = self
            .builder
            .ins()
            .stack_addr(types::I64, self.stack_data, 0);
        let off = self.builder.ins().imul_imm(sp, 8);
        self.builder.ins().iadd(base, off)
    }

    fn load_local(&mut self, slot: usize) -> Result<Value> {
        let ss = *self
            .locals
            .get(slot)
            .ok_or_else(|| anyhow!("bad local {slot}"))?;
        Ok(self.builder.ins().stack_load(types::I64, ss, 0))
    }

    fn store_local(&mut self, slot: usize, v: Value) -> Result<()> {
        let ss = *self
            .locals
            .get(slot)
            .ok_or_else(|| anyhow!("bad local {slot}"))?;
        self.builder.ins().stack_store(v, ss, 0);
        Ok(())
    }

    fn load_global(&mut self, g: usize, off: i64) -> Result<Value> {
        let base = self.global_ptrs[g];
        let addr = if off == 0 {
            base
        } else {
            self.builder
                .ins()
                .iadd_imm(base, off * 8)
        };
        Ok(self
            .builder
            .ins()
            .load(types::I64, MemFlags::trusted(), addr, 0))
    }

    fn store_global(&mut self, g: usize, off: i64, v: Value) -> Result<()> {
        let base = self.global_ptrs[g];
        let addr = if off == 0 {
            base
        } else {
            self.builder
                .ins()
                .iadd_imm(base, off * 8)
        };
        self.builder
            .ins()
            .store(MemFlags::trusted(), v, addr, 0);
        Ok(())
    }

    fn load_global_idx(&mut self, g: usize, idx: Value) -> Result<Value> {
        let base = self.global_ptrs[g];
        let off = self.builder.ins().imul_imm(idx, 8);
        let addr = self.builder.ins().iadd(base, off);
        Ok(self
            .builder
            .ins()
            .load(types::I64, MemFlags::trusted(), addr, 0))
    }

    fn store_global_idx(&mut self, g: usize, idx: Value, v: Value) -> Result<()> {
        let base = self.global_ptrs[g];
        let off = self.builder.ins().imul_imm(idx, 8);
        let addr = self.builder.ins().iadd(base, off);
        self.builder
            .ins()
            .store(MemFlags::trusted(), v, addr, 0);
        Ok(())
    }

    fn binop<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut cranelift_frontend::FunctionBuilder, Value, Value) -> Value,
    {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = f(self.builder, a, b);
        self.push(r);
        Ok(())
    }

    fn binop_div(&mut self) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = self.builder.ins().sdiv(a, b);
        self.push(r);
        Ok(())
    }

    fn binop_mod(&mut self) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = self.builder.ins().srem(a, b);
        self.push(r);
        Ok(())
    }

    fn cmp(&mut self, cc: IntCC) -> Result<()> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = self.builder.ins().icmp(cc, a, b);
        let rv = self.builder.ins().uextend(types::I64, r);
        self.push(rv);
        Ok(())
    }

    fn emit_builtin(&mut self, id: u8) -> Result<()> {
        let b = Builtin::from_u8(id).ok_or_else(|| anyhow!("unknown builtin {id}"))?;
        match b {
            Builtin::PrintI64 => {
                let v = self.pop()?;
                self.builder.ins().call(self.rt.print_i, &[v]);
            }
            Builtin::PrintStr => {
                let ptr = self.pop()?;
                self.builder.ins().call(self.rt.print_str, &[ptr]);
            }
            Builtin::PutChar => {
                let v = self.pop()?;
                self.builder.ins().call(self.rt.putchar, &[v]);
            }
            Builtin::GetChar => {
                let call = self.builder.ins().call(self.rt.getchar, &[]);
                let rv = self.builder.inst_results(call)[0];
                self.push(rv);
            }
            Builtin::StrLen => {
                let ptr = self.pop()?;
                let call = self.builder.ins().call(self.rt.strlen, &[ptr]);
                let rv = self.builder.inst_results(call)[0];
                self.push(rv);
            }
            Builtin::CharAt => {
                let i = self.pop()?;
                let ptr = self.pop()?;
                let call = self.builder.ins().call(self.rt.char_at, &[ptr, i]);
                let rv = self.builder.inst_results(call)[0];
                self.push(rv);
            }
            Builtin::Argc => {
                let call = self.builder.ins().call(self.rt.argc, &[]);
                let rv = self.builder.inst_results(call)[0];
                self.push(rv);
            }
            Builtin::Argv => {
                let i = self.pop()?;
                let call = self.builder.ins().call(self.rt.argv, &[i]);
                let rv = self.builder.inst_results(call)[0];
                self.push(rv);
            }
        }
        Ok(())
    }
}

fn declare_runtime(module: &mut ObjectModule) -> Result<RuntimeFuncs> {
    let mut void_i64 = module.make_signature();
    void_i64.params.push(AbiParam::new(types::I64));

    let mut i64_void = module.make_signature();
    i64_void.returns.push(AbiParam::new(types::I64));

    let mut i64_i64_i64 = module.make_signature();
    i64_i64_i64.params.push(AbiParam::new(types::I64));
    i64_i64_i64.params.push(AbiParam::new(types::I64));
    i64_i64_i64.returns.push(AbiParam::new(types::I64));

    let mut i64_i64 = module.make_signature();
    i64_i64.params.push(AbiParam::new(types::I64));
    i64_i64.returns.push(AbiParam::new(types::I64));

    Ok(RuntimeFuncs {
        print_i: module.declare_function("glyph_print_i", Linkage::Import, &void_i64)?,
        print_str: module.declare_function("glyph_print_str", Linkage::Import, &void_i64)?,
        putchar: module.declare_function("glyph_putchar", Linkage::Import, &void_i64)?,
        getchar: module.declare_function("glyph_getchar", Linkage::Import, &i64_void)?,
        strlen: module.declare_function("glyph_strlen", Linkage::Import, &i64_i64)?,
        char_at: module.declare_function("glyph_char_at", Linkage::Import, &i64_i64_i64)?,
        argc: module.declare_function("glyph_argc", Linkage::Import, &i64_void)?,
        argv: module.declare_function("glyph_argv", Linkage::Import, &i64_i64)?,
    })
}

fn max_stack_depth(code: &[u8]) -> usize {
    let mut depth = 0usize;
    let mut max = 0usize;
    let mut dec = Decoder::new(code);
    while dec.remaining() > 0 {
        let op = dec.read_op().unwrap_or(Op::Nop);
        match op {
            Op::ConstI64 => {
                dec.read_i64().ok();
                depth += 1;
            }
            Op::ConstStr | Op::LoadLocal | Op::LoadGlobIdx => {
                dec.read_u16().ok();
                depth += 1;
            }
            Op::LoadGlob => {
                dec.read_u16().ok();
                dec.read_u32().ok();
                depth += 1;
            }
            Op::Dup => depth += 1,
            Op::Pop => depth = depth.saturating_sub(1),
            Op::StoreLocal | Op::StoreGlobIdx => {
                dec.read_u16().ok();
                depth = depth.saturating_sub(1);
            }
            Op::StoreGlob => {
                dec.read_u16().ok();
                dec.read_u32().ok();
                depth = depth.saturating_sub(1);
            }
            Op::Neg | Op::Not => {}
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod
            | Op::CmpLt | Op::CmpGt | Op::CmpLe | Op::CmpGe | Op::CmpEq | Op::CmpNe => {
                depth = depth.saturating_sub(1);
            }
            Op::Jmp => {
                dec.read_i32().ok();
            }
            Op::JmpIf | Op::JmpIfNot => {
                dec.read_i32().ok();
                depth = depth.saturating_sub(1);
            }
            Op::Call => {
                dec.read_u16().ok();
                let argc = dec.read_u8().unwrap_or(0) as usize;
                depth = depth.saturating_sub(argc);
                depth += 1;
            }
            Op::CallBuiltin => {
                let id = dec.read_u8().unwrap_or(0);
                dec.read_u8().ok();
                let pushes = matches!(
                    Builtin::from_u8(id),
                    Some(Builtin::GetChar | Builtin::StrLen | Builtin::CharAt | Builtin::Argc | Builtin::Argv)
                );
                if pushes {
                    depth += 1;
                } else {
                    depth = depth.saturating_sub(1);
                }
            }
            Op::Ret | Op::Halt => depth = depth.saturating_sub(1),
            _ => {}
        }
        max = max.max(depth);
    }
    max
}

