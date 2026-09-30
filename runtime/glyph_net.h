#ifndef GLYPH_NET_H
#define GLYPH_NET_H

#include <stdint.h>

int64_t glyph_tcp_connect(const char *host, int64_t port);
int64_t glyph_tcp_send(int64_t fd, const char *buf, int64_t len);
int64_t glyph_tcp_read(int64_t fd, char *buf, int64_t max);
void glyph_tcp_close(int64_t fd);
void glyph_write_stdout(const char *buf, int64_t len);
int64_t glyph_parse_http_url(const char *url, char *host, int64_t hmax, char *path, int64_t pmax);
void glyph_http_read_body(int64_t fd);

#endif
