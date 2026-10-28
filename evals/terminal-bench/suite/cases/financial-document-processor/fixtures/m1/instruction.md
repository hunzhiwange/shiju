# financial-document-processor m1

Work in /app. Inputs are synthetic UTF-8 text transcriptions in documents/; OCR is deliberately outside this narrow diagnostic.

Classify every synthetic document transcription as invoice or other. A payment invoice starts with INVOICE or TAX INVOICE. Purchase orders and memoranda are other even when they mention invoices or monetary totals. Write result.json: an object mapping every exact filename to invoice or other.

Implement /app/solution.py using Python standard library. The verifier executes it from /app. Only write solution.py and the specified outputs; do not use network, change inputs (except required moves in this task), read hidden tests or expand permissions. Fresh environment, one attempt, 900 seconds, at most 24 turns and 32 tools; output budget 32000, input window 200000.
