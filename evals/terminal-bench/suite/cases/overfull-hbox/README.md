# Overfull hbox diagnostics

Four standalone Python JSON stdin/stdout programs isolate real TeX behavior:

- m1: reproduce warning counts and distinguish clean, overfull and invalid TeX.
- m2: shorten synthetic prose using only supplied synonym families.
- m3: preserve all other bytes, including whitespace, punctuation, commands,
  repeated occurrences and protected surrounding text.
- m4: compile clean, overfull and invalid documents in fresh directories,
  report actual warning counts and PDF presence in order without stale reuse.

Only synthetic public examples enter `/app`. The checker, hidden probes,
reference programs and three faulty programs per child remain verifier-only.
The checker independently runs pdflatex with shell escape disabled, using fresh
exit codes, log warning counts and PDF headers. Candidate reports are compared
with these independent compilations. Missing pdflatex exits 2, including for
nop, and never counts as an intentional rejection. Neither keywords nor Agent
self-tests certify behavior.

The tools-only image is a real amd64 single OCI manifest:
`vw-overfull-hbox-tools:task65@sha256:44fee1e96bc7a9464263914021f9936b2a138c967e71bc92203de9377bb82ec9`,
config `sha256:063b871f9e2d4a0af2d61d10a967bfa8d2fc60b386b54860f9d98dfb36a37431`.
It extends the unchanged cached task52 Python/Debian12 tools base. Signed APT
metadata resolved the no-recommends dependency closure: 32 packages, exactly
44,891,344 archive bytes, including texlive-latex-base `2022.20230122-3`.
`environment/dependencies.json` records HTTPS URLs, exact versions, archive
sizes, SHA256 and measured decompressed tar bytes. Reconstruct the build
context by fetching only those files into `environment/packages/`, enforcing
sizes and SHA256 before building the Dockerfile. Archives remain ignored.
Installation uses local verified packages, no network and policy-rc.d exit 101
to prevent service startup. No parent task, solution or tests enter the image.

The tool admission retained the existing diagnostic S=1GiB, B=2GiB, A=1GiB and
5GiB margin. Three compressed-copy reservations of 768MiB each and a positive
complete U hard cap of 3GiB require 15,300,820,992 bytes. The smaller host/VM
free-space snapshot was 18,282,835,968 bytes. Downloads enforced exact byte and
hash limits; ar/xz/gzip streams bounded dependency expansion. The classic
builder used the public build command and shared resource lock. The actual
RUN had CPU quota/period 100000/100000 and memory=swap=536870912 from creation;
its SizeRw plus base/all package layers was monitored against complete Ucap.
No later docker update supplied limits. Actual final streamed save payload is
451,941,335 bytes and U including repeated RootFS references is 1,112,603,136
bytes. All layer diff IDs, manifest/config hashes and pinned-ref identities
match. No full save/export archive copy was written. A first identity guard
mistakenly equated OCI manifest Id with config digest and exited 1; corrected
checks validate the distinct manifest/config objects and full RootFS instead.

The x86_64 frozen Agent plus existing runtime libraries passed `shiju --help`
in this read-only offline image. pdflatex version, its linked libraries, Python
and an empty `/app` with absent `/tests` and `/solution` were also checked.
No DOCKER_DEFAULT_PLATFORM, private --platform or Trial/Compose override was
used. Standard materialization keeps the original FROM/ref bytes.

TDD first exposed punctuation rejection and whitespace-loss acceptance. A real
TeX checker mutation ignoring warning counts then wrongly accepted m1/m4's
ignores-warning programs, producing two red assertions. Restoring the checker
passed all 30 tests with actual pdflatex and no skips. Fixture validation
passed all 20 fresh containers: four gold, four nop and twelve negatives.
`validation.json` binds those declared fixture/checker bytes and image.
The initial host-only run (10 pass, 20 skipped) remains historical evidence;
it is not substituted for real compilation.

Eight standard fresh Harbor oracle/nop controls passed directly through public
`harbor_trial` and its repaired SuiteDockerEnvironment backend. Resource,
first-score, task/adapter/runtime/source identity receipts are retained in the
ignored evidence directory. All four oracle rewards were 1 and all four nop rewards were 0, using the first
score without recovery. Completion is recorded in the numbered task checkpoint. They do not run a paid model or certify the original parent.
No private Trial wrapper or artifact recovery is used.

The official parent was admitted and imported on 2026-10-30 without changing
its source or image. Manifest:
`sha256:7dca952bb6736194a736b5110945f48842eafa8551e24468f1dddeed0daee799`;
config: `sha256:492bea53bb36318ba79f670c0449d388229bed86eb82f12845ad8e1ebe93a83e`.
All six compressed hashes and decompressed diff IDs match. C=131,843,285 bytes;
complete U=320,532,992 bytes. The initial host/VM minimum was 43,424,010,240
bytes, above the conservative 24,017,849,983-byte reservation (three C copies,
two positive 2GiB U caps, original S10/B2/A1 and 5GiB margin). OCI import was
streamed without a disk archive; the official tag resolves to that exact digest.

The real APT dependency probe exceeded 180 seconds. The verifier-only
`environment/verifier-bootstrap.sh` supplies hash-checked cached uv 0.9.5,
Python 3.13.7, genuine Linux curl and the exact official pytest/CTRF packages.
A first offline closure probe exposed missing SQLite libraries. Its complete
Python dynamic-library closure was extracted from the pinned tools image,
verified by file hashes, and passed an offline parent-image probe including
ssl/sqlite/bz2/lzma, authentic uv installation, pytest/CTRF and real pdflatex.
The cache enters only after the agent exits. Real apt/apt-get are bounded,
return their original error/timeout status, and never synthesize a grade.
The official test script, checker, oracle, resources and timeouts stay unchanged.

Current acceptance evidence is under
`evals/promptfoo/results/qwen38-suite/diagnostics/overfull-hbox/continuation-20261030T140215Z/`.
`preparation.json` is complete and passes the public `verify_preparation` against
current inputs: two original parent and eight diagnostic standard Harbor controls.
Each oracle first score is 1 and each nop first score is 0. The parent oracle
passed all four unmodified official tests. No recovered score or artifact grading
replay was used, and no paid model was called.

The current execution identity is
`94dec6f0c90690b5c0ea348c8142a22b0cec1d62fa174df762f6dc93cb7653df`.
All controls used `harbor_trial` and `SuiteDockerEnvironment`; the official source,
image, 2CPU/4096MiB, 10GiB storage and 750/360-second limits are unchanged.
Children retained 1CPU/512MiB and their egress sidecars 0.25CPU/64MiB. Raw live
inspect receipts for all ten accepted controls were checked. Each phase admitted
against current min(host,VM), retaining S/B/A/5GiB and other active task reserves.
All owned control containers and locks were released.

Resource-lock contention and disappearing-container snapshots produced separate
pre-agent infrastructure attempts. Two controller samplers also failed after
valid standard results: m2 nop on a disappearing container, and the parent oracle
on containerd snapshotter.Usage while APT rotated pyc files. Their raw errors,
resource samples and genuine Harbor result/first-score files are preserved. Those
real results were collected without rerunning or changing their scores; later
samplers resample transient missing snapshots rather than treating unknown sizes
as zero. `final-acceptance.json` records the hashes and this monitor limitation.

Current validation passed 30 real TeX tests without skips, 20 fresh fixture
controls (4 gold, 4 nop, 12 negatives), eight fresh standard child controls,
the frozen ELF help probe in the pinned tools image, and fixed Promptfoo 0.122.0
configuration validation. `cargo clippy --all-targets --all-features -- -D warnings`
exited 0 using dynamic slot 2,
`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-2`,
matching `CARGO_BUILD_BUILD_DIR` and `CARGO_BUILD_JOBS=1`. No Rust source changed
for this case and no full unit-test, fmt-check, wasm or Makefile run was used.
