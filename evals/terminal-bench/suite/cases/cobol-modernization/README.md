# cobol-modernization diagnostics

Four independent synthetic tasks prepare narrow checks of legacy modernization.
The official parent, its source data, reference solution, three pytest tests,
resources and 900-second agent/verifier timeouts remain unchanged. No model
requests, model scores, budget increases or model substitutions are involved.

- m1 decodes three 28-byte account records into typed JSON. It preserves record
  order and leading/internal name spaces, strips only trailing padding, and
  treats unsigned balances as whole cents. Controls reject decimal scaling,
  leading-space loss and reordered records.
- m2 ports an explicitly supplied synthetic COBOL tax calculation. The checker
  executes `calculate.py` on eight inputs using an independent Decimal
  oracle, including half ties, zero, full discount and large values. Controls
  reject truncation, tax on gross and ties-to-even rounding.
- m3 executes `update.py` on eight synthetic transactions. It tests all failed
  validation paths, unrelated records, zero amount, existing journal entries
  and buyer=seller. Independent debit and credit preserve the same-account
  balance. Controls reject omitted ownership validation, exclusive debit/credit
  branches and overwritten journal history.
- m4 executes `compare.py` on six fresh directory pairs. It compares three files
  byte-for-byte in a fixed order, detects missing files even on both sides,
  preserves binary NUL/CR/LF/space distinctions, ignores unrelated files, and
  rejects mutation of either directory. Controls reject whitespace stripping,
  ignored transaction output and treating two missing files as equal.

Gold programs are actual runnable implementations; the checker does not inspect
code keywords, final answers or agent self-tests. Candidate programs receive
only their input in a fresh temporary work directory, with a three-second
per-invocation timeout. JSON decoding rejects duplicate keys and nonfinite
numbers; canonical comparison distinguishes integer, float and boolean values.
Each child declares gold, nop and three meaningful negative controls.

The pinned Python image in `case.json` contains no parent or evaluator data.
Materialization copies only explicitly listed synthetic fixtures into each
child image. Harbor injects the checker and oracle metadata only during the
verifier phase, and gold only during oracle calibration. Fixtures run in fresh
offline containers with 1 CPU, 512 MiB, a read-only root, 128-process limit,
dropped capabilities and no-new-privileges. The four Harbor tasks each use a
fresh independent environment, 900-second agent / 120-second verifier limits,
and the frozen 24-request / 32-tool, 32000-output / 200000-input, one-attempt
Qwen3.8-Flash policy. The official parent keeps 1 CPU, 2048 MiB and 10240 MiB
storage; Docker's backend does not enforce a writable-layer disk quota.

Validation from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/cobol-modernization/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/cobol-modernization/case.json --run-fixtures
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/cobol-modernization/runtime.json --parent-id cobol-modernization --output evals/promptfoo/results/qwen38-suite/preflight/cobol-modernization-final
```

The output must be a new directory. The historical results/runtime.json is an
identity receipt, not scheduler configuration. The case-local ignored runtime
selects the calibrated generation-2 runtime and the original parent settings.
Its checksum-verified verifier-only dependency cache reuses the existing generic
uv 0.9.5 archive, Python 3.13.7 archive, pytest 8.4.1 / pytest-json-ctrf 0.3.5
wheels and generic bootstrap script from previous preparations. They are injected
after the agent exits under `/opt/vw-verifier-deps`; official test.sh is unchanged.
This is required because the container's astral.sh probe returned HTTP 403;
Debian and PyPI probes returned HTTP 200. Oracle's original apt commands ran in
the original parent image.

Raw red/green logs, image identity, dependency probes, runtime configuration,
Cargo receipts and acceptance evidence are retained under the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/cobol-modernization/` directory.
The preflight directory retains original Harbor trials, official rewards and
verifier logs. `validation.json` binds fixture/checker hashes; `preparation.json`
binds original parent bytes, runtime identity, dependency hashes and calibration
logs. The intentionally permissive checker failed 12 negative subcases before
the final black-box checker passed all 20 declared controls. Four unittest
methods also exercise seven malformed/nonfunctional/mutating controls and
check every numeric fixture against the declared COBOL field limits.

Final calibration: all 20 fixture controls and all ten fresh Harbor oracle/nop
controls matched expectations. The parent oracle passed all three official
pytest tests; nop failed the missing-program and output tests and passed only
the input-data-presence test. `verify_preparation` accepted the complete receipt.
The parent image digest is
`sha256:593ab9df3d83f771e927888b2d9436b380cdf7ec4d0af2a9f0802d541edebba0`
(linux/amd64, 162011504 compressed bytes, 431784448 uncompressed layer bytes).
The layer digests and uncompressed diff IDs match the loaded image. The
fixture Python image is independently digest-pinned in `case.json`.

All required commands exited 0 after the intentional checker red phase.
Clippy used dynamically claimed slot 1 with both `CARGO_TARGET_DIR` and
`CARGO_BUILD_BUILD_DIR` set to the absolute repository path ending in
`target/rust-slot-1`, and `CARGO_BUILD_JOBS=1`. Its lock was released and cache
retained. The existing block 0.1.6 future-compatibility notice remains; no Rust
files were changed. Detailed timestamps, command results, hashes and resource
observations are recorded in the ignored `acceptance.json` and associated logs.

Final review corrected one synthetic m2 rate from 17 to 1 basis point so that
the large-value case obeys the stated 12-digit total bound. The previous
preflight is retained as superseded evidence; final acceptance uses the fresh
`preflight/cobol-modernization-final/` receipt. The dedicated domain test checks
all eight numeric inputs. This correction was derived from the published input
contract, without consulting a model output or changing the scoring policy.
