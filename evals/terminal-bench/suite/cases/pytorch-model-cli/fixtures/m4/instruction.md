# m4: Synthetic model CLI diagnostic

Work only in /app. Create solution.py using Python 3 standard library.

Implement `python3 solution.py logits.json`. Input is a JSON array of ten finite logits. Choose the largest logit; ties choose the lowest digit index. Exit 0, stderr empty, stdout exactly the digit plus one newline, and overwrite prediction.txt with exactly the same bytes. Test multiple logits, ties, and overwriting a stale prediction file. No additional output or whitespace is allowed.

No network, host files, or external services. Limit: 24 model turns, 32 tool calls, 900 seconds; one attempt.
