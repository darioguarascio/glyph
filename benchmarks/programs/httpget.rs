use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;

fn parse_url(url: &str) -> Option<(String, String, u16)> {
    let rest = url.strip_prefix("http://")?;
    let (host_port, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if let Some(i) = host_port.rfind(':') {
        let host = host_port[..i].to_string();
        let port: u16 = host_port[i + 1..].parse().ok()?;
        return Some((host, path.to_string(), port));
    }
    Some((host_port.to_string(), path.to_string(), 80))
}

fn build_req(host: &str, path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    )
}

fn read_body(stream: &mut TcpStream) -> io::Result<()> {
    let mut acc = Vec::new();
    let mut buf = [0u8; 4096];
    while acc.len() < 8192 {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        acc.extend_from_slice(&buf[..n]);
        if let Some(i) = acc.windows(4).position(|w| w == b"\r\n\r\n") {
            stdout().write_all(&acc[i + 4..])?;
            io::copy(stream, &mut stdout())?;
            return Ok(());
        }
    }
    io::copy(stream, &mut stdout())?;
    Ok(())
}

fn fetch(url: &str) {
    let Some((host, path, port)) = parse_url(url) else {
        eprintln!("error: bad url");
        return;
    };
    let addr = format!("{host}:{port}");
    let mut stream = match TcpStream::connect(&addr) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error: connect failed");
            return;
        }
    };
    let req = build_req(&host, &path);
    if stream.write_all(req.as_bytes()).is_err() {
        eprintln!("error: send failed");
        return;
    }
    let _ = read_body(&mut stream);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: httpget http://HOST/PATH");
        std::process::exit(1);
    }
    fetch(&args[1]);
}
