# Original-package lifecycle verification

`t61/package` is a repository-authored MIT stdio MCP package, pinned by `t61/source.json`.
The registered `compat::t61_tests` use the production transactional installer,
discovery, enabled-state persistence, MCP connection/catalog/tool routing, and removal.
The server writes a caller-selected temporary output file; source and installed file
SHA-256 maps must remain identical. State and evidence are outside the package.
Malformed manifests fail installation; removal of one marketplace identity leaves
the other identity intact. Disabling removes the package from fresh MCP discovery.

This local fixture is not evidence for third-party package compatibility, public HTTP,
OAuth, actual desktop rendering, skills-only execution, hooks, catalog loading,
or upgrade lifecycle. Those remain necessary before task 61 can be accepted.
