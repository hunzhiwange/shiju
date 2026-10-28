Read a fixed snapshot

Create /app/leaderboard.py. It reads one JSON object from standard input and writes exactly one JSON value to standard output. Use only the Python standard library. Do not access the network or write outside /app. example.json is synthetic, not a live leaderboard. The checker also supplies other inputs with the same schema. Budget: 24 turns, 32 tools, 900 seconds; one attempt.
Input has snapshots (objects with date and rows) and as_of. Choose the latest snapshot whose ISO YYYY-MM-DD date is <= as_of, and output its rows unchanged in input order. Return [] if no snapshot qualifies. Do not merge snapshots or use current data.
