mod ast;
mod lex;
mod lower;
mod parse;

pub use ast::*;
pub use lower::{LowerError, Lowerer};
pub use parse::{ParseError, Parser};

#[derive(Debug)]
pub enum CompileError {
    Parse(ParseError),
    Lower(LowerError),
    Ir(glyph_ir::IrError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "parse: {e}"),
            Self::Lower(e) => write!(f, "lower: {e}"),
            Self::Ir(e) => write!(f, "ir: {e}"),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<ParseError> for CompileError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

impl From<LowerError> for CompileError {
    fn from(e: LowerError) -> Self {
        Self::Lower(e)
    }
}

impl From<glyph_ir::IrError> for CompileError {
    fn from(e: glyph_ir::IrError) -> Self {
        Self::Ir(e)
    }
}

/// Compile Glyph source text to a verified GBC module.
pub fn compile(source: &str) -> Result<glyph_ir::Module, CompileError> {
    let prog = Parser::parse(source)?;
    let module = Lowerer::lower(prog)?;
    module.verify()?;
    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glyph_vm::Vm;
    use std::path::PathBuf;

    fn run_example(name: &str) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(name);
        let src = std::fs::read_to_string(&path).unwrap();
        let module = compile(&src).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut vm = Vm::new(module).unwrap_or_else(|e| panic!("{name} vm: {e}"));
        vm.run().unwrap_or_else(|e| panic!("{name} run: {e}"));
    }

    #[test]
    fn hello_gl() {
        run_example("hello.gl");
    }

    #[test]
    fn fib_gl() {
        run_example("fib.gl");
    }

    #[test]
    fn fizzbuzz_gl() {
        run_example("fizzbuzz.gl");
    }
}
