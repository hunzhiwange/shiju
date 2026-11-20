# Pi ACP managed runtime patch

Upstream: https://github.com/svkozak/pi-acp commit
`8be5f22769f71d759aa48d798416c56e8be101ef` (v0.0.22, MIT).
The patch adds absolute `PI_ACP_STATE_DIR` to `getPiAcpDir()` and an owned
`SHIJU_PI_SKILL_ROOT` directory contract shared by the ACP catalog and native RPC spawn.
An unset skill root preserves upstream discovery. Managed launches keep
`--no-skills --no-extensions`. The v3 session-model launch uses Pi's pinned
resolver to add enabled files from the project's `.pi/skills`, then adds the
explicit ShiJu session root when present. Native files take precedence on name
collisions. The ACP startup catalog uses the resulting native RPC inventory.
CLI-local launches without a bound relay retain the explicit-view behavior.
An unset `PI_ACP_STATE_DIR` preserves `homedir()/.pi/pi-acp`, including cli_local;
relative or empty state-directory overrides fail explicitly. Pi itself is unchanged.

The v2 managed runtime connects ACP session mounts to Pi's RPC tool registry.
It loads only the explicit reviewed extension for that session. Each call is
bound to its in-progress Pi tool call and requires an ACP `allow_once` decision.
Discovery and execution reuse the reviewed Cortex stdio sandbox and its
single-operation runner. This path supports absolute macOS stdio commands with
no environment overrides. The gateway passes the resolved tool allow and deny
rules, and the runner gets a private HOME and a workspace read-only view.
Loading a session reconstructs the tool set from that load request.

Apply and build in a private checkout of that exact commit:

```sh
git apply --unidiff-zero /absolute/path/pi-acp-0.0.22-state-dir.patch
npm ci --ignore-scripts
npm run build
```

Pinned upstream package-lock SHA256:
`e257ee4cbe345ee0de44714d34401ed2ed14de41abc2c6ed7511b1a3d923c606`.
Patch SHA256:
`e860e7597343eef5c47af62cac90e4ce1e036efcb4968a4ea60786b03a834ad0`.
Built dist/index.js SHA256 (Darwin arm64):
`625fb0af3d7170a000c9ce763355091c64ce67ad6be00abd548e49f1dcf05001`.
The session_model driver checks the adapter and installed companion file hashes against the managed registry before launch;
launch the executable dist/index.js directly (no npx wrapper or extra arguments).
The native engine used for acceptance is the real published
`@mariozechner/pi-coding-agent@0.73.1`, gitHead
`781152fc24841dc54b22284514604048ebe5e2c9`; no second agent implementation is maintained.

Version and model-request acceptance are separate: pi-acp 0.0.22 has no --version
output. Verify package identity, patch/build digest and native Pi version rather
than adding a fake version-reporting CLI. The managed installer preserves both package pins.

Verify the patch's default/absolute/invalid-directory behavior without reading or
writing user files:

```sh
node --import tsx /absolute/path/verify-state-dir.mjs "$PWD"
```

Native Pi `dist/cli.js` SHA256 is additionally checked by the driver:
`e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa`.

Published npm inputs (registry.npmjs.org, npm verified integrity on installation):

- `pi-acp/-/pi-acp-0.0.22.tgz`:
  `sha512-A9DFxi3dF/xFgf0vm5r5jvEtTszRXqK2MUYXWQN0aSSLeJW53efqFlueczZCj5J119s+FLlQVQYso71PDPL0sw==`.
- `@mariozechner/pi-coding-agent/-/pi-coding-agent-0.73.1.tgz`:
  `sha512-gXQh3SaZmWTfVMc4Ao5+LGbVeKvzyO7tolok0nLsZgq9nGjZx/EEU3NM8C+qUnB4Nvs2rswG5qOVgLzQkq0fHQ==`.

Apply the patch against the exact pinned commit; the original state-directory
isolation remains intact. Skills are read by the unchanged native engine, on demand.

With the managed v2 installer output, exercise discovery, host approval, same-ID
tool results, cancellation, and removal in new and loaded sessions:

```sh
python3 patches/acp-model-source/pi/probe.py <pi-acp-entry> <rustcodegraph> <indexed-ruejs-copy>
```
