#!/usr/bin/env python3
import socket
import sys

def parse_url(url):
    if not url.startswith("http://"):
        return None, None, -1
    rest = url[7:]
    host, _, tail = rest.partition("/")
    path = "/" + tail if tail else "/"
    if ":" in host:
        h, p = host.rsplit(":", 1)
        try:
            return h, path, int(p)
        except ValueError:
            return None, None, -1
    return host, path, 80

def adds(buf, pos, s):
    for ch in s:
        buf[pos] = ord(ch)
        pos += 1
    return pos

def build_req(host, path):
    req = bytearray(4096)
    n = 0
    n = adds(req, n, "GET ")
    n = adds(req, n, path)
    n = adds(req, n, " HTTP/1.1\r\nHost: ")
    n = adds(req, n, host)
    n = adds(req, n, "\r\nConnection: close\r\n\r\n")
    return bytes(req[:n])

def read_body(sock):
    acc = b""
    while b"\r\n\r\n" not in acc and len(acc) < 8192:
        chunk = sock.recv(4096)
        if not chunk:
            break
        acc += chunk
    sep = acc.find(b"\r\n\r\n")
    if sep >= 0:
        sys.stdout.buffer.write(acc[sep + 4:])
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        sys.stdout.buffer.write(chunk)

def fetch(url):
    host, path, port = parse_url(url)
    if port < 0:
        print("error: bad url", file=sys.stderr)
        return
    try:
        sock = socket.create_connection((host, port))
    except OSError:
        print("error: connect failed", file=sys.stderr)
        return
    try:
        data = build_req(host, path)
        sock.sendall(data)
        read_body(sock)
    except OSError:
        print("error: send failed", file=sys.stderr)
    finally:
        sock.close()

def main():
    if len(sys.argv) < 2:
        print("usage: httpget http://HOST/PATH", file=sys.stderr)
        sys.exit(1)
    fetch(sys.argv[1])

if __name__ == "__main__":
    main()
