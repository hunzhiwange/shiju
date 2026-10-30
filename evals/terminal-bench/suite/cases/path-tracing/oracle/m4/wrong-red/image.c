#include <stdio.h>
#include <stdlib.h>
int main(int n,char **v){if(n!=4)return 1;int w=atoi(v[1]),h=atoi(v[2]),s=atoi(v[3]);FILE*f=fopen("reconstructed.ppm","w");if(!f)return 2;fprintf(f,"P3\n%d %d\n255\n",w,h);for(int y=0;y<h;y++)for(int x=0;x<w;x++)fprintf(f,"%d %d %d\n",(16*x+s)%256,(31*y+s)%256,(x+3*y+s)%256);return fclose(f)!=0;}
