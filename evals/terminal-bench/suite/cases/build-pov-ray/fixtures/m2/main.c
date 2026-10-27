#include <stdio.h>
#include <stdlib.h>
#include "shade.h"
int main(int argc,char **argv){if(argc!=2)return 2;printf("%d\n",shade(atoi(argv[1])));return 0;}
