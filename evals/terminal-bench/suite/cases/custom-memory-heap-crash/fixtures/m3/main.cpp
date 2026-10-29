#include "user.h"
#include <cstdlib>
#include <cstdio>
#include <cstring>
static char *arena=nullptr, *node=nullptr;
static bool arena_owned=false, destroyed=false;
void touch_cache() {
 if(node) return;
#ifdef NDEBUG
 if(arena) { node=arena; arena_owned=true; }
 else node=static_cast<char*>(std::malloc(64));
#else
 node=static_cast<char*>(std::malloc(64));
#endif
 if(!node) std::abort();
 std::memset(node,7,64);
}
struct Finalizer {
 ~Finalizer() {
  std::puts("finalizer");
  if(arena_owned && destroyed) std::abort();
  if(node && !arena_owned) std::free(node);
  std::puts("cleanup-complete");
 }
} finalizer;
int main() {
 user_init();
 arena=static_cast<char*>(std::malloc(64));
 if(!arena) return 3;
 touch_cache();
 std::puts("work-complete");
 std::free(arena); arena=nullptr; destroyed=true;
 user_cleanup();
 return 0;
}
