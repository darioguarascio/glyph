#ifndef GLYPH_RT_H
#define GLYPH_RT_H

#include <stdio.h>
#include <stdint.h>
#include <inttypes.h>

static inline void glyph_print_i(int64_t x) {
    printf("%" PRId64 "\n", x);
}

static inline void glyph_print_f(double x) {
    printf("%g\n", x);
}

static inline void glyph_print_s(const char *s) {
    printf("%s\n", s);
}

#endif
