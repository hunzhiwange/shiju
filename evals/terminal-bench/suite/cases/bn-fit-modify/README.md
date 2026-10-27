# bn-fit-modify diagnostics

These four independent synthetic tasks isolate schema/constraint reading (m1),
three-node collider recovery (m2), surgical constant intervention (m3), and
edge/sample CSV production (m4). They do not contain the official dataset,
reference solution, hidden tests, or model output. The original full task and
its scorer remain unchanged, including their respective CSV conventions.

Each child has a complete prompt under `fixtures/mN/instruction.md`.
`case.json` is the explicit public/hidden allowlist. Only synthetic input files
enter the agent environment. `oracle/` contains independently calculated gold
artifacts and negative mutations; `check.py` reads actual candidate files.
No final-answer text or agent-authored test counts as evidence.

m2 enumerates all 27 triples of independent centered U,V,noise in {-1,0,1}
and sets W=U+2V+noise. U and V are marginally independent but become dependent
conditional on W, fixing the collider. m3 replaces the entire Y mechanism,
retaining its outgoing influence. m4 uses eight fixed innovation draws to
make the downstream structural equations and CSV sample checks deterministic;
it is a narrow diagnostic, not a substitute for the parent's 10k statistical
sampling test. CSV headers, duplicate edges, row counts, finite values and
all sample cells are checked. Edge row order is unrestricted.

The diagnostic base is the clean official Python 3.13.7 slim-bookworm ARM64
manifest `sha256:fcf02c9e248b995ae2ca9aac6fa24b489f34b589dbfdc44698ad245d2ad41d1e`.
Native architecture avoids overriding Harbor's separately built egress sidecar.
It contains no parent input, solutions, checker, or credentials. No parent or
repository is mounted. Fixture calibration runs offline with read-only rootfs,
1 CPU, 512MiB, 128 PIDs, dropped capabilities, and read-only checker/oracle
mounts. Harbor injects checker and solution only in their respective phases.

The parent stays on its original amd64 image and official package bytes.
Its optional service-mode `recovery.bootstrap` supplies authentic uv 0.9.5
(archive SHA256 `2cf10babba653310606f8b49876cfb679928669e7ddaa1fb41fb00ce73e64f66`)
and Python 3.13.7 caches under `/opt/vw-verifier-deps/` only after the agent
exits. `environment/verifier-bootstrap.sh` extracts that interpreter; the
unmodified official installer consumes the uv archive via `UV_DOWNLOAD_URL`.
A complete 13-wheel closure is fetched from official PyPI release URLs and
verified against its published SHA256 values. `UV_FIND_LINKS` and `UV_OFFLINE`
make the unmodified uvx command consume those authentic wheels without network
access. All supplied files are pinned and checked on both sides. The runtime settings
and `scoring/bootstrap.json` bind cache digests and setup output. Original
failed scoring remains under `preflight/bn-fit-modify/`; no failure is hidden.

Validation (repository root, no model calls):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/bn-fit-modify/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/bn-fit-modify/case.json --run-fixtures
```

`validation.json` binds every declared byte to 24 fresh-container checks:
4 gold passes, 4 nop rejections, and 16 meaningful negative rejections.
Removing the finite-value guard demonstrably accepts NaN and fails the
regression test; restoring it rejects the mutation. Local raw evidence is
kept in the ignored `results/qwen38-suite/diagnostics/bn-fit-modify/` directory.
Parent oracle/nop and four-child Harbor preflight receipts must additionally
pass before this parent is ready for the suite freeze.

The reproducible local execution configuration is
`evals/promptfoo/results/qwen38-suite/diagnostics/bn-fit-modify/resume/runtime-wheelhouse.json`;
`wheel-manifest.json` records each official release URL, version, size, and hash.
`python-origin.json` binds the Python cache to its reviewed source image.
No cache archives, official parent material, credentials, or historical model
outputs belong in this case's public fixture maps.

`preparation.json` is a local machine-bound receipt and is intentionally ignored;
`validation.json` is portable and binds the synthetic fixtures and checker.

Preparation verified on 2026-10-27: the parent passed all 9 official tests;
parent and m1..m4 each scored oracle=1/nop=0 in 10 fresh Harbor trials.
The final preparation receipt passed `verify_preparation` against current
inputs after the upstream task-name fix. Earlier executions retain their
original identities and times. The final audit binds 18 container/resource
records, original amd64 image
`alexgshaw/bn-fit-modify@sha256:5aeb11ca6e802b83816c9a31f055ac9d028309a853509400c2e3139ac2d8380d`,
native diagnostic image, input/checker/fixture/cache hashes, and zero model
requests. Main containers kept official CPU/memory limits; the native egress
sidecars were capped at 0.25 CPU/128MiB without changing networking policy.
Raw final audit: `evals/promptfoo/results/qwen38-suite/diagnostics/bn-fit-modify/resume/acceptance.json`.
Clippy passed with slot 1 (`target/rust-slot-1`); its lock was released and cache
retained. Focused shared regressions passed 5 bootstrap + 2 scoring + 9 preparation
tests; the case's 2 unit tests and 24 isolated fixture checks passed.

Earlier dependency-download reward=0 traces never reached pytest and are
infrastructure failures, not model capability scores; the final audit explicitly
classifies them while preserving their raw receipts.
