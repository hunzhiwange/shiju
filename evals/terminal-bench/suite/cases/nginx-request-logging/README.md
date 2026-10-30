# Nginx request logging diagnostics

Four independent synthetic exercises use real Nginx and fresh local HTTP probes:

| Child | Contract | Negative controls |
| --- | --- | --- |
| m1 | Generate a static server config; serve nested and updated bytes | wrong root, wrong port, missing path returns success |
| m2 | Append exact timestamp/method/status/quoted user-agent records | omitted method, unquoted agent, hardcoded status |
| m3 | Custom internal 404 page and append-only access/error log paths | wrong 404 body, public error page, wrong access log |
| m4 | Send exactly two GET requests with supplied agent, tolerate expected 404 | one request, wrong agent, wrong second path |

`configure.py ROOT PORT` writes a config; the verifier runs `nginx -t`, starts
Nginx, probes HTTP and observes logs after graceful shutdown. All temporary,
pid and log paths must stay under ROOT because the container rootfs is read-only.
`request.py BASE_URL USER_AGENT` interacts with a verifier-owned Nginx server;
actual server access records determine the request sequence. Printed assertions,
self-tests, keywords and prerecorded artifacts do not count. No parent reference
solution, original input or upstream tests are included in the child images.

The tools-only image is amd64, single OCI manifest
`sha256:ef27f9fbbcb46c21f223adc5915ab60c676351bdafe3896814155568f0497b3a`,
config `sha256:a47817ba8a50e0641719b0063c58f5869463be3943f07accaab2c60723902155`.
It extends the fixed clean task52 Python/Debian12 tools image. Official signed
APT metadata resolved eight packages totaling 1,943,416 bytes, including Nginx
`1.22.1-9+deb12u10`. `environment/dependencies.json` records each HTTPS source,
exact size, version and SHA256. To reconstruct the build context, fetch only
those files into `environment/packages/`, enforce the recorded byte limits and
verify every SHA256, then build `environment/Dockerfile`. Archives are deliberately
kept in the ignored evidence directory rather than committed. Installation
uses local verified packages and policy-rc.d exit 101 to prevent service startup;
no remote script or benchmark material enters the image.

The actual classic builder RUN was captured with CPU period/quota 100000/100000
and memory=swap=536870912 from creation. No later update was used. The build used
network none because dependencies had already been resolved and verified; this
does not change any original parent network semantics. Production child packages
retain standard materialization's exact FROM/ref, with no platform/Trial/Compose
wrapper. The new shared SuiteDockerEnvironment backend owns standard build and
runtime limits. The frozen x86_64 Agent plus its existing runtime sidecars passed
`shiju --help` in the read-only, offline tools image; nginx -V, ldd and Python
also passed, and /app was empty with no /solution or /tests.

The existing child profile requests 1CPU, 512MiB and 1GiB storage. Tool admission
retained 2GiB cache, 1GiB evidence and 5GiB margin, with three compressed-copy
reservations each capped at 512MiB and complete U capped at 2GiB: 12.5GiB total.
Host/VM snapshots used their smaller free-space value, about 18.68GB. All base
layer digests were matched; the final compressed layer total is 335,161,124 bytes
and U including repeated RootFS layer references is 920,099,840 bytes. Streaming
save/export inspection enforced read/decompression caps without writing full
archives. Metadata used bounded tmpfs; downloads enforced exact sizes/hashes.
Build context, downloaded packages, installed layers and cache copies were
included in the reservations. Inspect Size was not treated as U.

TDD first exposed Nginx's default temporary path on the read-only filesystem.
All five temporary paths are now explicit under ROOT. A separate checker mutation
omitting status comparison produced a red test: wrong-status was accepted.
Restoring complete field comparison passed all 20 unittest controls. Fixture
validation passed 20 fresh containers: each child gold/nop/three negatives.
`validation.json` binds the fixture/checker bytes. Eight fresh standard Harbor
controls also passed through the new shared backend, all four oracle rewards 1
and all four nop rewards 0; first scores were used without recovery or replay.
The task/adapter/runtime identities and actual main/sidecar limits are archived
in children-calibration-production-backend.json and production-*-resources.json. Prior missing-Nginx failures,
the overly narrow OCI identity guard and all other exploratory logs remain
separate in the ignored evidence directory; they are not successful controls.

The original parent capacity blocker was resolved on 2026-10-30 after measuring
44,527,923,200 host bytes and 74,117,140,480 VM bytes free. Admission retained
unchanged S=10GiB, B=2GiB, A=1GiB and margin=5GiB, plus two compressed copies and
a positive 1GiB decompression cap: 20,535,650,052 bytes. The original registry
manifest is `sha256:693e64431a0dbf45952e8b91a298d54fd60247e501c6623fb8fa3ebc07fc3d3d`,
config `sha256:41217bae6667f04a767dc1b2c5c12b034dfc51a18226202003313aaf86832055`,
linux/amd64. All seven compressed digests and diff_ids were verified; C is
67,277,698 bytes and complete U is 164,346,880 bytes. Two missing layers were
fetched with exact size/hash limits. OCI import streamed verified blobs without
creating a full archive; imported manifest, config and layer chain matched.
No other task, business container or cache was removed.

Fresh official parent controls used the unchanged upstream task and common
`suite.scoring.harbor_trial` / `SuiteDockerEnvironment` backend. Oracle passed
all eight official pytest tests with first reward 1; nop failed all eight with
first reward 0. Neither used verifier recovery, replay or a dependency patch.
APT, uv 0.9.5 and all three PyPI indexes returned 200 inside the original image.
The frozen runtime-v2 ELF and sidecars passed offline/read-only `--help` there.
Parent agent/verifier limits remain 900 seconds and main limits 1 CPU/2048MiB.
The earlier missing `historical_root` error remains in historical logs; current
controls use the ignored private execution config referencing runtime-v2.

On 2026-10-30, the focused unittest and fixture validation each passed 20 fresh
controls. Ten new standard Harbor controls also produced the expected first
rewards (parent plus four children, oracle=1/nop=0), without recovery or replay.
These ran under execution identity
`7faf6e3471f51146cfdcf0d6ce62a6d63df5deaec17fb3866e6e723de9afd6c5`.
The final production identity guard correctly rejected preparation because an
unrelated concurrent edit to `suite/variants.py` changed the scheduler identity
to `f5218f...`, then `7ad9af...` during the next admission wait. No hashes or old
results were rewritten, and no current `preparation.json` was published.

Status: **implementation complete, awaiting regression with stable shared code**.
The second controller started no control before being stopped: other preparation
topics occupied both slots. Resume by obtaining a stable shared execution window,
remeasuring min(host, VM) capacity and active topics, and running all ten fresh
standard controls followed by `verify_preparation`. Do not treat the historical
ten results as current readiness or launch a paid model from them.

Clippy passed using dynamically acquired slot 1, matching absolute target/build
path `/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1` and one job;
its owned lock was released. Promptfoo 0.122.0 configuration validation also
passed using a task-local npm cache and the official registry, after the existing
shared cache lacked `cookie`. Initial network waits and direct-cache failures
remain separate. The temporary npm cache grew to 3,089,555,456 bytes, exceeding
the earlier 2GiB B reservation; further calibration was stopped and that owned,
reconstructible cache was removed. Its exact path, package hash, successful
validation log and before/after space are in `npm-cleanup-receipt.json`. Future
admission must budget tool cache growth or use a verified complete installation.
No Rust source or shared evaluation file was changed by this task.

Evidence root: `evals/promptfoo/results/qwen38-suite/diagnostics/nginx-request-logging/`.
The `resume-20261030/` directory retains original registry/config/layer identities,
capacity and runtime probes, all ten control logs/resources/scores, two 20-control
fixture checks, Clippy and Promptfoo logs, and the identity rejection checkpoint.
`resume-20261030-current/` preserves the unexecuted admission wait. The original
source, scoring, model/token policy and parent resources/timeouts remain unchanged.
