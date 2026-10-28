#include <stdio.h>
#include <stdlib.h>
#include <math.h>
int main(int argc,char **v){if(argc!=4)return 2;int w=atoi(v[1]),h=atoi(v[2]),s=atoi(v[3]);printf("P3\n%d %d\n255\n",w,h);for(int y=0;y<h;y++)for(int x=0;x<w;x++)printf("%d %d %d\n",(s+7*y)%256,(s+9*x)%256,(s+x+2*y)%256);return 0;}
