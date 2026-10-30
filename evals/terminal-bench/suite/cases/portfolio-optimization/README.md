# Portfolio optimization diagnostics

Four independent synthetic tasks cover matrix/weight layout (m1), small risk
and return kernels (m2), precision and invalid input boundaries (m3), and
compiling/importing/timing a genuine CPython C extension (m4). They contain no
official task source, solution, hidden tests, data or model output. Each prompt
states its input, output/API, write boundary and budget independently. The
native diagnostic uses Python lists and the CPython C API; the original parent
separately covers the NumPy wrapper and 8000-asset scale.

The checker executes submissions in separate interpreters on unseen synthetic
inputs. m3 checks empty input, signed weights, ragged shapes, nonfinite values
and negative variance. m4 independently compiles native.c with Python headers,
-O3, -fPIC and libm, requires ExtensionFileLoader and a native callable, and
checks results within 1e-10. It times three real calls at n=320 against its own
nested-loop baseline and requires >=1.2x speedup. Submitted reports and self-tests
cannot satisfy these checks. Each child has gold, nop and three distinct
semantic negative controls. The permissive initial checker produced 18 red
failures; the corrected checker passed all 20 real-process host controls.

The diagnostic tool is pinned to the reviewed, clean linux/amd64 Docker v2 leaf
`vw-log-summary-diagnostics:task52-single@sha256:32c8a19cdeab820f8ea710fcdacaef0893848898158f0d3ccc890a75ab365adf`
(config `sha256:a6c16930afacbb61ef2e9f1e5f994d6c560b787fa4c5f849b5c15b3f017d111c`).
It has Python 3.13.7, gcc 12.2.0 and Python headers, with no parent materials.
Only each synthetic example.json enters the materialized environment. Checker,
oracle and gold are injected in verifier/solution phases. No new tool image,
Agent dependency, platform override or DOCKER_DEFAULT_PLATFORM was introduced.

The original parent leaf is
`sha256:93461db3cec697c31585941a3b9387c1398495aa1ff0aee2ac752c7230b50ef8`
(config `sha256:b0ef263fdd85ca609faafead645c86c8a03f534178b3368e3eedc0a273b3fed9`).
All 12 compressed layer SHA-256 values and complete decompressed diff_ids were
verified: C=196,450,772 bytes; U=592,377,344 bytes. Initial admission used three
host/VM samples and a positive 2GiB decompression cap; import streamed OCI
without an extra archive file, then checked exact Config/RootFS and platform.
Parent S10GiB and diagnostic S1GiB reservations remain separate, each retaining
B2+A1+safety5GiB+128MiB metadata. Cached reuse adds no transfer/archive copies.
Capacity interruptions and busy shared-resource admission were preserved;
continuation rechecked completed score hashes before running remaining controls.
No business container, shared cache or another task's resource was removed.

Fresh 2026-10-31 calibration passed all 20 Linux fixture controls and ten standard
Harbor oracle/nop controls through public suite.scoring.harbor_trial and
SuiteDockerEnvironment. The complete preparation receipt passes the shared
verify_preparation validator. Execution identity:
`588690e423e955cab7b2c335d302678cae81598b2ec421946065c88f964ce16c`.
Every main container was created with 1 CPU, parent 4096MiB / children 512MiB
RAM and matching swap limits; diagnostic sidecars had 0.25 CPU / 64MiB.
Startup used up --no-build and the shared limited builder/admission policy.
Original source, official checker, scoring thresholds, storage reservations and
3600-second parent agent/verifier budgets are unchanged.

The parent oracle received reward=1 with all six official tests passing in
154.29 seconds, including correctness/performance at 5000, 6000 and 8000 assets.
Parent nop received reward=0 with one baseline test passing and five expected
implementation failures in 27.17 seconds. All four children received oracle=1
and nop=0. The m4 oracle measured baseline=0.008099916998617118 seconds,
native=0.00052829199921689 seconds, speedup=15.332272702641673x.
Python 3.13.7, NumPy 2.3.2 and setuptools 78.1.1 were verified in the original
parent image. The unchanged frozen amd64 Agent and all declared sidecars passed
offline --help there as the ELF owner. A Python installer probe returned 403,
but the unchanged official curl installer succeeded with uv 0.9.5. No verifier
bootstrap or alternate checker was enabled. No paid model request was made;
these are preparation controls, not model evaluation scores.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/portfolio-optimization/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/portfolio-optimization/case.json --run-fixtures
```

Both passed (one unittest method covering 20 actual processes; 20 Linux controls).
validation.json binds every declared fixture/checker byte and the shared
diagnostic implementation. Raw commands, resources, timing, first scores, layer
verification and preserved infrastructure failures live under the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/portfolio-optimization/resume-20261030/`.
The ignored runtime.json one directory above references verified runtime-v2 and
retains the original parent timeout; the legacy suite runtime report does not
supply historical_root. Local preparation.json contains absolute execution and
evidence paths and should stay out of the asset commit. The model remains
token-plan/qwen3.8-flash, output 32000, input window 200000 and one paid attempt.

The final Cargo gate remains a separate repository regression requirement.
The first Clippy run lost a serde rmeta file and its original slot lock during
shared-cache removal. The retry dynamically claimed slot 4 with identical
CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR and jobs=1, then found the unrelated
`crates/vw-agent/src/session/processor/runner_tests.rs:2416` reference to missing
`super::llm_runner::TestLlmMessages` (E0425). No Rust file was changed for this
case. The task plan remains **completed, pending Clippy regression** until that
shared Rust error is corrected and the mandated all-targets/all-features Clippy
command passes. The 20+10 calibration receipts are valid under the identity above.
