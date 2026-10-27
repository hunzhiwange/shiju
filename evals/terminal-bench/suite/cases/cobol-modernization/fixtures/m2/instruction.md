Port this synthetic COBOL calculation into /app/calculate.py. Read one JSON object from stdin with keys gross_cents and discount_cents (nonnegative integers, discount <= gross), and tax_basis_points (0..10000). Write exactly a JSON object {"net_cents": integer, "tax_cents": integer, "total_cents": integer} to stdout. The input values fit the declarations and total is under 10^12. All amounts are whole cents. Use exact decimal/integer arithmetic. This COBOL fragment defines the semantics:
    01 GROSS PIC 9(12).
    01 DISCOUNT PIC 9(12).
    01 RATE PIC 9(5).
    01 NET PIC 9(12).
    01 TAX PIC 9(12).
    01 TOTAL PIC 9(12).
    SUBTRACT DISCOUNT FROM GROSS GIVING NET
    COMPUTE TAX ROUNDED = NET * RATE / 10000
    ADD NET TO TAX GIVING TOTAL
ROUNDED means nearest integer, with exact half rounded upward (inputs are nonnegative). No external packages are available.
