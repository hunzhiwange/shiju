#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <errno.h>
int main(int argc, char **argv) {
    char *end;
    unsigned long x, r;
    uint32_t value, result;
    if (argc != 3 || argv[1][0] == '-' || argv[2][0] == '-') return 2;
    { int i; const char *p;
      for (i = 1; i < argc; ++i) {
        if (!argv[i][0]) return 2;
        for (p = argv[i]; *p; ++p) if (*p < '0' || *p > '9') return 2;
      }
    }
    errno = 0;
    x = strtoul(argv[1], &end, 10);
    if (errno || !argv[1][0] || *end || x > 4294967295UL) return 2;
    errno = 0;
    r = strtoul(argv[2], &end, 10);
    if (errno || !argv[2][0] || *end || r > 31) return 2;
    value = (uint32_t)x & 0x7fffffffU;
    result = r == 0 ? value : (value << r) | (value >> (32 - r));
    printf("%08x\n", (unsigned int)result);
    return 0;
}
