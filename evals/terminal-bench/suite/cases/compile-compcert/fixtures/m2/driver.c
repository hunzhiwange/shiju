#include <unistd.h>
#include <stdio.h>
#ifndef COMPILER
#define COMPILER "/usr/local/bin/ccomp"
#endif
int main(int argc, char **argv) {
    (void)argc;
    argv[0] = COMPILER;
    execv(COMPILER, argv);
    perror("execv");
    return 127;
}
