# fix-ocaml-gc diagnostics

Four independent synthetic executable exercises isolate compressed free-run
decoding, heap traversal, heap/cursor validation, and a minimal first-fit
allocation regression. The signed-header representation is deliberately
synthetic. No parent solution, OCaml source, hidden tests, or earlier model
trace enters the public fixtures. Each candidate writes `solution.py` for the
JSON stdin/stdout contract in its own instruction. The checker runs real
programs; allocation expectations use an independent word-ownership model.

The parent retains the complete frozen official task and original image
`sha256:66a1e2293076c9ef27a118611b3089a6284fdf467fd4675592d0cce7d11e5b0b`,
with one CPU, 2048MiB and separate 3600-second agent/verifier limits. Its
Registry/config/layer SHA256 values, complete diff_id chain and amd64 identity
were verified. Diagnostic fixtures use the digest-pinned Python image in
`case.json`, fresh offline one-CPU/512MiB containers, and the suite's unchanged
model/output/window policy. No paid model was invoked.

Calibration passed:

- The complete parent oracle bootstrapped and ran 40 individual basic tests:
  40 passed, zero failed. The official pytest check collected one test and
  passed it, producing reward 1 without an exception or timeout.
- The complete parent nop used the same frozen bootstrap and image. Its
  original runtime segfaulted during `coldstart`; the official pytest check
  collected one test and failed it, producing reward 0 without an exception
  or timeout. Both fresh verifier checkouts resolved to the official tag
  `356d558bf89552ed9229253ec4135c691234f060` and identical testsuite tree.
- Every child has a gold, a missing-program nop, and three faulty programs.
  All 20 executable fixture checks passed. A checker mutation that tested
  only the first trivial request caused 12 negative-fixture tests to fail;
  restoring the complete probes rejected all faulty programs.
- The four fresh Harbor child oracle/nop pairs produced rewards 1/0. Their
  original results, scorer receipts and materialized package bytes were
  revalidated and reused when the parent disk window resumed. No child
  control was repeated. Production `verify_preparation` accepted all ten
  controls against current inputs.

`validation.json` binds all declared case/checker/fixture/oracle bytes.
`preparation.json` binds the full source, runtime, dependencies and calibration.
Raw logs, CTRF, live parent resource observations, source identities, capacity
receipts and prior infrastructure failures remain in the ignored directory
`evals/promptfoo/results/qwen38-suite/diagnostics/fix-ocaml-gc/` and matching
preflight output. Parent live observations confirm one CPU, 2GiB and no host
mounts. The independent diagnostic resource probe confirms one CPU/512MiB;
child Harbor locks retain their declared enforced limits. Writable-layer quota
enforcement is not claimed.

`environment/verifier-bootstrap.sh` runs only after the parent agent exits.
It unpacks SHA-pinned authentic curl/uv/Python 3.13.7/pytest caches under
`/opt/vw-verifier-deps`, verifies uv binaries and Python imports, and bounds
redundant original apt calls while preserving their return codes. The original
image lacks Python, as recorded by the first dependency probe. Official source,
solution, tests, grading and time limits remain byte-identical. A separately
verified pure Git cache from the fresh official verifier clone was not used
by either parent control or exposed to an agent. Its unused extra copy was
removed for capacity after preserving the full provenance, reference, object
hash and fsck evidence. Old image refs differed from the current upstream
and were rejected.

Run the focused deterministic checks from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/fix-ocaml-gc/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/fix-ocaml-gc/case.json --run-fixtures
```

The historical `results/qwen38-suite/runtime.json` is an identity report, not
an execution configuration (`historical_root` is absent). This preparation
used the task-local generation-2 runtime configuration. The resumable helper
calls the same `inputs`, `harbor_trial` and `verify_preparation` APIs while
retaining the eight completed child controls. Failed capacity/TLS/tag/token
attempts remain infrastructure evidence and are not scored controls.
