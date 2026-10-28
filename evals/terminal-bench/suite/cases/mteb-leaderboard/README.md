# MTEB leaderboard diagnostics

Four independent synthetic exercises isolate reading a dated snapshot (m1),
filtering language and metric/task coverage (m2), competition ranking with ties
(m3), and canonical organization/model identifiers (m4). Each task specifies a
standalone stdin/stdout JSON program at /app/leaderboard.py, its public example,
write boundary, offline behavior and tool/time budgets.

The checker executes the submitted program on unseen synthetic inputs and uses
explicit expected outputs. It never imports the gold program or relies on a
self-report. It covers zero/null scores, incomplete languages/tasks, wrong metrics,
unsorted dates, cutoff ties, precision, duplicate IDs and case preservation.
Each child has gold, an empty submission and three semantic negative controls.
Only public fixtures enter materialized images; checker/oracle material is injected
at verification time. The digest-pinned Python base is the existing clean amd64
single manifest, independent of the upstream parent image.

Run unittest discovery for tests/test_cases.py and suite.diagnostics validate
--case evals/terminal-bench/suite/cases/mteb-leaderboard/case.json --run-fixtures
with PYTHONPATH=evals/terminal-bench and the pinned Harbor Python. validation.json
binds the declared bytes and checker to twenty fresh, offline fixture controls.
Standard Harbor oracle/nop controls use unchanged materialize output, resource
limits and original digest reference; they establish diagnostic calibration only.

The original parent passed fresh official oracle=1 and nop=0 controls, each
executing both upstream tests completely. Both parent and diagnostic images ran
the frozen x86_64 ELF --help offline with a read-only root and no credentials;
the parent also passed installed mteb/torch/transformers/Pillow import and SQLite
closure probes. The original parent instruction, solution, checker, image layers,
1CPU/8192MiB/10240MiB resources and 3600/3600s timeouts were retained.

The original amd64 parent manifest is
sha256:3eb9e900bf0a5c865eb0da0252c78eba2777cbd790794ff978d41d8982a04f09;
config sha256:0cdccdd4ec71822cd55179e77e0dca2be313be44d8ec046d94861e4147d5a4af.
Full compressed SHA and streamed diff_id checks measured exact U=2242184192
bytes within an 8GiB hard stop, then streaming OCI import verified config and
RootFS. Admission includes all compressed/unpacked copy allowances and separate
S10/B2/A1/safety5GiB budgets. Parent SizeRw is S; extra host archives are B;
receipts use actual allocated blocks in A. Both real controls stayed within limits.

preparation.json passed the production verify_preparation check for all ten
current controls. Current evidence and recovery scripts are in the ignored
results/qwen38-suite/diagnostics/mteb-leaderboard directory beneath evals/promptfoo.
Historical failed admission and control receipts remain preserved. The isolated
runtime config references the existing runtime-v2 snapshot; the public historical
runtime report was not edited. No paid model calls or parent environment variants
were used. Calibration is separate from future model evaluation results.
