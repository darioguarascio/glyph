#ifndef GLYPH_RT_H
#define GLYPH_RT_H

#include <stdint.h>

extern int glyph_argc_val;
extern char **glyph_argv_val;

void glyph_print_i(int64_t x);
void glyph_print_str(const char *s);
void glyph_putchar(int64_t c);
int64_t glyph_getchar(void);
int64_t glyph_strlen(const char *s);
int64_t glyph_char_at(const char *s, int64_t i);
int64_t glyph_argc(void);
const char *glyph_argv(int64_t i);

#endif
