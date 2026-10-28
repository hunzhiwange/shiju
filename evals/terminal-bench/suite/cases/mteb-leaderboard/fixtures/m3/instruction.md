Rank with ties

Create /app/leaderboard.py. It reads one JSON object from standard input and writes exactly one JSON value to standard output. Use only the Python standard library. Do not access the network or write outside /app. example.json is synthetic, not a live leaderboard. The checker also supplies other inputs with the same schema. Budget: 24 turns, 32 tools, 900 seconds; one attempt.
Input has rows, each with model and mean_task (finite number), and positive integer k. Rank rows by mean_task descending using competition rank (1,1,3). Return objects with model and rank for all rows whose rank <= k, including ties at the cutoff. Equal scores are ordered by model ID ascending. Do not round scores.
