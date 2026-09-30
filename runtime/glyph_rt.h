#ifndef GLYPH_RT_H
#define GLYPH_RT_H

#include <stdio.h>
#include <stdint.h>
#include <inttypes.h>
#include <string.h>

static inline void glyph_print_i(int64_t x) {
    printf("%" PRId64 "\n", x);
}

static inline void glyph_print_f(double x) {
    printf("%g\n", x);
}

static inline void glyph_print_s(const char *s) {
    printf("%s\n", s);
}

static inline void glyph_putc(int64_t c) {
    putchar((int)c);
}

static inline int64_t glyph_getc(void) {
    return (int64_t)getchar();
}

static inline int64_t glyph_strlen(const char *s) {
    return (int64_t)strlen(s);
}

static inline int64_t glyph_char_at(const char *s, int64_t i) {
    return (unsigned char)s[i];
}

static inline int64_t glyph_argc(void) {
    extern int glyph_argc_val;
    return glyph_argc_val;
}

static inline const char *glyph_arg(int64_t i) {
    extern char **glyph_argv_val;
    return glyph_argv_val[i];
}

#endif
