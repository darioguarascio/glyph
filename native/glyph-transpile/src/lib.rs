//! Transpile Glyph AST to other programming languages.

mod c;
mod link;

pub use c::{emit_c, COptions};
pub use link::{link_c_executable, runtime_dir};
pub use glyph_frontend::{ParseError, Program};

use glyph_frontend::Parser;

/// Target language for transpilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    C,
}

impl Target {
    pub fn extension(self) -> &'static str {
        match self {
            Self::C => "c",
        }
    }

    pub fn emit(self, prog: &Program, opts: &TranspileOptions) -> anyhow::Result<String> {
        match self {
            Self::C => emit_c(prog, &opts.c),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranspileOptions {
    pub c: COptions,
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            c: COptions::default(),
        }
    }
}

pub fn parse(source: &str) -> Result<Program, ParseError> {
    Parser::parse(source)
}

pub fn transpile_source(source: &str, target: Target, opts: &TranspileOptions) -> anyhow::Result<String> {
    let prog = parse(source)?;
    target.emit(&prog, opts)
}

pub fn transpile_to_executable(
    source: &str,
    out: &std::path::Path,
    opts: &TranspileOptions,
) -> anyhow::Result<()> {
    let code = transpile_source(source, Target::C, opts)?;
    link_c_executable(&code, out, &opts.c)
}
