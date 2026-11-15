# Task 8 transaction fixtures

These synthetic fixtures are MIT-licensed VibeWindow test data. They contain no
upstream package bytes, credentials or user content. Tests materialize real
directories and exercise the public local/marketplace installation entry points.

| Fixture / behavior | Expected result | Reference |
| --- | --- | --- |
| `plugin.json`: name, version, description | Preserve the original legacy manifest bytes and exact identity | Frozen `store.rs::replace_plugin_root_atomically` and `PluginId::as_key`; host legacy parser |
| `payload.txt` | Copy original bytes without executing package commands | Task 8 host original-package policy |
| Post-copy checkpoint | Final destination stays absent until static audit and metadata preparation succeed | `store.rs::replace_plugin_root_atomically` staging and manifest-byte audit |
| Corrupted staged manifest | Reject and remove owned temporary state; original source is untouched | `store.rs::replace_plugin_root_atomically` manifest audit |
| Competing destination | Never replace an existing directory, including an empty one | Task 8 stricter no-overwrite policy |
| Existing metadata and concurrent installers | Preserve prior host state; publish at most one complete package for an identity | Host `identity.rs` package-external state and Task 8 transaction boundary |
| Changed manifest bytes or entry point | Reject even if name and version still match | Frozen `store.rs::replace_plugin_root_atomically` selected-path and byte audit |
| Skills, MCP, Hooks, Apps, onboarding and interface paths | Returned audited paths refer to the final package and match a fresh real load | Existing host normalized manifest and Task 8 publication |
| Nested destination or source symlink entry | Reject without recursive copying, source mutation or out-of-package writes | Host secure-copy policy |
| Executable `setup.sh` variation | Preserve file bytes without executing it | Task 8 install-does-not-execute policy |
| Existing local name `.codex-plugin` plus unrelated root JSON | Staging never appears as an installed or rejected plugin during real discovery | Host native-name compatibility, discovery entry points and Task 8 isolation |

The checkpoint only pauses real filesystem work; it does not substitute copy,
manifest parsing, identity validation or publication. Reference source hashes
are recorded in task evidence. `artifact_operation.rs` was reviewed but concerns
hosted artifact attribution, which is excluded here, not package installation.
No non-macOS runtime acceptance is claimed by this task.

Inline test variations only use synthetic JSON, file paths and harmless shell
content. The no-replace publisher uses the existing libc dependency on macOS
and Linux, and directory rename on Windows. Unsupported filesystems/platforms
fail explicitly; no overwrite fallback is permitted. Package updates and
persistent install-state recovery remain separate tasks (51 and 52).
