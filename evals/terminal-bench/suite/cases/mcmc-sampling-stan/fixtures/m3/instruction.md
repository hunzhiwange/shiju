# Run seeded chains and measure convergence

Create /app/solution.py. It reads one JSON object from stdin and writes exactly one JSON result to stdout, exit 0. Use Python 3 standard library only.

Input JSON contains positive weights for a finite posterior grid, starts (four integer grid indices), seed, iterations>=200, burn=half iterations. Run four independent Metropolis chains: RNG random.Random(seed+chain_index); each iteration propose current+(-1 if rng.random()<0.5 else 1); draw a second rng.random() even for out-of-range proposals; accept in-range proposal iff u<min(1,weights[proposal]/weights[current]). Record state after each transition; discard first burn draws. Return {draws: four retained integer lists,rhat: conventional unsplit Gelman-Rubin Rhat}. For n retained draws use sample variances (ddof=1), W=mean within variance, B=n*sample_variance(chain means), varplus=(n-1)/n*W+B/n; Rhat=sqrt(varplus/W). Cases have W>0. Exact seeded draws and numeric Rhat are required; this diagnoses a narrow sampler, not full Stan convergence.

Public example input: {"weights":[1,2,1],"starts":[0,1,2,1],"seed":1,"iterations":200,"burn":100}

Work only in /app; do not read /tests, /solution, host paths or network. The independent verifier executes multiple unseen synthetic inputs. Budget: 24 model turns, 32 tools, 900 seconds; one attempt.
