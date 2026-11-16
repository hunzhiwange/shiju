# Task 59: static cleanup implemented; whole-plan gate remains open

No Cargo, build, unit test, executable test or device command was run. No compilation, runtime coverage, or speed improvement is claimed.

## Implemented cleanup

51 pure wildcard forwarding modules were replaced with direct domain aliases in their existing parents. Existing namespace, visibility and surrounding cfg remain in place. The old source/parent/target mapping is task-59-aliases.json (task_worktree_pool uses the equivalent `pub use ::task_worktree_pool;`). No domain algorithm migration is counted for these aliases.

17 unregistered memory child files were removed; memory already exported agent_memory, whose public modules own the real implementation. See task-59-orphans.json. Knowledge chunker/fusion wrappers had no callers outside themselves; remove them, alias the crate-visible import module and export SqliteKnowledgeStore directly. This removes four additional forwarding files without moving algorithms or assertions.

Retain real project/tenant/config/plugin/resource capture and host error conversion. Per-file responsibilities are task-59-retained.json; symbol, export and cfg indexes are after-files.json. These indexes are navigational evidence, not blanket approval of every retained domain branch.

Necessary adjacent registration fixes: gateway/router.rs now explicitly points to the existing sibling router_tests.rs. The baseline scanner had already reported this missing default module. Restore original test attributes on 18 channel lifecycle functions in the host-mounted channel-runtime session_commands_tests.rs and nine project scope functions in host channels/session_tests.rs. They had declarations but no test attributes or callers. Assertion bodies are unchanged; no new mock or simulation was introduced. These 27 tests need native validation when authorized.

## Static validation

Commands (all from repository root):

- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_registrations.py`: 1173 lexical cfg-union edges, 0 unresolved, zero references to removed alias/orphan source targets. Baseline lexer reused; no macro expansion.
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_test_candidates.py`: 2465 baseline standard test declarations; counts {"retained_same_path": 865, "unique_name_candidate": 1575, "owner_evidence_candidate": 25, "ambiguous": 0, "missing": 0}. Exact-name/source-owner candidates provide a per-test navigation map. Owner evidence resolves 25 duplicate names (task28 destinations use task27's shared suite). This does not prove equivalent assertions, compiler registration or generated-test coverage. Includes pub(crate) declarations.
- `python3 .plans/2026-11-16-代理宿主完整减负/evidence/audit_final.py`: 1381 original paths, exact baseline/scope set equality and unique ownership. Current physical host: 67221 lines. Original baseline 135228, net physical reduction 68007. Limit 35000, remaining excess 32221. Residue 3317 lines.
- `git diff --check -- <task59 source and evidence paths>`: completed before commit; no whitespace errors.
- `rg -n 'vw-agent|vw_agent|vw-agent/src' crates/libs --glob Cargo.toml --glob '*.rs'`: broad audit candidates include unrelated vw-agent-apps names, tracing targets and resources. Structured literal dependency/path candidates are in remaining-host.json. Six CLI application composition crates depend on vw-agent; resource includes are not reverse Rust mounts. No literal Rust path/include mount to vw-agent was found by the lexical scan. Dynamic include expressions and macro expansion remain outside this static proof.

## Outstanding whole-plan gate and recovery ownership

The original task explicitly prohibits task59 from replacing unclosed domain migrations with a new monolith or rewriting other owners' domain implementations. Even deleting every residue line would leave 63904 physical host lines, above 35000. Finishing task59's forwarding cleanup alone therefore cannot close the whole-plan gate. This is a scope/acceptance dependency, not an external environment failure or a claim that migration is impossible.

remaining-host.json lists every retained original file, lexical symbols, line count and original owner. Original owner sessions must continue their domain splitting and test ownership work; task59 has not dispatched or rewritten those sessions. Complete production-call mapping and semantic old-test equivalence remain unverified. All 58 evidence files were read/indexed, but their statements are not runtime verification. No Cargo dependencies were removed without proof of disuse. Do not delete task59 or claim the whole plan complete until the hard gate and remaining mappings close.

| Owner | Remaining original host lines |
|---|---:|
| 59 | 3317 |
| 10 | 2925 |
| 11 | 2788 |
| 37 | 2705 |
| 24 | 2678 |
| 17 | 2351 |
| 53 | 2143 |
| 23 | 2118 |
| 50 | 2053 |
| 35 | 2001 |
| 12 | 1969 |
| 48 | 1762 |
| 15 | 1723 |
| 44 | 1667 |
| 38 | 1658 |
| 33 | 1650 |
| 54 | 1618 |
| 6 | 1507 |
| 43 | 1476 |
| 39 | 1466 |
| 32 | 1460 |
| 21 | 1321 |
| 16 | 1259 |
| 51 | 1247 |
| 18 | 1210 |
| 22 | 1123 |
| 42 | 1105 |
| 20 | 1082 |
| 14 | 1039 |
| 2 | 1023 |
| 19 | 1002 |
| 40 | 931 |
| 46 | 898 |
| 29 | 742 |
| 34 | 700 |
| 13 | 685 |
| 26 | 654 |
| 55 | 654 |
| 27 | 623 |
| 8 | 607 |
| 45 | 591 |
| 28 | 575 |
| 30 | 566 |
| 56 | 533 |
| 31 | 501 |
| 9 | 351 |
| 25 | 305 |
| 7 | 266 |
| 52 | 258 |
| 41 | 254 |
| 3 | 246 |
| 47 | 225 |
| 36 | 211 |
| 5 | 192 |
| 4 | 191 |
| 57 | 180 |
| 58 | 115 |
| 49 | 7 |

Future authorized entry: `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`; not run.
