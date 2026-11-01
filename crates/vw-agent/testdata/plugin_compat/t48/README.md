Fixtures are generated in temporary local git2 repositories by t48_tests.rs.
Authored for this repository under its license; no external payload or credentials.
Expected: pinned commit yields unchanged plugin.json, no .git, external provenance.
Reference: frozen core-plugins/marketplace_add/source.rs and git_policy.rs;
marketplace.git.url/path/ref/sha in docs/plugin-compat/baseline.json.
Security difference: explicit revision required; symlinks and submodules rejected.
