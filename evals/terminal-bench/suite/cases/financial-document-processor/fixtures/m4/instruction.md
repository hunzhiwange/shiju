# financial-document-processor m4

Work in /app. Inputs are synthetic UTF-8 text transcriptions in documents/; OCR is deliberately outside this narrow diagnostic.

Process all synthetic document transcriptions: invoice documents begin with INVOICE, all others are other. Move each original unchanged from documents/ into invoices/ or other/, leaving documents/ empty. Create invoices/summary.csv with exactly filename,total_amount,vat_amount. Include each invoice once and a final total row summing both columns. Total is tax inclusive; missing VAT is zero. No other documents belong in the CSV.

Implement /app/solution.py using Python standard library. The verifier executes it from /app. Only write solution.py and the specified outputs; do not use network, change inputs (except required moves in this task), read hidden tests or expand permissions. Fresh environment, one attempt, 900 seconds, at most 24 turns and 32 tools; output budget 32000, input window 200000.
