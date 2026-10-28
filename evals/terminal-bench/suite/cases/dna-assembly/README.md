# dna-assembly diagnostics

Four independent, synthetic sequence tasks; no parent sequence, solution,
hidden test or model trajectory is copied into their public fixtures.
No model requests are made during preparation.

| Child | Artifact checked | Negative controls |
| --- | --- | --- |
| m1 | Half-open selection, uppercase and reverse complement, in request order | unoriented, end off by one, reordered |
| m2 | Linear fragment order and overlap-trimmed merged sequence | wrong order, duplicated overlap, missing fragment |
| m3 | Complete boolean junction compatibility results | identity instead of reverse complement, complement without reverse, missing junction |
| m4 | Exact two-line FASTA plus length, GC count, SHA256 and fragment order | wrong length, newline included in hash, corrupted FASTA with correct metadata |

The independent checker reads real output files and compares complete typed
structures against frozen synthetic expected artifacts. It rejects input
modification, missing outputs, malformed or duplicate JSON keys, symlinks,
boolean integers and unexpected keys. Public instructions state coordinate,
orientation, overlap and hash conventions. These are narrow diagnostic skills,
not substitutes for primer design or official melting-temperature scoring.

The digest-pinned Python image needs no additional packages. Materialization
copies only each child's declared fixtures into its environment; gold and
oracle files remain hidden until solution/verifier phases. Fixture calibration
uses fresh offline containers, 1 CPU/512MiB, read-only root filesystem,
128-PID limit, all capabilities dropped, no-new-privileges, and no repository
or parent-task mounts. Harbor calibrations use fresh tasks with 900-second
agent and 120-second verifier limits. The official parent remains unchanged:
1 CPU, 2048MiB memory, 10240MiB declared storage, 1800-second agent/verifier
limits. Docker does not enforce the writable-layer storage quota.

Raw commands, failures, digest/config/layer identities, dependency checks,
resources and calibration receipts are archived in the ignored
`evals/promptfoo/results/qwen38-suite/diagnostics/dna-assembly/` and
`evals/promptfoo/results/qwen38-suite/preflight/` directories.
`validation.json` binds every declared fixture/checker byte and the image;
it does not alone establish full parent readiness.

Verification:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m unittest discover -s evals/terminal-bench/suite/cases/dna-assembly/tests -p test_cases.py
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.diagnostics validate --case evals/terminal-bench/suite/cases/dna-assembly/case.json --run-fixtures
```

The historical runtime.json is an identity receipt without historical_root.
The per-case ignored runtime.json selects the verified existing generation-2
runtime and unchanged source/case roots, preserving the parent timeout.

Acceptance on 2026-10-28: 25 focused unit tests, 20 fresh offline fixture
controls, and all ten fresh Harbor controls passed. The untouched parent and
each child scored oracle=1 and nop=0. Parent oracle pytest collected and passed
one test; nop failed that same test because primers.fasta was absent.
`verify_preparation` accepted the complete source/runtime/scoring receipt.
`evals/promptfoo/score.sh test` exited 0. The initial permissive checker had
failed 24 rejection assertions before correction, and a separate calculation
confirmed all four frozen synthetic gold artifacts.

The parent image is the unchanged linux/amd64 manifest
`sha256:d1adf6835f1dd91205ba70e452c699d0aea601010038e5617f370716efb50569`
(compressed layers 29724813 bytes, uncompressed 80646144 bytes). Manifest,
config, compressed layer hashes and uncompressed diff_ids were checked before
OCI import; local RootFS matches the original config. Actual parent limits
were memory.max=2147483648 and cpu.max=100000/100000. Official installation
provided EMBOSS 6.6.0+dfsg-12ubuntu2, primer3 2.6.1-4 and curl 8.5.0-2ubuntu10.16.
No dependency was preinstalled into the Agent environment, and no verifier
repair, environment variant, timeout extension, model call or budget change
was needed for the passing run. Old timeout and interrupted-capacity attempts
remain archived separately and are excluded from the successful receipt.

The network recovery audit downloaded the original 61018452-byte emboss-data
package in 7.24 seconds inside a throwaway container; those bytes never entered
the calibration container. Before the passing fresh run, host free space was
71439282176 bytes, exceeding the 18GiB conservative admission requirement;
Docker VM free space also exceeded it. Neither official storage nor the
mandatory free-space margin was reduced.

Passing preflight (exit 0):

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.run preflight --catalog evals/terminal-bench/suite/catalog.json --runtime evals/promptfoo/results/qwen38-suite/diagnostics/dna-assembly/runtime.json --parent-id dna-assembly --output evals/promptfoo/results/qwen38-suite/preflight/dna-assembly-recovery-20261028-02
```

Exact commands, hashes, resource/isolation evidence, historical failures and
final checks are recorded in the ignored diagnostics directory; the current
acceptance record is recovery-final-20261028.json. Include these five
independently scored cases in the later suite freeze.

Required `cargo clippy --all-targets --all-features -- -D warnings` exited 0
on 2026-10-28. It dynamically claimed slot 1; CARGO_TARGET_DIR and
CARGO_BUILD_BUILD_DIR were both
`/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`, with
CARGO_BUILD_JOBS=1. The owned lock was released and the cache retained.
No Rust source changes or Rust behavior tests were needed for this task.
