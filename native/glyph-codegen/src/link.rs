use anyhow::{bail, Context, Result};
use glyph_ir::Module as GbcModule;
use std::path::Path;
use std::process::Command;

use crate::{compile_module, NativeObject};

pub fn link_executable(obj: &NativeObject, out: &Path) -> Result<()> {
    let dir = tempfile::tempdir().context("tempdir")?;
    let obj_path = dir.path().join("module.o");
    std::fs::write(&obj_path, &obj.bytes).context("write object")?;
    let rt = Path::new(env!("CARGO_MANIFEST_DIR")).join("../glyph-rt/glyph_rt.c");
    let status = Command::new("cc")
        .arg("-O2")
        .arg("-o")
        .arg(out)
        .arg(&obj_path)
        .arg(&rt)
        .status()
        .context("invoke cc")?;
    if !status.success() {
        bail!("link failed (cc exit {status})");
    }
    Ok(())
}

pub fn build_executable(gbc: &GbcModule, out: &Path) -> Result<()> {
    let obj = compile_module(gbc)?;
    link_executable(&obj, out)
}
