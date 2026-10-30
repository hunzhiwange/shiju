# m2: Synthetic model CLI diagnostic

Work only in /app. Create solution.py using Python 3 standard library.

Implement `python3 solution.py weights.json vector.json`. Weights have w1 (2 rows x 3 columns), b1 (2 values), w2 (3 rows x 2 columns), b2 (3 values). Compute h=ReLU(w1*x+b1), then y=w2*h+b2 (no output ReLU). Return only the JSON list of three logits. Inputs are numeric and have the declared shapes. Test alternate vectors and weights; do not hardcode the example.

No network, host files, or external services. Limit: 24 model turns, 32 tool calls, 900 seconds; one attempt.
