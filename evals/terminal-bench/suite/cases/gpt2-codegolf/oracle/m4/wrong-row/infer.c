#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
static uint32_t u32(FILE*f){unsigned char b[4];if(fread(b,1,4,f)!=4)exit(2);return (uint32_t)b[0]|(uint32_t)b[1]<<8|(uint32_t)b[2]<<16|(uint32_t)b[3]<<24;}
static float val(FILE*f){uint32_t u=u32(f);float v;memcpy(&v,&u,4);return v;}
int main(int argc,char**argv){if(argc!=4)return 2;FILE*f=fopen(argv[1],"rb");if(!f)return 2;uint32_t v=u32(f),d=u32(f);if(!v||v>16||!d||d>16)return 2;float w[256];for(unsigned i=0;i<v*d;i++)w[i]=val(f);float b[16];for(unsigned i=0;i<v;i++)b[i]=val(f);if(fgetc(f)!=EOF)return 2;fclose(f);char*end;long t=strtol(argv[3],&end,10);if(*end||t<0||t>=v)return 2;unsigned best=0;float max=0;for(unsigned i=0;i<v;i++){float s=b[i];for(unsigned j=0;j<d;j++)s+=w[j]*w[i*d+j];if(i==0||s>max){max=s;best=i;}}f=fopen(argv[2],"r");if(!f)return 2;char word[128];for(unsigned i=0;i<=best;i++)if(!fgets(word,sizeof(word),f))return 2;fclose(f);printf("%s",word);return 0;}
