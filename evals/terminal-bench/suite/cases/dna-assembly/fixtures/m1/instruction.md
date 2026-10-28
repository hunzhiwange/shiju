Read input.json. For each requests entry, select sequences[id][start:end] using zero-based half-open coordinates, uppercase it, and for strand "-" return its reverse complement (A↔T, C↔G); "+" keeps orientation. Write result.json as {"records":[{"id":...,"sequence":...},...]} in request order. Include every request, with no extra keys.

Work only in /app. Keep input.json unchanged. No network, external data, parent task, or dependency installation is needed. Only write the requested output files. Budget: 24 model turns, 32 tool calls, 900 seconds, one attempt.
