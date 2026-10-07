use glyph_codegen::build_executable;
use glyph_frontend::compile;
use glyph_ir::Module;
use glyph_transpile::{emit_c, parse, runtime_dir, transpile_to_executable, COptions, TranspileOptions};
use glyph_vm::Vm;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

fn usage() -> ! {
    eprintln!("glyph — native Glyph toolchain (GBC bytecode)");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  glyph build <in.gl|in.gbc> -o <binary>   Cranelift → native executable");
    eprintln!("  glyph transpile <in.gl> -o <binary>      C transpile + gcc link");
    eprintln!("  glyph emit-c <in.gl> -o <out.c>          emit C source only");
    eprintln!("  glyph compile <in.gl> -o <out.gbc>       compile source to bytecode");
    eprintln!("  glyph run <file.gbc|file.gl>             execute in VM");
    eprintln!("  glyph emit-hello <out.gbc>               write sample hello module");
    process::exit(1);
}

fn read_module(path: &str) -> Module {
    if path.ends_with(".gl") {
        let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("glyph: cannot read {path}: {e}");
            process::exit(1);
        });
        return compile(&src).unwrap_or_else(|e| {
            eprintln!("glyph: {e}");
            process::exit(1);
        });
    }
    let mut f = File::open(path).unwrap_or_else(|e| {
        eprintln!("glyph: cannot open {path}: {e}");
        process::exit(1);
    });
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap_or_else(|e| {
        eprintln!("glyph: read error: {e}");
        process::exit(1);
    });
    Module::read(&mut io::Cursor::new(buf)).unwrap_or_else(|e| {
        eprintln!("glyph: {e}");
        process::exit(1);
    })
}

fn parse_o_flag(args: &mut impl Iterator<Item = String>) -> Option<String> {
    let rest: Vec<String> = args.collect();
    let mut out = None;
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == "-o" && i + 1 < rest.len() {
            out = Some(rest[i + 1].clone());
            i += 2;
        } else {
            eprintln!("glyph: unknown argument {}", rest[i]);
            process::exit(1);
        }
    }
    out
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else { usage() };
    match cmd.as_str() {
        "emit-c" => {
            let Some(in_path) = args.next() else { usage() };
            let out_path = parse_o_flag(&mut args).unwrap_or_else(|| usage());
            let src = std::fs::read_to_string(&in_path).unwrap_or_else(|e| {
                eprintln!("glyph: cannot read {in_path}: {e}");
                process::exit(1);
            });
            let prog = parse(&src).unwrap_or_else(|e| {
                eprintln!("glyph: {e}");
                process::exit(1);
            });
            let rt = runtime_dir();
            let opts = COptions {
                rt_header: rt.join("glyph_rt.h").to_string_lossy().into_owned(),
                net_header: rt.join("glyph_net.h").to_string_lossy().into_owned(),
            };
            let code = emit_c(&prog, &opts).unwrap_or_else(|e| {
                eprintln!("glyph: emit-c failed: {e}");
                process::exit(1);
            });
            let mut f = File::create(&out_path).unwrap_or_else(|e| {
                eprintln!("glyph: cannot create {out_path}: {e}");
                process::exit(1);
            });
            f.write_all(code.as_bytes()).unwrap_or_else(|e| {
                eprintln!("glyph: write error: {e}");
                process::exit(1);
            });
        }
        "transpile" => {
            let Some(in_path) = args.next() else { usage() };
            let out_path = parse_o_flag(&mut args).unwrap_or_else(|| usage());
            let src = std::fs::read_to_string(&in_path).unwrap_or_else(|e| {
                eprintln!("glyph: cannot read {in_path}: {e}");
                process::exit(1);
            });
            transpile_to_executable(&src, PathBuf::from(&out_path).as_path(), &TranspileOptions::default())
                .unwrap_or_else(|e| {
                    eprintln!("glyph: transpile failed: {e}");
                    process::exit(1);
                });
        }
        "build" => {
            let Some(in_path) = args.next() else { usage() };
            let out_path = parse_o_flag(&mut args).unwrap_or_else(|| usage());
            let module = read_module(&in_path);
            build_executable(&module, PathBuf::from(&out_path).as_path()).unwrap_or_else(|e| {
                eprintln!("glyph: build failed: {e}");
                process::exit(1);
            });
        }
        "compile" => {
            let Some(in_path) = args.next() else { usage() };
            let out_path = parse_o_flag(&mut args).unwrap_or_else(|| usage());
            let src = std::fs::read_to_string(&in_path).unwrap_or_else(|e| {
                eprintln!("glyph: cannot read {in_path}: {e}");
                process::exit(1);
            });
            let module = compile(&src).unwrap_or_else(|e| {
                eprintln!("glyph: {e}");
                process::exit(1);
            });
            let mut f = File::create(&out_path).unwrap_or_else(|e| {
                eprintln!("glyph: cannot create {out_path}: {e}");
                process::exit(1);
            });
            module.write(&mut f).unwrap_or_else(|e| {
                eprintln!("glyph: {e}");
                process::exit(1);
            });
        }
        "run" => {
            let Some(path) = args.next() else { usage() };
            let module = read_module(&path);
            let mut vm = Vm::new(module).unwrap_or_else(|e| {
                eprintln!("glyph: {e}");
                process::exit(1);
            });
            if let Err(e) = vm.run() {
                eprintln!("glyph: runtime error: {e}");
                process::exit(1);
            }
        }
        "emit-hello" => {
            let Some(path) = args.next() else { usage() };
            let m = Module::hello();
            let mut f = File::create(&path).unwrap_or_else(|e| {
                eprintln!("glyph: cannot create {path}: {e}");
                process::exit(1);
            });
            m.write(&mut f).unwrap_or_else(|e| {
                eprintln!("glyph: {e}");
                process::exit(1);
            });
        }
        _ => usage(),
    }
}
