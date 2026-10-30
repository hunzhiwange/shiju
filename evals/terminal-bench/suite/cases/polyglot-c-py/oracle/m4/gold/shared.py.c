#if 0
"""
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc,char **argv){ long n=strtol(argv[1],0,10); printf("%ld %ld\n",n*n-3*n+7,(17*n+11)%97);return 0;}
#if 0
"""
import sys
n=int(sys.argv[1])
print(n*n-3*n+7, (17*n+11)%97)
#endif
