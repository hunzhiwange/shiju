# financial-document-processor m2

Work in /app. Inputs are synthetic UTF-8 text transcriptions in documents/; OCR is deliberately outside this narrow diagnostic.

Extract amounts from every synthetic invoice transcription. Write result.json: object mapping every filename to [total_amount, vat_amount], both decimal strings. Use the gross tax-inclusive Total, Grand Total, or Amount Due, never Subtotal/Net and never add tax again. VAT, GST, and Tax are equivalent tax amount labels.

Implement /app/solution.py using Python standard library. The verifier executes it from /app. Only write solution.py and the specified outputs; do not use network, change inputs (except required moves in this task), read hidden tests or expand permissions. Fresh environment, one attempt, 900 seconds, at most 24 turns and 32 tools; output budget 32000, input window 200000.
