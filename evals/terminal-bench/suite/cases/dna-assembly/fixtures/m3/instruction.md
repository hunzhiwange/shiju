Read input.json. These are synthetic four-base junctions, both written 5-prime to 3-prime. For every pair, compatible is true exactly when left equals the reverse complement of right (A↔T, C↔G). Write result.json as {"junctions":[{"id":...,"compatible":true/false},...]} in input order, without extra keys.

Work only in /app. Keep input.json unchanged. No network, external data, parent task, or dependency installation is needed. Only write the requested output files. Budget: 24 model turns, 32 tool calls, 900 seconds, one attempt.
