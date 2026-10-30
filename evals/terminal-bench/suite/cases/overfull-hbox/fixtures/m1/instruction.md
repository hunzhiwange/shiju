# m1: Reproduce layout warnings

Implement /app/answer.py, a Python 3 JSON stdin/stdout program. Read one JSON object from stdin containing main and input strings. Compile the provided LaTeX with pdflatex in a fresh temporary directory, no shell escape. Return only a JSON object with compiled (boolean, exit code zero), overfull (integer count of Overfull \hbox warnings in the fresh log), and pdf (boolean, fresh PDF exists and starts %PDF-). Warnings do not mean compilation failed.

The main.tex, input.tex and synonyms.txt files are a synthetic public example; hidden inputs use the same API and independent content. The verifier runs your program and independently compiles outputs with the installed real TeX engine. No network, package installation, privilege changes, persistent file writes outside temporary directories or changes to verifier files. Budget: 900 seconds, 24 turns, 32 tools, one attempt, 32000 output tokens, 200000 input tokens.
