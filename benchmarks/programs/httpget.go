package main

import (
	"fmt"
	"io"
	"net"
	"os"
	"strings"
)

func parseURL(url string) (host, path string, port int, ok bool) {
	if !strings.HasPrefix(url, "http://") {
		return "", "", -1, false
	}
	rest := url[7:]
	slash := strings.IndexByte(rest, '/')
	hostPort := rest
	path = "/"
	if slash >= 0 {
		hostPort = rest[:slash]
		path = rest[slash:]
	}
	port = 80
	host = hostPort
	if i := strings.LastIndexByte(hostPort, ':'); i >= 0 {
		host = hostPort[:i]
		var p int
		if _, err := fmt.Sscanf(hostPort[i+1:], "%d", &p); err != nil {
			return "", "", -1, false
		}
		port = p
	}
	return host, path, port, true
}

func buildReq(host, path string) string {
	return "GET " + path + " HTTP/1.1\r\nHost: " + host + "\r\nConnection: close\r\n\r\n"
}

func readBody(conn net.Conn) {
	buf := make([]byte, 4096)
	acc := make([]byte, 0, 8192)
	for len(acc) < 8192 {
		n, err := conn.Read(buf)
		if n <= 0 {
			break
		}
		acc = append(acc, buf[:n]...)
		if i := strings.Index(string(acc), "\r\n\r\n"); i >= 0 {
			os.Stdout.Write(acc[i+4:])
			io.Copy(os.Stdout, conn)
			return
		}
		if err != nil {
			break
		}
	}
	io.Copy(os.Stdout, conn)
}

func fetch(url string) {
	host, path, port, ok := parseURL(url)
	if !ok {
		fmt.Fprintln(os.Stderr, "error: bad url")
		return
	}
	addr := fmt.Sprintf("%s:%d", host, port)
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error: connect failed")
		return
	}
	defer conn.Close()
	req := buildReq(host, path)
	if _, err := conn.Write([]byte(req)); err != nil {
		fmt.Fprintln(os.Stderr, "error: send failed")
		return
	}
	readBody(conn)
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: httpget http://HOST/PATH")
		os.Exit(1)
	}
	fetch(os.Args[1])
}
