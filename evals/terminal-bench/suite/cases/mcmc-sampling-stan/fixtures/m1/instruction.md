# Parse synthetic binomial group data

Create /app/solution.py. It reads one JSON object from stdin and writes exactly one JSON result to stdout, exit 0. Use Python 3 standard library only.

Input JSON contains csv: CSV text with exactly columns y,n (either order). Return {num_obs,y,n}, preserving row order and zero successes. Require at least one row, integer 0<=y<=n and n>0; reject malformed/missing columns, fractional counts, empty data or out of range with JSON null.

Public example input: {"csv":"y,n\n0,5\n2,7\n"}

Work only in /app; do not read /tests, /solution, host paths or network. The independent verifier executes multiple unseen synthetic inputs. Budget: 24 model turns, 32 tools, 900 seconds; one attempt.
