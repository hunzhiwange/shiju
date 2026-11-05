# Copilot 1.0.105 ACP skill directory binding

The fixed Darwin-arm64 SEA has SHA-256 `5fd7d15ff02deb08a58593b1b488015bb52c1686014b80b5509e28263cb3ad70` and embeds the JavaScript CLI and native runtime. The installer verifies the npm archive and SEA, extracts with the vendor's `COPILOT_SEA_EXTRACT_ONLY` entry into its own staging directory, and patches only `cli-main.js` with exact before/after hashes. Global npm installations and user caches are untouched. Original license notices remain in the installation.

The ACP boot-option builder now forwards `SHIJU_COPILOT_SKILL_ROOT` as the sole `skillDirectories` entry with `enableConfigDiscovery=false`, `includedBuiltinSkills=[]` and no plugin discovery. An empty root disables skills. The native SDK implements discovery and the native `skill`, `view` and shell tools; this patch does not implement a replacement loader or expand bodies into the first prompt. Existing model, MCP, sandbox, permission and authentication options remain in the ACP boot object.

The gateway validates the session view and all reviewed entry/runtime hashes before setting this environment. It clears the ambient `COPILOT_SKILLS_DIRS` list. Directory updates rebuild at idle through the existing lifecycle; no hot-update support is claimed. Resume gets fresh ACP boot options. The npm `.bin/copilot` in the managed installation points to the extracted entry, so the SEA cannot silently choose a downloaded version or a different cached bundle.

Run the task-specific verification commands with the managed installation's `node_modules/.bin` first in PATH and a dynamically acquired Cargo slot. Only `docs/vibewindow/acp-skill-mounts/evidence/copilot.json` records the final acceptance verdict; discovery-only probes are not acceptance.
