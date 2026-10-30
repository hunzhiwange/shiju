#if 0
"""
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc,char **argv){ unsigned long long n=strtoull(argv[1],0,10); printf("%llu\n",n*n); return 0; }
#if 0
"""
import sys
n=int(sys.argv[1])
print(n*n)
#endif
