# Pi ACP state directory patch

Upstream: https://github.com/svkozak/pi-acp commit
`8be5f22769f71d759aa48d798416c56e8be101ef` (v0.0.22, MIT).
The patch adds absolute `PI_ACP_STATE_DIR` to `getPiAcpDir()` and an owned
`SHIJU_PI_SKILL_ROOT` directory contract shared by the ACP catalog and native RPC spawn.
An empty skill root disables skills; an absolute root is passed through native
`--no-skills --no-extensions --skill <root>` so defaults and extension skills cannot leak.
An unset skill root preserves upstream skill discovery.
An unset `PI_ACP_STATE_DIR` preserves `homedir()/.pi/pi-acp`, including cli_local;
relative or empty state-directory overrides fail explicitly. Pi itself is unchanged.

Apply and build in a private checkout of that exact commit:

```sh
git apply --unidiff-zero /absolute/path/pi-acp-0.0.22-state-dir.patch
npm ci --ignore-scripts
npm run build
```

Pinned upstream package-lock SHA256:
`e257ee4cbe345ee0de44714d34401ed2ed14de41abc2c6ed7511b1a3d923c606`.
Patch SHA256:
`88c36304e8d2b510fb684477c53f9f712de2b02616064093b36dfa78e5a4ac2d`.
Built dist/index.js SHA256 (Node 26.13.0 / npm 11.20.0, Darwin arm64):
`5132b90b8244ef8ab8ddbda77a812b5e477501baafb25862d1c0c0d1623f9dc6`.
The session_model driver rejects any other adapter artifact before launching it;
launch the executable dist/index.js directly (no npx wrapper or extra arguments).
The native engine used for acceptance is the real published
`@mariozechner/pi-coding-agent@0.73.1`, gitHead
`781152fc24841dc54b22284514604048ebe5e2c9`; no second agent implementation is maintained.

Version and model-request acceptance are separate: pi-acp 0.0.22 has no --version
output. Verify package identity, patch/build digest and native Pi version rather
than adding a fake version-reporting CLI. Installer/registry publication belongs
to task 33 and must preserve this pin.

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
