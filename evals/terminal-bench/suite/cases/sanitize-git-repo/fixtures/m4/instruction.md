Implement python3 solution.py ROOT MANIFEST: read only, scan current ROOT and
compare with the provided expected sanitized manifest. MANIFEST has hashes
(relative path -> SHA256), normal (normal paths), sanitized (contaminated paths).
Print exactly {"leaks": ARRAY, "preserved": BOOL, "placeholders_ok": BOOL,
"ready": BOOL}. ARRAY uses the occurrence format {path,line,kind}, sorted by
(path,line,kind), including duplicates. preserved is true iff the full current
working-tree file set equals hashes keys and each normal file matches its hash.
placeholders_ok is true iff every sanitized file exists and matches its expected
hash. ready is true iff leaks is empty and both booleans true. Do not repair or
modify ROOT or MANIFEST. Audit wrong placeholders even when no tokens remain.

All credentials here are fictional. Recognize only complete case-sensitive tokens
SYNTH_<KIND>_<SUFFIX>, where KIND is AWS_ID, AWS_SECRET, GITHUB or HF, and SUFFIX
is exactly eight uppercase ASCII letters/digits. Neither neighbor may be an ASCII
letter, digit or underscore. Near matches and existing placeholders are ordinary
content. Map AWS_ID to <your-aws-access-key-id>, AWS_SECRET to
<your-aws-secret-access-key>, GITHUB to <your-github-token>, HF to
<your-huggingface-token>. Preserve all other bytes, including CRLF and comments.
Submit /app/solution.py (Python standard library). Do not use network or change
permissions. Inputs are regular files/directories with no symlinks. Root inputs
are Git working trees: exclude every .git path component, include hidden files
and nested directories, and ignore non-UTF-8 binary files. Do not rewrite history.

Budget: 24 turns, 32 tools, 900 seconds, one attempt.
