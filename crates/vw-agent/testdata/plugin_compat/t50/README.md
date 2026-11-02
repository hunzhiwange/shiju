# Task 50 archives

Repository-authored fixtures under the repository license, generated with tar/flate2 in `compat/t50_tests.rs`. No third-party package, credentials or user files.

The package contains `package/package.json` (`@scope/pkg`, `1.2.3`, a lifecycle script that must never execute) and an unchanged `.codex-plugin/plugin.json` (`fixture`, `1.2.3`). The npm package identity and plugin manifest name are independent, matching the frozen Codex npm source behavior.

Variants: parent traversal, absolute paths, wrong archive root, symlink, hard link, FIFO, duplicate package.json, wrong npm name, modified bytes, oversized declared file. All must fail before installation publishes any entry. Positive installation publishes `fixture@catalog` plus provenance outside original package bytes. Limits: 50 MiB compressed, 250 MiB extracted files, 266 MiB decoded tar, 10,000 entries, 1 MiB package.json; no preserved setuid/setgid bits.

Reference: baseline marketplace.npm.archive/unpack; Codex npm_source::validate_npm_package_metadata and plugin_bundle_archive::unpack_plugin_bundle_tar. Additional restrictions: duplicate paths, Windows-style separators, special files and links rejected; decompression overhead bounded and gzip CRC checked. Extraction is private and never invokes npm or lifecycle scripts.
