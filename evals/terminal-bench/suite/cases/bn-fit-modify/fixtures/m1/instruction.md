Inspect synthetic sample.csv and constraints.json. Write /app/schema.json with exactly these keys: columns (CSV names in order), numeric_columns (columns whose every value parses as a finite number, in order), root, edge_count, ambiguous_direction. The last three describe the graph constraints; encode the direction rule as "later-to-earlier" or "earlier-to-later". Do not learn graph edges in this task.

Work in /app. Only write the requested output files; no network or external data. Each task is independent. Budget: 900 seconds, 24 model requests, 32 tool executions.
