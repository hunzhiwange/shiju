# Implement an executable hierarchical model and Metropolis entry

Create /app/solution.py. It reads one JSON object from stdin and writes exactly one JSON result to stdout, exit 0. Use Python 3 standard library only.

Input JSON gives y,n,alpha,beta,theta,proposal (theta vectors) and u in (0,1). Implement log joint density: sum_i log Binomial(y_i|n_i,theta_i)+log Beta(theta_i|alpha,beta) minus 2.5 log(alpha+beta), for positive alpha,beta and theta in (0,1). Constants from Binomial and Beta must be included. For a symmetric proposal implement one Metropolis step accepting iff log(u)<min(0, proposed_log_joint-current_log_joint). Return {log_joint: current value,accepted: boolean,theta: selected vector}. This is a controlled actual sampling transition; do not return a fitted point estimate. Inputs obey these domains.

Public example input: {"y":[1],"n":[4],"alpha":2,"beta":3,"theta":[0.3],"proposal":[0.4],"u":0.5}

Work only in /app; do not read /tests, /solution, host paths or network. The independent verifier executes multiple unseen synthetic inputs. Budget: 24 model turns, 32 tools, 900 seconds; one attempt.
