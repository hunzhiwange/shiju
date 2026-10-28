# Decode MIPS32 words
Input: {"words":[unsigned 32-bit integers]}. Output one object per word in order. Return all raw fields, even overlapping ones: opcode bits31..26, rs25..21, rt20..16, rd15..11, shamt10..6, funct5..0, immediate15..0, signed_immediate (16-bit two's complement), target25..0. All fields are JSON integers. Do not execute instructions.

Implement `/app/run.py` using Python 3 standard library. Read one JSON object from stdin and write exactly one JSON value to stdout, exit 0. No network or files outside /app. `example.json` is an illustrative input; grading sends additional inputs. Do not hardcode example answers.
