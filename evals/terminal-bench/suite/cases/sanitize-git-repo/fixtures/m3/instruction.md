Implement python3 solution.py ROOT: sanitize every UTF-8 working-tree file in
place using the mapping above. Preserve exact paths, bytes outside matches,
all normal files, binary assets and .git contents. Do not create/delete files or
modify executable permissions. Stdout is not graded.

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
