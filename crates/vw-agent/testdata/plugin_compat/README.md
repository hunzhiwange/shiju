# Plugin compatibility fixtures

Fixtures contain hand-authored input and independent fixed expectations. They never
read production code to calculate expected answers. `t02/provenance.json` records
source symbols, source/license information and SHA-256 for the seed package and
checker inputs. The JSON evidence in `t02/complete.json` is **synthetic input for
testing the checker**, not evidence of compatibility on three platforms.

`t02/package/` is a minimal native VibeWindow package accepted by the existing
`load_plugin_manifest` entry point. It proves that the harness reaches the real
loader; Codex manifest behavior belongs to tasks 3 and 4. The Rust suite copies its
bytes into an isolated temporary workspace and stores evidence in sibling turn
directories. The package is never modified to make a compatibility test pass.

Run the registered Rust tests after acquiring a shared Cargo slot. This repository
also fixes `build.build-dir` in `.cargo/config.toml`; set `CARGO_BUILD_BUILD_DIR` to
`$CARGO_TARGET_DIR/build` so intermediate artifacts stay inside the same slot:

```sh
cargo test -p vw-agent --lib --all-features codex_plugin_compat_t02_ -- --nocapture
```

The checker has its own black-box subprocess suite, without Cargo:

```sh
python3 -m unittest discover -s scripts/plugin_compat -p test_check_evidence.py -v
python3 scripts/plugin_compat/check_evidence.py --baseline docs/plugin-compat/baseline.json --evidence docs/plugin-compat/evidence.json
```

The latter command must fail until the full frozen baseline has evidence. Task
records live under `docs/plugin-compat/evidence/tNN.json`; the final task produces
the aggregate `evidence.json`. No partial task record proves complete compatibility.

## Evidence schema, version 1

`baseline` contains `schema_version`, a `sources` map with SHA-256 values, and
unique `items` with `id`, positive integer `tasks`, and source/symbol/hash entries.
The frozen baseline uses `basis: excluded_execution` for the user-excluded hosted
connector row; it is excluded from acceptance, while its source remains required.
Other rows default to in scope. Explicit `in_scope` booleans are supported for
small independently authored baseline fixtures.

`evidence` contains `schema_version: 1` and `items`. Every in-scope baseline ID
requires exactly one evidence item with:

- `tasks`: all task numbers mapped by the baseline;
- `tests`: nonempty test names;
- `red` and `green`: `command`, integer `exit_code`, positive integer
  `effective_tests`, and nonempty `record` containing an output excerpt or log
  reference. Red must fail, green must pass, and commands must match;
- `platforms.verified`: all `required_platforms` from the baseline row or root
  (default macos, linux, windows). An unverified platform prevents full acceptance;
- `external_prerequisites`: an explicit array, empty only when no external
  prerequisite applies. Each entry has `name`, `status: verified`, `mode: real`,
  and `record`;
- `conclusion: verified`. Pending, blocked and unavailable are rejected.

Tasks 62 and 63 additionally require at least one real public-service prerequisite;
mock-only public integration cannot pass. Unknown or duplicate IDs, malformed
input, missing fields, zero tests and partial platforms fail with exit code 1.
Task 2's actual host and Cargo evidence is stored separately from these synthetic
checker inputs. Platform evidence outside the actual host remains future work.
