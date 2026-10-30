/* Rust nests this opening comment.
/* C++ closes here. */
#include <cstdio>
#include <cstdlib>
#include <cstring>
int main(int argc, char **argv) { if(argc!=2) return 2; size_t len=strlen(argv[1]); if(len<1 || len>6) return 2; for(size_t j=0;j<len;j++) if(argv[1][j]!='+' && argv[1][j]<'0'||argv[1][j]>'9') return 2; long long n=atoll(argv[1]); long long value = n*1000000007+23; printf("%lld\n", value); return 0; }
const char *rust_text = R"SYNTHETIC(
// */
fn main() { let args: Vec<String> = std::env::args().skip(1).collect(); if args.len()!=1 || args[0].is_empty() || args[0].len()>6 || !args[0].bytes().all(|c| c.is_ascii_digit() || c == b'+') { std::process::exit(2); } let n: i64 = args[0].parse().unwrap(); let value: i64 = n*1000000007+23; println!("{}", value); }
/* Rust nests this closing comment.
/* C++ raw text ends here. */
)SYNTHETIC";
// */
