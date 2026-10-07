use glyph_codegen::build_executable;
use glyph_frontend::compile;
use std::path::PathBuf;
use std::process::Command;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn build_and_run(gl: &str, args: &[&str]) -> String {
    let src = std::fs::read_to_string(example(gl)).unwrap();
    let module = compile(&src).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("bin");
    build_executable(&module, &out).unwrap();
    let output = Command::new(&out).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn native_hello() {
    assert!(build_and_run("hello.gl", &[]).contains("Hello, World!"));
}

#[test]
fn native_fib() {
    assert!(build_and_run("fib.gl", &[]).contains("55"));
}

#[test]
fn native_fizzbuzz() {
    let out = build_and_run("fizzbuzz.gl", &[]);
    assert!(out.contains("FizzBuzz"));
    assert!(out.contains("Buzz"));
}
