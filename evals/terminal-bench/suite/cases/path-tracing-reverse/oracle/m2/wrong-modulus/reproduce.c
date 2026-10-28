#include <stdio.h>
#include <stdlib.h>
#include <math.h>
int main(int argc,char **v){if(argc!=3)return 2;int n=atoi(v[1]),s=atoi(v[2]),sum=0;for(int i=0;i<n;i++){s=(17*s+23)%256;sum=(sum+s)%65521;}printf("%d %d\n",s,sum);return 0;}
