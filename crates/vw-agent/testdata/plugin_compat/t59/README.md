# Host execution fixture

Reference: codex-rs/core/src/tools/handlers/unified_exec.rs, ExecCommandArgs and ExecCommandWorkdirArgs in the local read-only Codex snapshot (Apache-2.0). Only field names and behavioral observations are used; no source copied.

Public skill reference: codex-rs/skills/src/assets/samples/skill-installer/SKILL.md, script invocations at lines 36–41. Tests use a local harmless printf/sleep command instead of its network installer.

Supported mapping: cmd -> command, workdir unchanged, timeout_ms -> timeout, yield_time_ms unchanged. Host environment filtering, workspace checks, budgets, approval and session ownership remain authoritative. Poll and cancel use existing bash_wait/bash_cancel handles. No package-supplied approval is accepted.

Unsupported modes fail before process creation: custom shells, login shells, PTY, custom output token budgets, additional permission profiles and stdin. These are visible compatibility limits, not silently ignored parameters. This fixture does not establish complete Codex compatibility.

Reference SHA-256:
- unified_exec.rs: `0e4917e6be975e0127b33f1bc8bfab76ee05a44714c0a318adfa0e8061885ccc`
- skill-installer/SKILL.md: `d68b77e5bbb34dedab89d134da52855f140fc4b4299b80104f534e3b9e98f8ee`
