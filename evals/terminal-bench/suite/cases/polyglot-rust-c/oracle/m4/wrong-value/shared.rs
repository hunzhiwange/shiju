/* Rust nests this opening comment.
/* C++ closes here. */
#include <cstdio>
#include <cstdlib>
#include <cstring>
int main(int argc, char **argv) { long long n = atoll(argv[1]); long long value = (n*n+5*n+19)+1; printf("%lld %lld\n", value, (n*n+31)%103); return 0; }
const char *rust_text = R"SYNTHETIC(
// */
fn main() { let n: i64 = std::env::args().nth(1).unwrap().parse().unwrap(); let value: i64 = (n*n+5*n+19)+1; println!("{} {}", value, (n*n+31)%103); }
/* Rust nests this closing comment.
/* C++ raw text ends here. */
)SYNTHETIC";
// */
