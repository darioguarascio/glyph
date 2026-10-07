//! GBC — Glyph Bytecode IR (canonical program representation).
//!
//! See `spec/BYTECODE.md` for the wire format and opcode semantics.

use std::fmt;
use std::io::{self, Read, Write};

pub const MAGIC: &[u8; 4] = b"GBC1";
pub const VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GlobalKind {
    I64Array = 0,
    ByteArray = 1,
}

impl GlobalKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::I64Array),
            1 => Some(Self::ByteArray),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSeg {
    pub kind: GlobalKind,
    pub name_idx: u16,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name_idx: u16,
    pub arity: u8,
    pub slot_count: u16,
    pub code: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub flags: u8,
    pub strings: Vec<String>,
    pub globals: Vec<GlobalSeg>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Op {
    Nop = 0x00,
    Halt = 0x01,
    ConstI64 = 0x02,
    ConstStr = 0x03,
    Pop = 0x04,
    Dup = 0x05,
    LoadLocal = 0x10,
    StoreLocal = 0x11,
    LoadGlob = 0x12,
    StoreGlob = 0x13,
    LoadGlobIdx = 0x14,
    StoreGlobIdx = 0x15,
    Add = 0x20,
    Sub = 0x21,
    Mul = 0x22,
    Div = 0x23,
    Mod = 0x24,
    Neg = 0x25,
    Not = 0x26,
    CmpLt = 0x30,
    CmpGt = 0x31,
    CmpLe = 0x32,
    CmpGe = 0x33,
    CmpEq = 0x34,
    CmpNe = 0x35,
    Jmp = 0x40,
    JmpIf = 0x41,
    JmpIfNot = 0x42,
    Call = 0x50,
    Ret = 0x51,
    CallBuiltin = 0x52,
    Syscall = 0x53,
}

impl Op {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::Nop),
            0x01 => Some(Self::Halt),
            0x02 => Some(Self::ConstI64),
            0x03 => Some(Self::ConstStr),
            0x04 => Some(Self::Pop),
            0x05 => Some(Self::Dup),
            0x10 => Some(Self::LoadLocal),
            0x11 => Some(Self::StoreLocal),
            0x12 => Some(Self::LoadGlob),
            0x13 => Some(Self::StoreGlob),
            0x14 => Some(Self::LoadGlobIdx),
            0x15 => Some(Self::StoreGlobIdx),
            0x20 => Some(Self::Add),
            0x21 => Some(Self::Sub),
            0x22 => Some(Self::Mul),
            0x23 => Some(Self::Div),
            0x24 => Some(Self::Mod),
            0x25 => Some(Self::Neg),
            0x26 => Some(Self::Not),
            0x30 => Some(Self::CmpLt),
            0x31 => Some(Self::CmpGt),
            0x32 => Some(Self::CmpLe),
            0x33 => Some(Self::CmpGe),
            0x34 => Some(Self::CmpEq),
            0x35 => Some(Self::CmpNe),
            0x40 => Some(Self::Jmp),
            0x41 => Some(Self::JmpIf),
            0x42 => Some(Self::JmpIfNot),
            0x50 => Some(Self::Call),
            0x51 => Some(Self::Ret),
            0x52 => Some(Self::CallBuiltin),
            0x53 => Some(Self::Syscall),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Builtin {
    PrintI64 = 0,
    PrintStr = 1,
    PutChar = 2,
    GetChar = 3,
    StrLen = 4,
    CharAt = 5,
    Argc = 6,
    Argv = 7,
}

impl Builtin {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::PrintI64),
            1 => Some(Self::PrintStr),
            2 => Some(Self::PutChar),
            3 => Some(Self::GetChar),
            4 => Some(Self::StrLen),
            5 => Some(Self::CharAt),
            6 => Some(Self::Argc),
            7 => Some(Self::Argv),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum IrError {
    Io(io::Error),
    BadMagic,
    UnsupportedVersion(u8),
    Truncated,
    UnknownOpcode(u8),
    UnknownGlobalKind(u8),
    UnknownBuiltin(u8),
    Verify(String),
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::BadMagic => write!(f, "bad magic (expected GBC1)"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            Self::Truncated => write!(f, "truncated input"),
            Self::UnknownOpcode(o) => write!(f, "unknown opcode 0x{o:02x}"),
            Self::UnknownGlobalKind(k) => write!(f, "unknown global kind {k}"),
            Self::UnknownBuiltin(b) => write!(f, "unknown builtin {b}"),
            Self::Verify(msg) => write!(f, "verify: {msg}"),
        }
    }
}

impl std::error::Error for IrError {}

impl From<io::Error> for IrError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

fn read_u8(r: &mut impl Read) -> Result<u8, IrError> {
    let mut b = [0u8; 1];
    match r.read_exact(&mut b) {
        Ok(()) => Ok(b[0]),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(IrError::Truncated),
        Err(e) => Err(IrError::Io(e)),
    }
}

fn read_u16(r: &mut impl Read) -> Result<u16, IrError> {
    let mut b = [0u8; 2];
    match r.read_exact(&mut b) {
        Ok(()) => Ok(u16::from_le_bytes(b)),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(IrError::Truncated),
        Err(e) => Err(IrError::Io(e)),
    }
}

fn read_u32(r: &mut impl Read) -> Result<u32, IrError> {
    let mut b = [0u8; 4];
    match r.read_exact(&mut b) {
        Ok(()) => Ok(u32::from_le_bytes(b)),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(IrError::Truncated),
        Err(e) => Err(IrError::Io(e)),
    }
}

fn read_i64(r: &mut impl Read) -> Result<i64, IrError> {
    let mut b = [0u8; 8];
    match r.read_exact(&mut b) {
        Ok(()) => Ok(i64::from_le_bytes(b)),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(IrError::Truncated),
        Err(e) => Err(IrError::Io(e)),
    }
}

fn read_i32(r: &mut impl Read) -> Result<i32, IrError> {
    let mut b = [0u8; 4];
    match r.read_exact(&mut b) {
        Ok(()) => Ok(i32::from_le_bytes(b)),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Err(IrError::Truncated),
        Err(e) => Err(IrError::Io(e)),
    }
}

fn write_u8(w: &mut impl Write, v: u8) -> Result<(), IrError> {
    w.write_all(&[v]).map_err(IrError::Io)
}

fn write_u16(w: &mut impl Write, v: u16) -> Result<(), IrError> {
    w.write_all(&v.to_le_bytes()).map_err(IrError::Io)
}

fn write_u32(w: &mut impl Write, v: u32) -> Result<(), IrError> {
    w.write_all(&v.to_le_bytes()).map_err(IrError::Io)
}

impl Module {
    pub fn read(r: &mut impl Read) -> Result<Self, IrError> {
        let mut magic = [0u8; 4];
        match r.read_exact(&mut magic) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(IrError::Truncated),
            Err(e) => return Err(IrError::Io(e)),
        }
        if &magic != MAGIC {
            return Err(IrError::BadMagic);
        }
        let version = read_u8(r)?;
        if version != VERSION {
            return Err(IrError::UnsupportedVersion(version));
        }
        let flags = read_u8(r)?;
        let str_count = read_u16(r)?;
        let glob_count = read_u16(r)?;
        let fn_count = read_u16(r)?;

        let mut strings = Vec::with_capacity(str_count as usize);
        for _ in 0..str_count {
            let len = read_u16(r)? as usize;
            let mut buf = vec![0u8; len];
            match r.read_exact(&mut buf) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(IrError::Truncated),
                Err(e) => return Err(IrError::Io(e)),
            }
            let s = String::from_utf8_lossy(&buf).into_owned();
            strings.push(s);
        }

        let mut globals = Vec::with_capacity(glob_count as usize);
        for _ in 0..glob_count {
            let kind_byte = read_u8(r)?;
            let kind = GlobalKind::from_u8(kind_byte)
                .ok_or(IrError::UnknownGlobalKind(kind_byte))?;
            let name_idx = read_u16(r)?;
            let size = read_u32(r)?;
            globals.push(GlobalSeg { kind, name_idx, size });
        }

        let mut functions = Vec::with_capacity(fn_count as usize);
        for _ in 0..fn_count {
            let name_idx = read_u16(r)?;
            let arity = read_u8(r)?;
            let slot_count = read_u16(r)?;
            let code_len = read_u32(r)? as usize;
            let mut code = vec![0u8; code_len];
            match r.read_exact(&mut code) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(IrError::Truncated),
                Err(e) => return Err(IrError::Io(e)),
            }
            functions.push(Function {
                name_idx,
                arity,
                slot_count,
                code,
            });
        }

        let module = Module {
            flags,
            strings,
            globals,
            functions,
        };
        module.verify()?;
        Ok(module)
    }

    pub fn write(&self, w: &mut impl Write) -> Result<(), IrError> {
        self.verify()?;
        w.write_all(MAGIC).map_err(IrError::Io)?;
        write_u8(w, VERSION)?;
        write_u8(w, self.flags)?;
        write_u16(w, self.strings.len() as u16)?;
        write_u16(w, self.globals.len() as u16)?;
        write_u16(w, self.functions.len() as u16)?;

        for s in &self.strings {
            let bytes = s.as_bytes();
            if bytes.len() > u16::MAX as usize {
                return Err(IrError::Verify("string too long".into()));
            }
            write_u16(w, bytes.len() as u16)?;
            w.write_all(bytes).map_err(IrError::Io)?;
        }

        for g in &self.globals {
            write_u8(w, g.kind as u8)?;
            write_u16(w, g.name_idx)?;
            write_u32(w, g.size)?;
        }

        for f in &self.functions {
            write_u16(w, f.name_idx)?;
            write_u8(w, f.arity)?;
            write_u16(w, f.slot_count)?;
            write_u32(w, f.code.len() as u32)?;
            w.write_all(&f.code).map_err(IrError::Io)?;
        }
        Ok(())
    }

    /// Index of `main` if present, otherwise 0.
    pub fn entry_fn(&self) -> usize {
        self.functions
            .iter()
            .position(|f| {
                self.strings
                    .get(f.name_idx as usize)
                    .map(|s| s == "main")
                    .unwrap_or(false)
            })
            .unwrap_or(0)
    }

    pub fn verify(&self) -> Result<(), IrError> {
        for g in &self.globals {
            if g.name_idx as usize >= self.strings.len() {
                return Err(IrError::Verify(format!(
                    "global name_idx {} out of range",
                    g.name_idx
                )));
            }
        }
        for f in &self.functions {
            if f.name_idx as usize >= self.strings.len() {
                return Err(IrError::Verify(format!(
                    "function name_idx {} out of range",
                    f.name_idx
                )));
            }
            verify_code(&f.code)?;
        }
        Ok(())
    }

    /// Minimal hello-world module: print one string and halt.
    pub fn hello() -> Self {
        Module {
            flags: 0,
            strings: vec!["Hello, World!".into()],
            globals: vec![],
            functions: vec![Function {
                name_idx: 0,
                arity: 0,
                slot_count: 0,
                code: vec![
                    Op::ConstStr as u8,
                    0,
                    0, // str idx 0
                    Op::CallBuiltin as u8,
                    Builtin::PrintStr as u8,
                    1,
                    Op::Halt as u8,
                ],
            }],
        }
    }
}

pub fn verify_code(code: &[u8]) -> Result<(), IrError> {
    let mut ip = 0usize;
    while ip < code.len() {
        let op = code[ip];
        ip += 1;
        let Some(op) = Op::from_u8(op) else {
            return Err(IrError::UnknownOpcode(op));
        };
        ip = ip.checked_add(operand_size(op)).ok_or(IrError::Truncated)?;
        if ip > code.len() {
            return Err(IrError::Truncated);
        }
    }
    Ok(())
}

fn operand_size(op: Op) -> usize {
    match op {
        Op::ConstI64 => 8,
        Op::ConstStr | Op::LoadLocal | Op::StoreLocal => 2,
        Op::LoadGlob | Op::StoreGlob => 6,
        Op::LoadGlobIdx | Op::StoreGlobIdx => 2,
        Op::Jmp | Op::JmpIf | Op::JmpIfNot => 4,
        Op::Call => 3,
        Op::CallBuiltin | Op::Syscall => 2,
        _ => 0,
    }
}

/// Decode helpers for the VM.
pub struct Decoder<'a> {
    pub code: &'a [u8],
    pub ip: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(code: &'a [u8]) -> Self {
        Self { code, ip: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.code.len().saturating_sub(self.ip)
    }

    pub fn read_op(&mut self) -> Result<Op, IrError> {
        if self.ip >= self.code.len() {
            return Err(IrError::Truncated);
        }
        let b = self.code[self.ip];
        self.ip += 1;
        Op::from_u8(b).ok_or(IrError::UnknownOpcode(b))
    }

    pub fn read_u8(&mut self) -> Result<u8, IrError> {
        if self.ip >= self.code.len() {
            return Err(IrError::Truncated);
        }
        let v = self.code[self.ip];
        self.ip += 1;
        Ok(v)
    }

    pub fn read_u16(&mut self) -> Result<u16, IrError> {
        if self.ip + 2 > self.code.len() {
            return Err(IrError::Truncated);
        }
        let v = u16::from_le_bytes([self.code[self.ip], self.code[self.ip + 1]]);
        self.ip += 2;
        Ok(v)
    }

    pub fn read_u32(&mut self) -> Result<u32, IrError> {
        if self.ip + 4 > self.code.len() {
            return Err(IrError::Truncated);
        }
        let v = u32::from_le_bytes([
            self.code[self.ip],
            self.code[self.ip + 1],
            self.code[self.ip + 2],
            self.code[self.ip + 3],
        ]);
        self.ip += 4;
        Ok(v)
    }

    pub fn read_i32(&mut self) -> Result<i32, IrError> {
        Ok(self.read_u32()? as i32)
    }

    pub fn read_i64(&mut self) -> Result<i64, IrError> {
        if self.ip + 8 > self.code.len() {
            return Err(IrError::Truncated);
        }
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.code[self.ip..self.ip + 8]);
        self.ip += 8;
        Ok(i64::from_le_bytes(b))
    }

    pub fn jump_rel(&mut self, rel: i32) -> Result<(), IrError> {
        let base = self.ip as i64;
        let next = base + rel as i64;
        if next < 0 || next > self.code.len() as i64 {
            return Err(IrError::Verify(format!("jump out of bounds: {rel}")));
        }
        self.ip = next as usize;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn hello_roundtrip() {
        let m = Module::hello();
        let mut buf = Vec::new();
        m.write(&mut buf).unwrap();
        let m2 = Module::read(&mut Cursor::new(buf)).unwrap();
        assert_eq!(m2.strings[0], "Hello, World!");
    }
}
