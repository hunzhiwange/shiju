#include <stdio.h>
#include <stdlib.h>
#include <math.h>
int main(int argc,char **v){if(argc!=3)return 2;double x=strtod(v[1],0),c=strtod(v[2],0);if(!isfinite(x)||!isfinite(c)){puts("INVALID");return 2;}printf("%.0f %d\n",floor(x),(int)(255.99*fmin(1,fmax(0,c))));return 0;}
