#define _DEFAULT_SOURCE
#include "glyph_net.h"
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>

int64_t glyph_tcp_connect(const char *host, int64_t port) {
    char port_str[16];
    snprintf(port_str, sizeof port_str, "%ld", (long)port);
    struct addrinfo hints = {0}, *res = NULL, *rp = NULL;
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, port_str, &hints, &res) != 0) {
        return -1;
    }
    int fd = -1;
    for (rp = res; rp; rp = rp->ai_next) {
        fd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
        if (fd < 0) {
            continue;
        }
        if (connect(fd, rp->ai_addr, rp->ai_addrlen) == 0) {
            break;
        }
        close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    return fd;
}

int64_t glyph_tcp_send(int64_t fd, const char *buf, int64_t len) {
    int64_t sent = 0;
    while (sent < len) {
        ssize_t n = send((int)fd, buf + sent, (size_t)(len - sent), 0);
        if (n <= 0) {
            return -1;
        }
        sent += n;
    }
    return sent;
}

int64_t glyph_tcp_read(int64_t fd, char *buf, int64_t max) {
    ssize_t n = recv((int)fd, buf, (size_t)max, 0);
    return n < 0 ? -1 : (int64_t)n;
}

void glyph_tcp_close(int64_t fd) {
    close((int)fd);
}

void glyph_write_stdout(const char *buf, int64_t len) {
    if (len > 0) {
        fwrite(buf, 1, (size_t)len, stdout);
        fflush(stdout);
    }
}

int64_t glyph_parse_http_url(const char *url, char *host, int64_t hmax, char *path, int64_t pmax) {
    int64_t i = 0, h = 0, p = 0, st = 0, port = 80;
    if (strncmp(url, "http://", 7) != 0) {
        return -1;
    }
    i = 7;
    host[0] = path[0] = '\0';
    while (url[i]) {
        char c = url[i];
        if (st == 0) {
            if (c == ':') { st = 1; port = 0; i++; continue; }
            if (c == '/') { path[0] = '/'; p = 1; st = 2; i++; continue; }
            if (h + 1 >= hmax) return -1;
            host[h++] = c;
        } else if (st == 1) {
            if (c == '/') { path[0] = '/'; p = 1; st = 2; i++; continue; }
            if (c < '0' || c > '9') return -1;
            port = port * 10 + (c - '0');
        } else {
            if (p + 1 >= pmax) return -1;
            path[p++] = c;
        }
        i++;
    }
    host[h] = '\0';
    if (p == 0) { path[0] = '/'; path[1] = '\0'; }
    else path[p] = '\0';
    return port;
}

void glyph_http_read_body(int64_t fd) {
    char buf[4096];
    char acc[8192];
    int64_t acc_len = 0, n, i;
    while (acc_len < 8000 && (n = glyph_tcp_read(fd, buf, 4096)) > 0) {
        if (acc_len + n > 8191) n = 8191 - acc_len;
        memcpy(acc + acc_len, buf, (size_t)n);
        acc_len += n;
        for (i = 3; i < acc_len; i++) {
            if (acc[i - 3] == '\r' && acc[i - 2] == '\n' && acc[i - 1] == '\r' && acc[i] == '\n') {
                glyph_write_stdout(acc + i + 1, acc_len - i - 1);
                while ((n = glyph_tcp_read(fd, buf, 4096)) > 0) {
                    glyph_write_stdout(buf, n);
                }
                return;
            }
        }
    }
    while ((n = glyph_tcp_read(fd, buf, 4096)) > 0) {
        glyph_write_stdout(buf, n);
    }
}
