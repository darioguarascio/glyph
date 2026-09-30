#define _DEFAULT_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>

static char host[256], path[2048], req[4096];

static int64_t parse_url(const char *url) {
    int64_t i = 7, h = 0, p = 0, st = 0, port = 80;
    if (strncmp(url, "http://", 7) != 0) return -1;
    host[0] = path[0] = '\0';
    while (url[i]) {
        char c = url[i++];
        if (st == 0) {
            if (c == ':') { st = 1; port = 0; continue; }
            if (c == '/') { path[0] = '/'; p = 1; st = 2; continue; }
            if (h + 1 >= (int64_t)sizeof host) return -1;
            host[h++] = c;
        } else if (st == 1) {
            if (c == '/') { path[0] = '/'; p = 1; st = 2; continue; }
            if (c < '0' || c > '9') return -1;
            port = port * 10 + (c - '0');
        } else {
            if (p + 1 >= (int64_t)sizeof path) return -1;
            path[p++] = c;
        }
    }
    host[h] = '\0';
    if (p == 0) { path[0] = '/'; path[1] = '\0'; }
    else path[p] = '\0';
    return port;
}

static int64_t adds(int64_t pos, const char *s) {
    while (*s) req[pos++] = *s++;
    return pos;
}

static int64_t build_req(void) {
    int64_t n = 0;
    n = adds(n, "GET ");
    n = adds(n, path);
    n = adds(n, " HTTP/1.1\r\nHost: ");
    n = adds(n, host);
    n = adds(n, "\r\nConnection: close\r\n\r\n");
    return n;
}

static int connect_host(int64_t port) {
    char port_str[16];
    snprintf(port_str, sizeof port_str, "%ld", (long)port);
    struct addrinfo hints = {0}, *res = NULL, *rp;
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, port_str, &hints, &res) != 0) return -1;
    int fd = -1;
    for (rp = res; rp; rp = rp->ai_next) {
        fd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
        if (fd < 0) continue;
        if (connect(fd, rp->ai_addr, rp->ai_addrlen) == 0) break;
        close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    return fd;
}

static int send_all(int fd, const char *buf, int64_t len) {
    int64_t sent = 0;
    while (sent < len) {
        ssize_t n = send(fd, buf + sent, (size_t)(len - sent), 0);
        if (n <= 0) return -1;
        sent += n;
    }
    return 0;
}

static void read_body(int fd) {
    char buf[4096], acc[8192];
    int64_t acc_len = 0, n, i;
    while (acc_len < 8000 && (n = recv(fd, buf, sizeof buf, 0)) > 0) {
        if (acc_len + n > 8191) n = 8191 - acc_len;
        memcpy(acc + acc_len, buf, (size_t)n);
        acc_len += n;
        for (i = 3; i < acc_len; i++) {
            if (acc[i - 3] == '\r' && acc[i - 2] == '\n' && acc[i - 1] == '\r' && acc[i] == '\n') {
                fwrite(acc + i + 1, 1, (size_t)(acc_len - i - 1), stdout);
                while ((n = recv(fd, buf, sizeof buf, 0)) > 0)
                    fwrite(buf, 1, (size_t)n, stdout);
                return;
            }
        }
    }
    while ((n = recv(fd, buf, sizeof buf, 0)) > 0)
        fwrite(buf, 1, (size_t)n, stdout);
}

static void fetch(const char *url) {
    int64_t port = parse_url(url);
    if (port < 0) { fputs("error: bad url\n", stderr); return; }
    int fd = connect_host(port);
    if (fd < 0) { fputs("error: connect failed\n", stderr); return; }
    int64_t n = build_req();
    if (send_all(fd, req, n) < 0) {
        close(fd);
        fputs("error: send failed\n", stderr);
        return;
    }
    read_body(fd);
    close(fd);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fputs("usage: httpget http://HOST/PATH\n", stderr);
        return 1;
    }
    fetch(argv[1]);
    return 0;
}
