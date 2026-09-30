use glyph_frontend::compile;
use glyph_ir::Module;
use glyph_vm::Vm;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process;

fn usage() -> ! {
    eprintln!("glyph — native Glyph toolchain (GBC bytecode)");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  glyph compile <in.gl> -o <out.gbc>   compile source to bytecode");
    eprintln!("  glyph run <file.gbc|file.gl>         execute bytecode or source");
    eprintln!("  glyph emit-hello <out.gbc>           write sample hello module");
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

fn main() {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else { usage() };
    match cmd.as_str() {
        "compile" => {
            let Some(in_path) = args.next() else { usage() };
            let mut out_path = None;
            let mut rest: Vec<String> = args.collect();
            while let Some(a) = rest.first().cloned() {
                if a == "-o" && rest.len() >= 2 {
                    out_path = Some(rest[1].clone());
                    rest.drain(0..2);
                } else {
                    eprintln!("glyph: unknown argument {a}");
                    process::exit(1);
                }
            }
            let Some(out_path) = out_path else { usage() };
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
