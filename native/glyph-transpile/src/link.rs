use crate::COptions;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn runtime_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime")
}

pub fn link_c_executable(code: &str, out: &Path, opts: &COptions) -> Result<()> {
    let rt = runtime_dir();
    let rt_header = rt.join("glyph_rt.h");
    let net_header = rt.join("glyph_net.h");
    let net_c = rt.join("glyph_net.c");

    let mut c_code = code.replace(&opts.rt_header, rt_header.to_string_lossy().as_ref());
    c_code = c_code.replace(&opts.net_header, net_header.to_string_lossy().as_ref());

    let dir = tempfile::tempdir().context("tempdir")?;
    let c_path = dir.path().join("out.c");
    std::fs::write(&c_path, c_code).context("write C source")?;

    let status = Command::new("cc")
        .arg("-O2")
        .arg("-std=c11")
        .arg("-o")
        .arg(out)
        .arg(&c_path)
        .arg(&net_c)
        .status()
        .context("invoke cc")?;
    if !status.success() {
        bail!("gcc link failed (exit {status})");
    }
    Ok(())
}
