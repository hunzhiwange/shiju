#include <stdio.h>
#include <stdlib.h>
#include <math.h>
int main(int argc,char **v){if(argc!=3)return 2;int x=atoi(v[1]),y=atoi(v[2]);printf("%d %d %d\n",(7*x+11*y+19)%256,(3*x+5*y+40)%256,(x+13*y+2)%256);return 0;}
