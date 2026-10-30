#if 0
"""
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc,char **argv){ unsigned long long n=strtoull(argv[1],0,10); printf("%llu\n",(unsigned long long)(unsigned int)(n*(n+1)/2)); return 0; }
#if 0
"""
import sys
n=int(sys.argv[1])
print(n*(n+1)//2)
#endif
