use glyph_transpile::{transpile_to_executable, TranspileOptions};
use std::path::PathBuf;
use std::process::Command;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn build_and_run(gl: &str, args: &[&str]) -> String {
    let src = std::fs::read_to_string(example(gl)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("bin");
    transpile_to_executable(&src, &out, &TranspileOptions::default()).unwrap();
    let output = Command::new(&out).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn transpile_hello() {
    assert!(build_and_run("hello.gl", &[]).contains("Hello, World!"));
}

#[test]
fn transpile_fib() {
    assert!(build_and_run("fib.gl", &[]).contains("55"));
}

#[test]
fn transpile_fizzbuzz() {
    let out = build_and_run("fizzbuzz.gl", &[]);
    assert!(out.contains("FizzBuzz"));
    assert!(out.contains("Buzz"));
}
