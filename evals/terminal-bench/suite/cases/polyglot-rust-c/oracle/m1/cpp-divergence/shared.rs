/* Rust nests this opening comment.
/* C++ closes here. */
#include <cstdio>
#include <cstdlib>
#include <cstring>
int main(int argc, char **argv) { long long n=0; long long value = (43)+2; printf("bridge-%lld\n", value); return 0; }
const char *rust_text = R"SYNTHETIC(
// */
fn main() { let n: i64 = 0; let value: i64 = 43; println!("bridge-{}", value); }
/* Rust nests this closing comment.
/* C++ raw text ends here. */
)SYNTHETIC";
// */
