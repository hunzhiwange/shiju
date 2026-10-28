# financial-document-processor m3

Work in /app. Inputs are synthetic UTF-8 text transcriptions in documents/; OCR is deliberately outside this narrow diagnostic.

Resolve every synthetic invoice transcription independently. Write result.json: object mapping every filename to [total_amount, vat_amount], both decimal strings. Prefer Total over Amount Due regardless of line order; otherwise use Amount Due. Missing VAT/Tax/GST must be zero (0 or empty string also accepted); never infer a tax rate.

Implement /app/solution.py using Python standard library. The verifier executes it from /app. Only write solution.py and the specified outputs; do not use network, change inputs (except required moves in this task), read hidden tests or expand permissions. Fresh environment, one attempt, 900 seconds, at most 24 turns and 32 tools; output budget 32000, input window 200000.
