//! Reference interpreter for GBC bytecode.

use glyph_ir::{Builtin, Decoder, IrError, Module, Op};
use std::env;
use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum VmError {
    Ir(IrError),
    StackUnderflow,
    DivByZero,
    BadStrIdx(u16),
    BadFnIdx(u16),
    BadGlobIdx(u16),
    BadLocal(u16),
    CallDepth,
    NoMain,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ir(e) => write!(f, "{e}"),
            Self::StackUnderflow => write!(f, "stack underflow"),
            Self::DivByZero => write!(f, "division by zero"),
            Self::BadStrIdx(i) => write!(f, "bad string index {i}"),
            Self::BadFnIdx(i) => write!(f, "bad function index {i}"),
            Self::BadGlobIdx(i) => write!(f, "bad global index {i}"),
            Self::BadLocal(i) => write!(f, "bad local slot {i}"),
            Self::CallDepth => write!(f, "call stack overflow"),
            Self::NoMain => write!(f, "module has no functions"),
        }
    }
}

impl std::error::Error for VmError {}

impl From<IrError> for VmError {
    fn from(e: IrError) -> Self {
        Self::Ir(e)
    }
}

use std::fmt;

struct Frame {
    fn_idx: u16,
    slots: Vec<i64>,
    ret_ip: usize,
    ret_fn: u16,
}

pub struct Vm {
    module: Module,
    globals: Vec<Vec<i64>>,
    stack: Vec<i64>,
    frames: Vec<Frame>,
    argv: Vec<String>,
}

impl Vm {
    pub fn new(module: Module) -> Result<Self, VmError> {
        module.verify().map_err(VmError::Ir)?;
        let mut globals = Vec::with_capacity(module.globals.len());
        for g in &module.globals {
            let cells = match g.kind {
                glyph_ir::GlobalKind::I64Array => g.size as usize,
                glyph_ir::GlobalKind::ByteArray => g.size as usize,
            };
            globals.push(vec![0i64; cells]);
        }
        Ok(Self {
            module,
            globals,
            stack: Vec::new(),
            frames: Vec::new(),
            argv: env::args().collect(),
        })
    }

    pub fn run(&mut self) -> Result<i64, VmError> {
        if self.module.functions.is_empty() {
            return Err(VmError::NoMain);
        }
        self.call_fn(0, 0)?;
        self.exec_current()
    }

    fn exec_current(&mut self) -> Result<i64, VmError> {
        loop {
            let frame = self.frames.last_mut().ok_or(VmError::StackUnderflow)?;
            let fn_idx = frame.fn_idx as usize;
            let code = self.module.functions[fn_idx].code.clone();
            let mut dec = Decoder::new(&code);
            dec.ip = frame.ret_ip;

            while dec.remaining() > 0 {
                let op_ip = dec.ip;
                let op = dec.read_op()?;
                match op {
                    Op::Nop => {}
                    Op::Halt => return Ok(0),
                    Op::ConstI64 => {
                        let v = dec.read_i64()?;
                        self.stack.push(v);
                    }
                    Op::ConstStr => {
                        let idx = dec.read_u16()?;
                        if idx as usize >= self.module.strings.len() {
                            return Err(VmError::BadStrIdx(idx));
                        }
                        self.stack.push(idx as i64);
                    }
                    Op::Pop => {
                        self.pop()?;
                    }
                    Op::Dup => {
                        let v = *self.stack.last().ok_or(VmError::StackUnderflow)?;
                        self.stack.push(v);
                    }
                    Op::LoadLocal => {
                        let slot = dec.read_u16()?;
                        let frame = self.frames.last().ok_or(VmError::StackUnderflow)?;
                        let v = *frame
                            .slots
                            .get(slot as usize)
                            .ok_or(VmError::BadLocal(slot))?;
                        self.stack.push(v);
                    }
                    Op::StoreLocal => {
                        let slot = dec.read_u16()?;
                        let v = self.pop()?;
                        let frame = self.frames.last_mut().ok_or(VmError::StackUnderflow)?;
                        if slot as usize >= frame.slots.len() {
                            return Err(VmError::BadLocal(slot));
                        }
                        frame.slots[slot as usize] = v;
                    }
                    Op::LoadGlob => {
                        let g = dec.read_u16()?;
                        let off = dec.read_u32()? as usize;
                        let v = self
                            .globals
                            .get(g as usize)
                            .and_then(|seg| seg.get(off))
                            .copied()
                            .ok_or(VmError::BadGlobIdx(g))?;
                        self.stack.push(v);
                    }
                    Op::StoreGlob => {
                        let g = dec.read_u16()?;
                        let off = dec.read_u32()? as usize;
                        let v = self.pop()?;
                        let seg = self
                            .globals
                            .get_mut(g as usize)
                            .ok_or(VmError::BadGlobIdx(g))?;
                        if off >= seg.len() {
                            return Err(VmError::BadGlobIdx(g));
                        }
                        seg[off] = v;
                    }
                    Op::LoadGlobIdx => {
                        let g = dec.read_u16()?;
                        let off = self.pop()? as usize;
                        let v = self
                            .globals
                            .get(g as usize)
                            .and_then(|seg| seg.get(off))
                            .copied()
                            .ok_or(VmError::BadGlobIdx(g))?;
                        self.stack.push(v);
                    }
                    Op::StoreGlobIdx => {
                        let g = dec.read_u16()?;
                        let off = self.pop()? as usize;
                        let v = self.pop()?;
                        let seg = self
                            .globals
                            .get_mut(g as usize)
                            .ok_or(VmError::BadGlobIdx(g))?;
                        if off >= seg.len() {
                            return Err(VmError::BadGlobIdx(g));
                        }
                        seg[off] = v;
                    }
                    Op::Add => binop(&mut self.stack, |a, b| a.wrapping_add(b))?,
                    Op::Sub => binop(&mut self.stack, |a, b| a.wrapping_sub(b))?,
                    Op::Mul => binop(&mut self.stack, |a, b| a.wrapping_mul(b))?,
                    Op::Div => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        if b == 0 {
                            return Err(VmError::DivByZero);
                        }
                        self.stack.push(a / b);
                    }
                    Op::Mod => {
                        let b = self.pop()?;
                        let a = self.pop()?;
                        if b == 0 {
                            return Err(VmError::DivByZero);
                        }
                        self.stack.push(a % b);
                    }
                    Op::Neg => {
                        let a = self.pop()?;
                        self.stack.push(-a);
                    }
                    Op::Not => {
                        let a = self.pop()?;
                        self.stack.push(if a == 0 { 1 } else { 0 });
                    }
                    Op::CmpLt => cmp(&mut self.stack, |a, b| a < b)?,
                    Op::CmpGt => cmp(&mut self.stack, |a, b| a > b)?,
                    Op::CmpLe => cmp(&mut self.stack, |a, b| a <= b)?,
                    Op::CmpGe => cmp(&mut self.stack, |a, b| a >= b)?,
                    Op::CmpEq => cmp(&mut self.stack, |a, b| a == b)?,
                    Op::CmpNe => cmp(&mut self.stack, |a, b| a != b)?,
                    Op::Jmp => {
                        let rel = dec.read_i32()?;
                        dec.jump_rel(rel)?;
                    }
                    Op::JmpIf => {
                        let rel = dec.read_i32()?;
                        let cond = self.pop()?;
                        if cond != 0 {
                            dec.jump_rel(rel)?;
                        }
                    }
                    Op::JmpIfNot => {
                        let rel = dec.read_i32()?;
                        let cond = self.pop()?;
                        if cond == 0 {
                            dec.jump_rel(rel)?;
                        }
                    }
                    Op::Call => {
                        let fn_idx = dec.read_u16()?;
                        let argc = dec.read_u8()?;
                        let mut args = Vec::with_capacity(argc as usize);
                        for _ in 0..argc {
                            args.push(self.pop()?);
                        }
                        args.reverse();
                        // save return point
                        {
                            let frame = self.frames.last_mut().ok_or(VmError::StackUnderflow)?;
                            frame.ret_ip = dec.ip;
                        }
                        self.call_fn(fn_idx, argc)?;
                        for (i, arg) in args.into_iter().enumerate() {
                            self.frames.last_mut().unwrap().slots[i] = arg;
                        }
                        break; // enter callee
                    }
                    Op::Ret => {
                        let ret = self.pop().unwrap_or(0);
                        let frame = self.frames.pop().ok_or(VmError::StackUnderflow)?;
                        if self.frames.is_empty() {
                            return Ok(ret);
                        }
                        self.stack.push(ret);
                        let parent = self.frames.last_mut().unwrap();
                        parent.ret_ip = frame.ret_ip;
                        break;
                    }
                    Op::CallBuiltin => {
                        let id = dec.read_u8()?;
                        let _argc = dec.read_u8()?;
                        self.call_builtin(id)?;
                    }
                    Op::Syscall => {
                        let id = dec.read_u8()?;
                        let _argc = dec.read_u8()?;
                        return Err(VmError::Ir(IrError::Verify(format!(
                            "syscall {id} not implemented in reference VM"
                        ))));
                    }
                }
                let frame = self.frames.last_mut().ok_or(VmError::StackUnderflow)?;
                frame.ret_ip = dec.ip;
                let _ = op_ip; // silence unused in some builds
            }
        }
    }

    fn call_fn(&mut self, fn_idx: u16, _argc: u8) -> Result<(), VmError> {
        if fn_idx as usize >= self.module.functions.len() {
            return Err(VmError::BadFnIdx(fn_idx));
        }
        if self.frames.len() >= 256 {
            return Err(VmError::CallDepth);
        }
        let f = &self.module.functions[fn_idx as usize];
        self.frames.push(Frame {
            fn_idx,
            slots: vec![0; f.slot_count as usize],
            ret_ip: 0,
            ret_fn: 0,
        });
        Ok(())
    }

    fn call_builtin(&mut self, id: u8) -> Result<(), VmError> {
        let b = Builtin::from_u8(id).ok_or(IrError::UnknownBuiltin(id))?;
        match b {
            Builtin::PrintI64 => {
                let v = self.pop()?;
                println!("{v}");
            }
            Builtin::PrintStr => {
                let idx = self.pop()? as u16;
                let s = self
                    .module
                    .strings
                    .get(idx as usize)
                    .ok_or(VmError::BadStrIdx(idx))?;
                println!("{s}");
            }
            Builtin::PutChar => {
                let c = self.pop()? as u8;
                print!("{}", c as char);
                io::stdout().flush().ok();
            }
            Builtin::GetChar => {
                let mut b = [0u8; 1];
                let n = io::stdin().read(&mut b).unwrap_or(0);
                self.stack.push(if n == 0 { -1 } else { b[0] as i64 });
            }
            Builtin::StrLen => {
                let idx = self.pop()? as u16;
                let s = self
                    .module
                    .strings
                    .get(idx as usize)
                    .ok_or(VmError::BadStrIdx(idx))?;
                self.stack.push(s.len() as i64);
            }
            Builtin::CharAt => {
                let i = self.pop()?;
                let idx = self.pop()? as u16;
                let s = self
                    .module
                    .strings
                    .get(idx as usize)
                    .ok_or(VmError::BadStrIdx(idx))?;
                let ch = s.as_bytes().get(i as usize).copied().unwrap_or(0);
                self.stack.push(ch as i64);
            }
            Builtin::Argc => {
                self.stack.push(self.argv.len() as i64);
            }
            Builtin::Argv => {
                let i = self.pop()? as usize;
                let s = self.argv.get(i).cloned().unwrap_or_default();
                // intern argv strings into module pool for this session
                let idx = self.module.strings.len();
                self.module.strings.push(s);
                self.stack.push(idx as i64);
            }
        }
        Ok(())
    }

    fn pop(&mut self) -> Result<i64, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }
}

fn binop(stack: &mut Vec<i64>, f: fn(i64, i64) -> i64) -> Result<(), VmError> {
    let b = stack.pop().ok_or(VmError::StackUnderflow)?;
    let a = stack.pop().ok_or(VmError::StackUnderflow)?;
    stack.push(f(a, b));
    Ok(())
}

fn cmp(stack: &mut Vec<i64>, f: fn(i64, i64) -> bool) -> Result<(), VmError> {
    let b = stack.pop().ok_or(VmError::StackUnderflow)?;
    let a = stack.pop().ok_or(VmError::StackUnderflow)?;
    stack.push(if f(a, b) { 1 } else { 0 });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glyph_ir::Module;

    #[test]
    fn hello_runs() {
        let mut vm = Vm::new(Module::hello()).unwrap();
        assert_eq!(vm.run().unwrap(), 0);
    }
}
