# compile-compcert diagnostics

Four independent synthetic tasks, with fixed Qwen3.8-Flash 32000/200000,
24 turns / 32 tools, one attempt, fresh offline 1 CPU / 512 MiB containers,
900 s agent / 120 s verifier. No paid model calls are used for preparation.

- m1 selects the CompCert Linux target for four architecture spellings and
  rejects unsupported OS/architecture/arity via an executable shell API.
- m2 builds a native ELF compiler entry from an immutable synthetic C driver
  using a candidate Makefile. The evaluator compiles distinct actual programs,
  exercises arguments and space-containing paths, and rejects GCC substitution,
  object-only output, broken linking, invalid C and unsupported VLAs.
- m3 implements a digit-only uint32 rotation CLI. The evaluator compiles it
  with genuine CompCert and checks 35 boundary/value combinations plus invalid
  and out-of-range inputs. No keyword or self-test assertion is used.
- m4 installs a repeatable compiler entry at /app/install/bin/ccomp. PATH
  resolution and real compilation/execution are checked from another directory.
  Wrong prefix, broken compiler path and GCC substitution are rejected.

Public fixture bytes are hash-bound in evaluator-only metadata. Each child has
independent gold, nop and three meaningful mutations. No parent reference
prefix, hidden probes or previous model trajectory is used in synthetic inputs.

The diagnostic tool image contains Python, GCC/make/binutils/libc/GMP and the
public CompCert compiler/runtime. It contains no official tests/solution,
diagnostic oracle/gold or repository mount. The Dockerfile is a reconstruction
recipe using the ignored evidence directory as its context. Exported toolchain
bytes must come from the **new, normally completed and passing** original
parent oracle, not the invalid late artifacts from the first calibration.
The small compcert.ini is regenerated from the same public configure/Makefile
recipe and the passing parent configuration summary, including -no-pie. It was
not recovered from late artifacts. Missing-config and link-option setup checks
are retained in the ignored evidence. CompCert's upstream license is retained in the tool image; no compiler binaries
or public dependency archives are committed to the repository or pushed.

Validation and readiness evidence are separate: fixture controls alone do not
establish original parent readiness. The original timeout remains 2400 s;
the historical late reward=1/3 passing tests remain invalid infrastructure
calibration and are never counted. Final readiness uses the new normally completed parent controls and eight
fresh child controls. The initial unscored late-parent attempt is preserved
separately and contributes no valid calibration or model score.

Three unit tests passed, including twenty gold/nop/mutation controls and two
regression checks. Fixture validation independently passed all twenty controls.
The fresh original parent oracle completed in 2130.827 seconds with 3/3 official
pytest tests passing; nop failed all three tests as expected. All eight child
oracle/nop controls passed their expected reward checks. `verify_preparation`
accepted the complete ten-control receipt.

The architecture checker regression first failed because the wrong-OS control
was accepted, then passed after the missing OS probe was added. Initial tool
setup checks caught the omitted installation config; its reconstruction also
retains the original -no-pie linker flag. Initial fixture gold checks exposed
the sandbox's noexec /tmp mount. Evaluator-generated binaries now stay under
writable/executable /app; network, root filesystem and capability restrictions
were not relaxed. All initial failures remain in the ignored evidence root.

Run from the repository root:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/compile-compcert/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/compile-compcert/case.json --run-fixtures
```

This run resumed preflight with the same inputs/materialize/harbor_trial and
verify_preparation primitives, reusing only the two newly completed original
parent controls. It did not redundantly rerun the 35-minute parent after its
successful calibration. The task-local resume script and private scheduler
runtime JSON are in the ignored diagnostics/compile-compcert evidence root;
the historical root runtime.json is an evidence document, not scheduler config.
The complete receipt binds unchanged source, checker, fixture, image, adapter,
runtime and actual scoring bytes. No shared scheduler or baseline was edited.

`evals/promptfoo/score.sh test` and the required Clippy command exited 0.
Clippy dynamically used slot 1, target/build directories at the repository's
absolute target/rust-slot-1 path, and one build job; the owned lock was released.
No Rust behavior tests or model calls ran. Raw commands, exit codes, timestamps,
image/dependency/compiler/checker hashes, isolation/resource records and the
invalid first attempt are indexed by the ignored acceptance-2026-10-28.json.
