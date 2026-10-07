#include <inttypes.h>
#include <stdio.h>
#include <string.h>

int glyph_argc_val;
char **glyph_argv_val;

void glyph_print_i(int64_t x) {
    printf("%" PRId64 "\n", x);
}

void glyph_print_str(const char *s) {
    puts(s);
}

void glyph_putchar(int64_t c) {
    putchar((int)c);
}

int64_t glyph_getchar(void) {
    return (int64_t)getchar();
}

int64_t glyph_strlen(const char *s) {
    return (int64_t)strlen(s);
}

int64_t glyph_char_at(const char *s, int64_t i) {
    return (unsigned char)s[i];
}

int64_t glyph_argc(void) {
    return glyph_argc_val;
}

const char *glyph_argv(int64_t i) {
    return glyph_argv_val[i];
}

int64_t _glyph_entry(void);

int main(int argc, char **argv) {
    glyph_argc_val = argc;
    glyph_argv_val = argv;
    _glyph_entry();
    return 0;
}
