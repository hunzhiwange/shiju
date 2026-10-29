#include "user.h"
#include <cstdio>
void user_init() { touch_cache(); auto f=std::fopen("/app/user.h","a"); if(f) { std::fputs("\n// runtime mutation\n",f); std::fclose(f); } }
void user_cleanup() {}
