#!/usr/bin/env node
const net = require("net");

function parseUrl(url) {
  if (!url.startsWith("http://")) return null;
  const rest = url.slice(7);
  const slash = rest.indexOf("/");
  const hostPort = slash >= 0 ? rest.slice(0, slash) : rest;
  const path = slash >= 0 ? rest.slice(slash) : "/";
  const colon = hostPort.indexOf(":");
  if (colon >= 0) {
    const host = hostPort.slice(0, colon);
    const port = Number(hostPort.slice(colon + 1));
    if (!Number.isFinite(port)) return null;
    return { host, path, port };
  }
  return { host: hostPort, path, port: 80 };
}

function buildReq(host, path) {
  return (
    "GET " + path + " HTTP/1.1\r\nHost: " + host + "\r\nConnection: close\r\n\r\n"
  );
}

function readBody(sock) {
  let acc = Buffer.alloc(0);
  sock.on("data", (chunk) => {
    acc = Buffer.concat([acc, chunk]);
    const sep = acc.indexOf("\r\n\r\n");
    if (sep >= 0) {
      process.stdout.write(acc.slice(sep + 4));
      acc = Buffer.alloc(0);
      sock.removeAllListeners("data");
      sock.on("data", (c) => process.stdout.write(c));
    }
  });
}

function fetch(url) {
  const u = parseUrl(url);
  if (!u) {
    process.stderr.write("error: bad url\n");
    return;
  }
  const sock = net.createConnection({ host: u.host, port: u.port }, () => {
    sock.write(buildReq(u.host, u.path));
    readBody(sock);
  });
  sock.on("error", () => process.stderr.write("error: connect failed\n"));
}

function main() {
  const url = process.argv[2];
  if (!url) {
    process.stderr.write("usage: httpget http://HOST/PATH\n");
    process.exit(1);
  }
  fetch(url);
}

main();
