#include <stdio.h>
#include <stdint.h>

static int64_t mem[30000];

static int64_t sf(const char *code, int64_t i) {
    int64_t d = 1;
    while (d > 0) {
        i++;
        int64_t x = (unsigned char)code[i];
        if (x == 91) d++;
        if (x == 93) d--;
    }
    return i;
}

static int64_t sb(const char *code, int64_t i) {
    int64_t d = 1;
    while (d > 0) {
        i--;
        int64_t x = (unsigned char)code[i];
        if (x == 93) d++;
        if (x == 91) d--;
    }
    return i;
}

static void run(const char *code) {
    int64_t dp = 0, i = 0, len = 0;
    while (code[len]) len++;
    while (i < len) {
        int64_t ch = (unsigned char)code[i];
        if (ch == 62) dp++;
        else if (ch == 60) dp--;
        else if (ch == 43) mem[dp]++;
        else if (ch == 45) mem[dp]--;
        else if (ch == 46) putchar((int)mem[dp]);
        else if (ch == 44) mem[dp] = getchar();
        else if (ch == 91 && mem[dp] == 0) i = sf(code, i);
        else if (ch == 93 && mem[dp] != 0) i = sb(code, i);
        i++;
    }
}

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("usage: bf CODE\n");
        return 1;
    }
    run(argv[1]);
    return 0;
}
