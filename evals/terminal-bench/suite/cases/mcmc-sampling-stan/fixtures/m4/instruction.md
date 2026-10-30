# Summarize posterior draws without warmup

Create /app/solution.py. It reads one JSON object from stdin and writes exactly one JSON result to stdout, exit 0. Use Python 3 standard library only.

Input JSON contains alpha and beta, four equally sized nonempty chains of finite posterior samples, plus warmup (number of initial draws per chain to discard; at least one draw remains). Pool retained draws with equal draw weights. Return {alpha:{mean,q05,q95},beta:{mean,q05,q95}}. Compute linear interpolated quantiles at positions (N-1)*p in sorted pooled samples, p=.05,.95. Reject nonfinite, unequal shapes, nonpositive alpha/beta, or warmup outside [0,chain_length) with null. Do not round results.

Public example input: {"alpha":[[90,1,2],[90,2,3],[90,3,4],[90,4,5]],"beta":[[80,5,6],[80,6,7],[80,7,8],[80,8,9]],"warmup":1}

Work only in /app; do not read /tests, /solution, host paths or network. The independent verifier executes multiple unseen synthetic inputs. Budget: 24 model turns, 32 tools, 900 seconds; one attempt.
