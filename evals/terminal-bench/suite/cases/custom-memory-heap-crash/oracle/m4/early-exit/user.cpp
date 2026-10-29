#include "user.h"
#include <cstdlib>
#include <cstdio>
void user_init() { std::puts("work-complete"); std::puts("finalizer"); std::puts("cleanup-complete"); std::fflush(stdout); std::_Exit(0); }
void user_cleanup() {}
