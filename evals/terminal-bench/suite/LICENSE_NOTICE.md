# Terminal-Bench 2.1 source and license notice

Source: [harbor-framework/terminal-bench-2-1](https://github.com/harbor-framework/terminal-bench-2-1/tree/7131e4375048a0e408a8fb404b5f499d726b695b), pinned revision `7131e4375048a0e408a8fb404b5f499d726b695b`.

The upstream repository is distributed under [Apache License 2.0](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/LICENSE). The complete original LICENSE and all task files remain byte-for-byte in the ignored source checkout at `evals/promptfoo/results/qwen38-suite/source/`. Preserve that license and each resource's original copyright, attribution, license and third-party notices when redistributing any source or assets; the repository license does not override third-party terms.

Only file names, hashes and public task configuration are committed here. Instructions, reference solutions, tests and input data are not copied into this package. Canary provenance markers remain in the original files; effective instruction hashes use Harbor 0.24.0 `strip_canary`, matching the instruction delivered by Harbor. Do not submit hidden task materials as model context.

`source.json` records the upstream license hash and Git trees. `catalog.json` records each original task file's SHA-256, Git blob and mode. Regenerate using the three required CLI arguments:

```sh
PYTHONPATH=evals/terminal-bench evals/promptfoo/results/task25/harbor-venv/bin/python -m suite.catalog --source evals/promptfoo/results/qwen38-suite/source --revision 7131e4375048a0e408a8fb404b5f499d726b695b --output evals/terminal-bench/suite/catalog.json
```

The CLI rejects missing, extra or changed task files before writing output. External URLs and container images are recorded as upstream references; hashing the source tree does not claim those external artifacts have been downloaded or are immutable.
