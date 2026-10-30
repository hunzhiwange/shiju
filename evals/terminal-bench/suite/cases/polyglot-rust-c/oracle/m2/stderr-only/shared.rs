/* Rust nests this opening comment.
/* C++ closes here. */
#include <cstdio>
#include <cstdlib>
#include <cstring>
int main(int argc, char **argv) { long long n = atoll(argv[1]); long long value = n*(n+1)*(2*n+1)/6; fprintf(stderr, "%lld\n", value); return 0; }
const char *rust_text = R"SYNTHETIC(
// */
fn main() { let n: i64 = std::env::args().nth(1).unwrap().parse().unwrap(); let value: i64 = n*(n+1)*(2*n+1)/6; eprintln!("{}", value); }
/* Rust nests this closing comment.
/* C++ raw text ends here. */
)SYNTHETIC";
// */
