Implement python3 solution.py FILE: replace every fictional token in that UTF-8
file with its kind-specific placeholder in place, preserving all other bytes.
Repeated values and multiple types on one line must all be replaced. Do not
create/delete files or change any other input. Stdout is not graded.

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
