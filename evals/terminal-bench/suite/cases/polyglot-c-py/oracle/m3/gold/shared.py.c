#if 0
"""
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc,char **argv){ if(argc!=2 || !argv[1][0]) return 2; unsigned long long n=0,v=1; for(char *p=argv[1];*p;p++){if(*p<'0'||*p>'9') return 2; n=n*10+(*p-'0'); if(n>20)return 2;} for(unsigned i=1;i<=n;i++)v*=i; printf("%llu\n",v);return 0;}
#if 0
"""
import sys
if len(sys.argv)!=2 or not sys.argv[1] or any(c not in "0123456789" for c in sys.argv[1]) or int(sys.argv[1])>20:
 sys.exit(2)
n=int(sys.argv[1]); v=1
for i in range(1,n+1): v*=i
print(v)
#endif
