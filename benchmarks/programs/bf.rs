use std::env;
use std::io::{self, Read, Write};

fn sf(code: &[u8], mut i: i64) -> i64 {
    let mut d = 1i64;
    while d > 0 {
        i += 1;
        let x = code[i as usize];
        if x == 91 { d += 1; }
        if x == 93 { d -= 1; }
    }
    i
}

fn sb(code: &[u8], mut i: i64) -> i64 {
    let mut d = 1i64;
    while d > 0 {
        i -= 1;
        let x = code[i as usize];
        if x == 93 { d += 1; }
        if x == 91 { d -= 1; }
    }
    i
}

fn run(code: &[u8]) {
    let mut mem = [0i64; 30000];
    let mut dp = 0i64;
    let mut i = 0i64;
    while i < code.len() as i64 {
        let ch = code[i as usize];
        match ch {
            62 => dp += 1,
            60 => dp -= 1,
            43 => mem[dp as usize] += 1,
            45 => mem[dp as usize] -= 1,
            46 => { write!(io::stdout(), "{}", mem[dp as usize] as u8 as char).unwrap(); },
            44 => { let mut b = [0u8; 1]; let _ = io::stdin().read(&mut b); mem[dp as usize] = b[0] as i64; },
            91 if mem[dp as usize] == 0 => i = sf(code, i),
            93 if mem[dp as usize] != 0 => i = sb(code, i),
            _ => {}
        }
        i += 1;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: bf CODE");
        return;
    }
    run(args[1].as_bytes());
}
