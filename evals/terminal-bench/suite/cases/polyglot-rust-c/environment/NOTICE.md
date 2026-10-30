Diagnostic tools only. The frozen original parent environment is unchanged.

The base is the existing generic Debian/Python/GCC tool image pinned in
`toolchain.Dockerfile`. Rust 1.75.0 compiler and standard library distributions
come directly from static.rust-lang.org; URLs, compressed lengths and SHA-256
are frozen in `sources.json`. Rust is distributed under MIT OR Apache-2.0;
its distribution COPYRIGHT and LICENSE files, including third-party notices,
are retained under `/opt/rust75/distribution-licenses/` in the image.

To rebuild, obtain the two exact archives from sources.json into an isolated
context alongside install.py and toolchain.Dockerfile after capacity admission.
Use the repository's shared build admission and limits before RUN, e.g. the
recorded classic builder with CPU period/quota 100000/100000 and memory/swap
512MiB. Build with `DOCKER_BUILDKIT=0 docker build --cpu-period 100000
--cpu-quota 100000 --memory 512m --memory-swap 512m -f toolchain.Dockerfile .`.
Do not pass platform overrides. Pin the resulting actual single amd64 manifest,
then rerun fixtures and standard Harbor calibration before accepting a new digest.
The installer copies only distribution compiler/std components, uses no Cargo,
retains licenses, and adds no Agent, benchmark, solution or test payload.
