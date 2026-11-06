# 完全访问能力证据

采集日期：2026-11-06（Asia/Shanghai）；平台：Darwin arm64。工作树审计基点：`1598d3243d88a3c707c26eeaba3a5460eddbecb1`。本任务无前置依赖；仅新增本文和 [支持矩阵](../support-matrix.md)。版本查询不运行 CLI、不安装或更新软件。

## 来源与可复查性

E1 代表仓库源码静态事实；E0 是尚待确认的线索；权限 E2/E3 定义见矩阵。所有权限外部行为仍未知。下面的“发布查询”来自本次官方 HTTP 响应，只用于核对发布版本。SHA256 为本次收到的原始响应字节摘要，动态端点重新查询可能变化；未保存完整响应。需要长期回放权限结论时，应保存固定版本源码和经脱敏的最小协议证据。

历史版本、上游 commit、完整 SRI、归档/源码哈希及 URL 可在 [模型来源版本清单](../../acp-model-source/cli-versions.json) 和 [托管契约](../../../../crates/vw-acp/src/managed_runtime/contracts.json) 中复查。本次不把该清单的模型来源状态升级为权限证据。制品已修改时要分别记录 original、patched 与 engine，不能只核对 npm integrity。未使用旧 target 目录作为证据。

## 审计来源 SHA256

| 仓库来源 | SHA256 |
| --- | --- |
| [crates/vw-acp/src/agent_registry.rs](../../../../crates/vw-acp/src/agent_registry.rs) | `938893e98645a6927b60e7b9e797156de603d8405dbe6a644e585b31cc503373` |
| [crates/vw-acp/src/managed_runtime.rs](../../../../crates/vw-acp/src/managed_runtime.rs) | `f0601c3c4419f1221828cb9517050deeea02921420ba36c00a05a5280f9e23e4` |
| [crates/vw-acp/src/managed_runtime/contracts.json](../../../../crates/vw-acp/src/managed_runtime/contracts.json) | `df288d87c1929b7faf475859cd8158e894344a094b9385bd720db9aa1da63c22` |
| [docs/shiju/acp-model-source/cli-versions.json](../../../../docs/shiju/acp-model-source/cli-versions.json) | `e2bfd02ca8f2e43eb5ba9f22c54abc557e91cd0249e6d4c27ec3b06f8a8797d3` |
| [crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs) | `f26b36b764954a6a7c5a4f31343dfc1c6691e6fb0fa2ee8621d1769033b01c2e` |
| [crates/vw-agent/src/session/llm/acp/model_source/mod.rs](../../../../crates/vw-agent/src/session/llm/acp/model_source/mod.rs) | `b95cba9a6367f5cbc147bcab0d3be967841e14e777eff073ffdf0fab4cb31b0c` |
| [crates/vw-agent/src/session/llm/acp/model_source/profile.rs](../../../../crates/vw-agent/src/session/llm/acp/model_source/profile.rs) | `5a369b794634c5d551d50deebb671ce631304cfac2f31ac44ed22f1375ee881b` |
| [crates/vw-agent/src/session/llm/acp/config.rs](../../../../crates/vw-agent/src/session/llm/acp/config.rs) | `1ce904fdee8c66c23a084bb78caad481d6baead3022c395c236ac8974c882ab1` |
| [crates/vw-acp/src/client/builder.rs](../../../../crates/vw-acp/src/client/builder.rs) | `3ba483019a211407f32b0556877296e2b07d25fad2190a54f34d47bc55fba8e8` |
| [crates/vw-acp/src/client/event_client.rs](../../../../crates/vw-acp/src/client/event_client.rs) | `e29133e75ca2468c2f09bf34899efba96713f287693beb4aaf94d496b2ea0dad` |
| [crates/vw-acp/src/permissions.rs](../../../../crates/vw-acp/src/permissions.rs) | `5ada48245da4f3cd669be6cfb245e8cbcfe4e542834ee54c2406a34828c6aac5` |
| [crates/vw-acp/src/agent_registry_tests.rs](../../../../crates/vw-acp/src/agent_registry_tests.rs) | `e32b093dd8b61a95541a7c40aafde5d5719ce05d76768a93ff35b51a21f622ee` |
| [crates/vw-acp/src/managed_runtime_tests.rs](../../../../crates/vw-acp/src/managed_runtime_tests.rs) | `2b98f0115992389627b5b00960cbabd3ddf3bfebb30154b6457f40fa4b4d560f` |
| [crates/vw-acp/src/client/event_client_tests.rs](../../../../crates/vw-acp/src/client/event_client_tests.rs) | `179c294224e844628a5265a1af1d4b4a9bd85ddaa739c33cb8ec1fbfdc045cb3` |
| [crates/vw-acp/src/permissions_tests.rs](../../../../crates/vw-acp/src/permissions_tests.rs) | `0b48c665c781679881d00fa575d7d1ed146a9c61a5e7c414456470d2d5e711c9` |

## 注册表快照与逐项来源

以下普通默认、local candidates/local args 来自 `BUILT_IN_AGENT_DEFINITIONS`；托管来自 `reviewed_agent_command`。`<reviewed>` 表示按 contract version 派生目录，校验成功后才返回的可执行文件；不是已安装路径记录。每个 driver 的 configure 入口以对应源码链接和 SHA256 为准。私有目录/中继/模型参数只描述入口，本文不存 token、环境、用户绝对路径或原始提示。

### auggie

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx @augmentcode/auggie@latest --acp`。
- 本地候选：`auggie`；local args：`--acp`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/auggie.rs)；显式拒绝：Provider 注册被拒绝 SHA256 `de3dd19a9748bd2d38258050bf30e210ba70447a695f5b2896f461cea487de2d`。
- 基线发布 `0.36.0`：[官方来源](https://registry.npmjs.org/%40augmentcode%2Fauggie/0.36.0)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.36.0`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-jb4kq97pGzG0i5UyISc//LjTVlFzIywwD+VfPemiSIl58KjLfxry3KDgxWYnr1vmV8yUkySVzjLnj4ewUEbT/Q==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40augmentcode%2Fauggie/latest)：HTTP 200，版本 `0.37.0`；响应 SHA256 `482a57d7b8d60e9b50a71415a12d8da2cdf903a814bdbafd5d34c66aa22dc9ec`。最终 URL：[响应来源](https://registry.npmjs.org/%40augmentcode%2Fauggie/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### claude

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx -y @agentclientprotocol/claude-agent-acp@^0.26.0`。
- 本地候选：`claude-agent-acp`；local args：`[]`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/claude.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `68cd69188d15ce6ec65b2e832c77c2acc4bbbefebb05eb3dc72c831ba62d59ab`。
- 基线发布 `0.26.0`：[官方来源](https://registry.npmjs.org/%40agentclientprotocol%2Fclaude-agent-acp/0.26.0)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`2.1.96`；kind `agent_sdk`。
- 基线 upstream commit：`0a501434c0bcd8308c66c7469b9bb4c7cf9c36b9`（历史元数据；本次未 checkout）。
- 基线发布完整 SRI：`sha512-eNbunIaVwvm+CS7x51cLuosPi8iCrcfUrJLr7zIR6NIlpeXvE3hg/1VY+mRIcHHGmYQtsqPUQaeSqP1w19/fFg==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40agentclientprotocol%2Fclaude-agent-acp/latest)：HTTP 200，版本 `0.101.0`；响应 SHA256 `6e0e7e819914289c6154df0bf1a919db80b14e35fa9631d5f144015ee1ba570e`。最终 URL：[响应来源](https://registry.npmjs.org/%40agentclientprotocol%2Fclaude-agent-acp/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### codex

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx -y @agentclientprotocol/codex-acp@latest`。
- 本地候选：`codex-acp`；local args：`[]`。
- 托管：`<reviewed>/node_modules/.bin/codex-acp `；contract `2.5.0`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::codex.files`；原始制品 `original`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `24c942d5fcd52ef7163b6ed93ba2436d019976ed7388734e1e4f011b66f332d6`。
- 基线发布 `2.5.0`：[官方来源](https://registry.npmjs.org/%40agentclientprotocol%2Fcodex-acp/2.5.0)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.165.0`；kind `external_cli`。
- 基线 upstream commit：`34e08970aa324a2be985f5080674d5aa91d7666f`（历史元数据；本次未 checkout）。
- 基线发布完整 SRI：`sha512-7e/4OhJwpvSTqsjiokoVYRV7O1XGem1CAsQgO1opmGYbRD5LvmydwDlJrldzQYO5TcPWueGzQ2Jdvyvp6l3qDw==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40agentclientprotocol%2Fcodex-acp/latest)：HTTP 200，版本 `2.5.0`；响应 SHA256 `35ad31794c25be1b4a05b1b97b2e66b7571544dd26e699ade043590e8e3919ab`。最终 URL：[响应来源](https://registry.npmjs.org/%40agentclientprotocol%2Fcodex-acp/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### codebuddy

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`codebuddy --acp`。
- 本地候选：`codebuddy`；local args：`--acp`。
- 托管：`<reviewed>/node_modules/.bin/codebuddy `；contract `2.179.3`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::codebuddy.files`；原始制品 `original`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/codebuddy.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `3e6322b7c04a84b96b1a4218dd0b5b6209cc9e9600a356a0072c9693874e5627`。
- 基线发布 `2.179.3`：[官方来源](https://registry.npmjs.org/%40tencent-ai%2Fcodebuddy-code/2.179.3)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`2.179.3`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-SQy1YDuITzH2MClXCrLBVIzW93syDE8VXWsyok8g3eGmahdBE7G+1aR4tBZvQHMwjPANLasTkINM26nUKstPHQ==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40tencent-ai%2Fcodebuddy-code/latest)：HTTP 200，版本 `2.180.0`；响应 SHA256 `9b3144aeb2d5d61d9ab8b86baf7c7e2d642e85c76678b2d3165c006fba2f4011`。最终 URL：[响应来源](https://registry.npmjs.org/%40tencent-ai%2Fcodebuddy-code/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### goose

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`goose acp`。
- 本地候选：`goose`；local args：`acp`。
- 托管：`<reviewed>/goose acp`；contract `v1.57.0`，layout `native`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::goose.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/goose.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `4d2928f682d26beb7424775c1bc45443445dfd8fa2c5da86794436db614f9986`。
- 基线发布 `v1.57.0`：[官方来源](https://api.github.com/repos/aaif-goose/goose/releases/latest)；历史标记 `official_release_metadata_not_executed`。底层身份：`v1.57.0`；kind `same_distribution`。
- 本次 [发布查询](https://github.com/aaif-goose/goose/releases/latest)：HTTP 200，版本 `v1.57.0`；响应 SHA256 `9b6d89fe0958f625cc1996345fde7d8517c9e4afb4988151a4b494623ecdae82`。最终 URL：[响应来源](https://github.com/aaif-goose/goose/releases/tag/v1.57.0)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### vibe

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`vibe-acp `。
- 本地候选：`vibe-acp`；local args：`[]`。
- 托管：`<reviewed>/bin/vibe-acp `；contract `3.0.4`，layout `python`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::vibe.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/vibe.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `eeb1728bf91aa8577e5320f1595af183458e739572263ddd3ab465eb74f7b164`。
- 基线发布 `3.0.4`：[官方来源](https://pypi.org/pypi/mistral-vibe/3.0.4/json)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`3.0.4`；kind `same_distribution`。
- 本次 [发布查询](https://pypi.org/pypi/mistral-vibe/json)：HTTP 200，版本 `3.1.0`；响应 SHA256 `0cb42454aa9bf548c1c51d17204765bdca0363483ce7d76b1717d7607b83901b`。最终 URL：[响应来源](https://pypi.org/pypi/mistral-vibe/json)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### hermes

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`hermes acp`。
- 本地候选：`hermes`；local args：`acp`。
- 托管：`<reviewed>/bin/hermes acp`；contract `0.19.0`，layout `python`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::hermes.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/hermes.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `25c61245b541a6dba35197a645ed5e6663dd61bc5c5707198026b73962c4d957`。
- 基线发布 `0.19.0`：[官方来源](https://pypi.org/pypi/hermes-agent/0.19.0/json)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.19.0`；kind `same_distribution`。
- 本次 [发布查询](https://pypi.org/pypi/hermes-agent/json)：HTTP 200，版本 `0.19.0`；响应 SHA256 `84679f4bcdbbf9d4dedde6c7814cd6989fc10b9219685f1c706e7ca7042f16af`。最终 URL：[响应来源](https://pypi.org/pypi/hermes-agent/json)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### snow

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`snow --acp`。
- 本地候选：`snow`；local args：`--acp`。
- 托管：`<reviewed>/node_modules/.bin/snow --acp`；contract `0.9.14`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::snow.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/snow.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `a4b9fe2ccf7e477c5a315394690b8ab5c51a2f334eff7d06e60a68aebb78f2f9`。
- 基线发布 `0.9.14`：[官方来源](https://registry.npmjs.org/snow-ai/0.9.14)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.9.14`；kind `same_distribution`。
- 基线 upstream commit：`a9eba5f9062afbf639829b4992e90a986924f519`（历史元数据；本次未 checkout）。
- 基线发布完整 SRI：`sha512-hVawSqfpwqSJgTAPVNp7Lw7gUoBpP89eKIC/lN5f6dvU8HWQtIo0M3BUhqByPmSUM8jZr1bbKF3CbQ9ur0pRaw==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/snow-ai/latest)：HTTP 200，版本 `0.9.15`；响应 SHA256 `98353a05965a06a6ede1383f051a2b405148f9d6b5595f2e01e7e2b6c367c46d`。最终 URL：[响应来源](https://registry.npmjs.org/snow-ai/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### copilot

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`copilot --acp --stdio`。
- 本地候选：`copilot`；local args：`--acp --stdio`。
- 托管：`<reviewed>/node_modules/.bin/copilot --acp --stdio`；contract `1.0.105`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::copilot.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/copilot.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `657941e6ff6f6e32986e777d72239e4f3655c73fc5a27f28126aabf53398d845`。
- 基线发布 `1.0.105`：[官方来源](https://registry.npmjs.org/%40github%2Fcopilot/1.0.105)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`1.0.105`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-k5JPVZrc0Ms4YdAmYS80OGxDvwjDZNVqDS3dxphIFUVSTFJRwcHkbHh3yUKk7MEyBjKqQYKLKNAKSF+dVmjl7A==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40github%2Fcopilot/latest)：HTTP 200，版本 `1.0.106`；响应 SHA256 `60a6f8a87cf7f73b237c74072c184eb73c155b92914d590078028ac4a43f3967`。最终 URL：[响应来源](https://registry.npmjs.org/%40github%2Fcopilot/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### cursor

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`cursor-agent acp`。
- 本地候选：`cursor-agent`；local args：`acp`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/cursor.rs)；显式拒绝：第三方 Provider 契约未核验 SHA256 `f657303277bee655013af357fa2ef03d35813eed8ca1fa0eb23f54f27772c1f4`。
- 基线发布 `2026.10.22-27dcf5f`：[官方来源](https://cursor.com/install)；历史标记 `official_release_metadata_not_executed`。底层身份：`2026.10.22-27dcf5f`；kind `same_distribution`。
- 本次 [发布查询](https://cursor.com/install)：HTTP 200，版本 `2026.11.05-86047df`；响应 SHA256 `f32ab18113f616745c119cd9fa0ffd0d743bc4ea4ee4aeaa98ac907934d0b568`。最终 URL：[响应来源](https://cursor.com/install)。 原生安装器只静态读取版本文本，未执行。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### droid

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`droid exec --output-format acp`。
- 本地候选：`droid`；local args：`exec --output-format acp`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/droid.rs)；显式拒绝：双向 ACP 与隔离根未核验 SHA256 `1a7b496e86eae8930506c6445a93a5775d71199f56499363dcd13c6581f86109`。
- 基线发布 `1.3.0`：[官方来源](https://app.factory.ai/cli)；历史标记 `official_release_metadata_not_executed`。底层身份：`1.3.0`；kind `same_distribution`。
- 本次 [发布查询](https://app.factory.ai/cli)：HTTP 200，版本 `1.5.0`；响应 SHA256 `054998ae1431592897d943adc320a58aaef53922bf8048891de9c259ffaa3694`。最终 URL：[响应来源](https://app.factory.ai/cli)。 原生安装器只静态读取版本文本，未执行。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### gemini

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx @google/gemini-cli@latest --experimental-acp --approval-mode yolo --skip-trust`。
- 本地候选：`gemini, gemini-cli`；local args：`--experimental-acp --approval-mode yolo --skip-trust`。
- 托管：`<reviewed>/node_modules/.bin/gemini --experimental-acp`；contract `0.66.0`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::gemini.files`；原始制品 `original`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/gemini.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `83e84a229a873075c815ca831f7561811a363911855e3d6859c9eecbd9c0e607`。
- 基线发布 `0.66.0`：[官方来源](https://registry.npmjs.org/%40google%2Fgemini-cli/0.66.0)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.66.0`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-EAtsJ9/JxiphHlduzvhoCkb7I0LdOxyugB5NpUtVqKVGZIWjV3JmurvK89ZrvMbV3kwk5SdWINz1M5ZN/2K/Lw==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40google%2Fgemini-cli/latest)：HTTP 200，版本 `0.67.0`；响应 SHA256 `f9b09cc93c67f5663b5dea97cd52e1d4dd55d1490aa9abf560b1ae725871d353`。最终 URL：[响应来源](https://registry.npmjs.org/%40google%2Fgemini-cli/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### iflow

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`iflow --experimental-acp`。
- 本地候选：`无通用本地优先候选`；local args：`[]`。
- 托管：`<reviewed>/node_modules/.bin/iflow --experimental-acp`；contract `0.5.19`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::iflow.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/iflow.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `d0221dfdbe9e5d441160d9b26ff3c46db5a50a6221e15e609978d91eadc205b7`。
- 基线发布 `0.5.19`：[官方来源](https://registry.npmjs.org/%40iflow-ai%2Fiflow-cli/0.5.19)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.5.19`；kind `same_distribution`。
- 基线 upstream commit：`da3e453f790995bcd56850264cf2810610a48857`（历史元数据；本次未 checkout）。
- 基线发布完整 SRI：`sha512-cOQNLXJAjqcKq6XQpx47EUV8ocY5sdLXiy7MfJm+b1miRlMJmrQvueT75w1h9UqzrlFPio2VluTxbLRBoR/QBA==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40iflow-ai%2Fiflow-cli/latest)：HTTP 200，版本 `0.5.19`；响应 SHA256 `97624d6bc01cd452189ecac5a2b507520a549eda6a39ca32919b9d61d51e1dc5`。最终 URL：[响应来源](https://registry.npmjs.org/%40iflow-ai%2Fiflow-cli/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### kilocode

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx -y @kilocode/cli acp`。
- 本地候选：`kilocode`；local args：`acp`。
- 托管：`<reviewed>/node_modules/.bin/kilocode acp --pure`；contract `7.8.18`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::kilocode.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/kilocode.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `387891bd76550c902e23a699629b49f07b485255a38f646da32adcd50147b313`。
- 基线发布 `7.8.18`：[官方来源](https://registry.npmjs.org/%40kilocode%2Fcli/7.8.18)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`7.8.18`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-/Ym2x0IK6drlX4eZK79JcAcKU90sxnqQlInUCiYtpekyMXeMGHLbr0WQpDrJGyTaAJQRhQ4WStADaaQtfQC/nQ==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40kilocode%2Fcli/latest)：HTTP 200，版本 `7.8.27`；响应 SHA256 `854811f2c6f225ed8d8d6a6521f649cce8414b5928678ec080f1e6e2621c8b63`。最终 URL：[响应来源](https://registry.npmjs.org/%40kilocode%2Fcli/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### kimi

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`kimi acp`。
- 本地候选：`无通用本地优先候选`；local args：`[]`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/kimi.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `fc2e369389f4aa78846271ccafae143b82f8ecf9d8a065805baf2a43feaa0f26`。
- 基线发布 `2.5.0`：[官方来源](https://code.kimi.com/kimi-code/binaries/2.5.0/manifest.json)；历史标记 `pinned_official_native_binary`。底层身份：`2.5.0`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-AYuT4fW7pzZAqxIIDjx+ypLJNoAL07NZCUyPDlf/JrgyKri35eVq9mDuhoe/xRX7wNHL+1fX+H0Dj/rB3H04Yg==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40moonshot-ai%2Fkimi-code/latest)：HTTP 200，版本 `2.5.1`；响应 SHA256 `4c1db90e5b3e03ec5e18583b0163bca650284b5ddf1332bbeb6b1ee4d01e117f`。最终 URL：[响应来源](https://registry.npmjs.org/%40moonshot-ai%2Fkimi-code/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### kiro

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`kiro-cli acp`。
- 本地候选：`kiro-cli`；local args：`acp`。
- 托管：`<reviewed>/kiro-cli acp`；contract `3.0.0`，layout `standalone`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::kiro.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/kiro.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `1310590bef71c923bfba909ad834bd1c158d528b315b7804bf970f0d59f317cd`。
- 基线发布 `3.0.0`：[官方来源](https://prod.download.cli.kiro.dev/stable/latest/manifest.json)；历史标记 `official_binary_verified_offline`。底层身份：`3.0.0`；kind `same_distribution`。
- 本次 [发布查询](https://prod.download.cli.kiro.dev/stable/latest/manifest.json)：HTTP 200，版本 `3.1.0`；响应 SHA256 `cc98a9f9989bad047e5f29ea6f1fb7bb40b68154b164d714a2a0efadfbf2536c`。最终 URL：[响应来源](https://prod.download.cli.kiro.dev/stable/latest/manifest.json)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### opencode

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx opencode-ai@latest acp`。
- 本地候选：`opencode`；local args：`acp`。 OpenCode 另按 OPENCODE_BIN、用户目录、PATH 查找。
- 托管：`<reviewed>/node_modules/.bin/opencode acp`；contract `1.19.5`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::opencode.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/opencode.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `0c50cd052da38a304a339bfd889774b32dd26c3ac07a6d10c3a6c822ed446050`。
- 基线发布 `1.19.5`：[官方来源](https://registry.npmjs.org/opencode-ai/1.19.5)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`1.19.5`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-tEqMvQahfhdDj8vceYpXtoIrvjPXm5XRfetWnPN5YISueU7adNx9JcUA3zZ+2GKuDVgDpGdQe1ZLtCZDjqodvA==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/opencode-ai/latest)：HTTP 200，版本 `1.19.5`；响应 SHA256 `8238b09a1b89cd12b3cead60e0bb29bc0c247541486de085e7d4bc9fb47f73e7`。最终 URL：[响应来源](https://registry.npmjs.org/opencode-ai/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### openclaw

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx openclaw acp`。
- 本地候选：`openclaw`；local args：`acp`。
- 托管：`<reviewed>/node_modules/.bin/openclaw acp`；contract `2026.10.10`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::openclaw.files`；原始制品 `original`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/openclaw.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `f4f4484c67ee6a914c9dd1fff5a5c4f16ee4b66167742c38e5de559baddf2652`。
- 基线发布 `2026.10.10`：[官方来源](https://registry.npmjs.org/openclaw/2026.10.10)；历史标记 `runtime_verified_fixed_release`。底层身份：`2026.10.10`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-9HZhzq/Yck3uH3M/iYP7szqoF5JXpXfz0+orhBOKalSH+dj0IC9uWPECkR1vH4lMCytxpHjJKHh9efXRYMJl6w==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/openclaw/latest)：HTTP 200，版本 `2026.11.1`；响应 SHA256 `03cd9bdbd1b125da008da567e81c14f0d370874a19e7be3cc0fe1a4a4c223f73`。最终 URL：[响应来源](https://registry.npmjs.org/openclaw/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### pi

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx -y pi-acp@^0.0.22`。
- 本地候选：`pi-acp`；local args：`[]`。
- 托管：`<reviewed>/node_modules/.bin/pi-acp `；contract `0.0.22`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::pi.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/pi.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `2a3982193041115e722e893caa32d6587fa58f598366438683a631a966cf4077`。
- 基线发布 `0.0.22`：[官方来源](https://registry.npmjs.org/pi-acp/0.0.22)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.73.1`；kind `external_cli`。
- 基线 upstream commit：`8be5f22769f71d759aa48d798416c56e8be101ef`（历史元数据；本次未 checkout）。
- 基线发布完整 SRI：`sha512-A9DFxi3dF/xFgf0vm5r5jvEtTszRXqK2MUYXWQN0aSSLeJW53efqFlueczZCj5J119s+FLlQVQYso71PDPL0sw==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/pi-acp/latest)：HTTP 200，版本 `0.0.34`；响应 SHA256 `9d9b8fe9111564571d8e69edf9e14dce9ba6faf5d885858886c9da9a931b34b8`。最终 URL：[响应来源](https://registry.npmjs.org/pi-acp/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### qoder

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx @qoder-ai/qodercli@latest --acp --permission-mode bypass_permissions`。
- 本地候选：`qodercli`；local args：`--acp --permission-mode bypass_permissions`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/qoder.rs)；显式拒绝：需要账号授权及权益 SHA256 `d24d40877b9efd7ef31500e232c704df2fd37aa8b48606dfa19ae4fe205e4c40`。
- 基线发布 `1.1.82`：[官方来源](https://registry.npmjs.org/%40qoder-ai%2Fqodercli/1.1.82)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`1.1.82`；kind `same_distribution`。
- 基线发布完整 SRI：`sha512-Vac4oHtL4F5oUbn51e3OD+CnHFQbMbqqO4a1cy0AnGGzELzvfHXCExgfhqwXUlTUatZbbKq45oMt8Od/GPPtBw==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40qoder-ai%2Fqodercli/latest)：HTTP 200，版本 `1.1.83`；响应 SHA256 `937f9889dd99851a6097af8c190c4069634839bd4bdd441f379f7835fefe9f65`。最终 URL：[响应来源](https://registry.npmjs.org/%40qoder-ai%2Fqodercli/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### qwen

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`npx @qwen-code/qwen-code@latest --acp --experimental-skills --approval-mode yolo`。
- 本地候选：`qwen, qwen-code`；local args：`--acp --experimental-skills --approval-mode yolo`。
- 托管：`<reviewed>/node_modules/.bin/qwen --acp`；contract `0.26.7`，layout `npm`；npm 实际从 node_modules/.bin 入口并检查 canonical target。
- 制品哈希入口：`contracts.json::qwen.files`。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/qwen.rs)；configure 生成私有模型启动材料；是否接受当前命令还受该驱动校验约束。 SHA256 `a6523811354080490103d563ef45d3d46d7b64e9c5e9413135a43600389b9694`。
- 基线发布 `0.26.7`：[官方来源](https://registry.npmjs.org/%40qwen-code%2Fqwen-code/0.26.7)；历史标记 `metadata_snapshot_not_runtime_pin`。底层身份：`0.26.7`；kind `same_distribution`。
- 基线 upstream commit：`f45c34d8ccbd651f2f0f4d97c9b013c11af52f5b`（历史元数据；本次未 checkout）。
- 基线发布完整 SRI：`sha512-sRFWwmakpvF3bWBfG1M/ijoZG5h8Yxs0IC4cXZP3dJLyEbbTHItyhzMySG4o4CzLGFIfofArummQJo/xcggweQ==`（不是本次已安装文件摘要）。
- 本次 [发布查询](https://registry.npmjs.org/%40qwen-code%2Fqwen-code/latest)：HTTP 200，版本 `0.26.8`；响应 SHA256 `8acd1f544d8f25647c6fe3779389a60bf4fe431d2f4da9747f062f93a97beed5`。最终 URL：[响应来源](https://registry.npmjs.org/%40qwen-code%2Fqwen-code/latest)。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

### trae

- 证据：E1（注册/驱动源码）；原生权限 E2/E3：未知。
- 普通默认：`traecli acp serve`。
- 本地候选：`traecli`；local args：`acp serve`。
- 托管：无契约；本地/默认启动。
- session_model：[驱动](../../../../crates/vw-agent/src/session/llm/acp/model_source/drivers/trae.rs)；显式拒绝：需要平台认证，隔离根未核验 SHA256 `173ac38a345edafd497eeb9c01a55addb2edc3859db4c7ece025353f0cb5e326`。
- 基线发布 `0.121.3`：[官方来源](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_latest_version.txt)；历史标记 `official_release_metadata_not_executed`。底层身份：`0.121.3`；kind `same_distribution`。
- 本次 [发布查询](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_latest_version.txt)：HTTP 200，版本 `v0.121.3`；响应 SHA256 `18ac2de7bde79e64b9d1ba63b19b44c70eab71ba63ed5d872efcc9a444a4061c`。最终 URL：[响应来源](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_latest_version.txt)。 原生安装器只静态读取版本文本，未执行。
- 本次实际执行版本：未知（未启动 CLI）；下一项核验见矩阵对应行。

## session_model 启动叠加入口

以下是对应 driver 对已选启动命令的叠加，不是新的审批参数；敏感字段仅列配置名。普通启动和托管启动须分别验收这些叠加是否被接受。

| key | 启动叠加 / 相邻边界 |
| --- | --- |
| auggie / cursor / droid / qoder / trae | configure 直接返回明确错误，不启动 session_model。 |
| claude | 私有 settings.json 与 Provider/模型环境；保留企业权限约束，不额外传 CLI 审批参数。 |
| codex | 私有 config.toml；追加 `cli --shiju-model-source`，校验托管适配器与底层引擎。 |
| codebuddy | 模型环境；追加 `--setting-sources none`。 |
| goose | 隔离配置环境；追加 `--shiju-model-source`。 |
| vibe | VIBE_HOME 与私有 session-model.toml；追加 `--session-model-config <file>`。 |
| hermes | 私有 config.yaml 与空 .env；追加 `--session-model-binding`。 |
| snow | 私有 config.json 与配置根环境；不额外传审批参数。 |
| copilot | 私有 providers.json 与配置环境；追加 `--no-auto-update`。 |
| gemini | 私有 managed-settings.json 与模型环境；追加 `--shiju-model-source`。 |
| iflow | 隔离配置/模型环境；追加 `--shiju-model-source`。 |
| kilocode | 私有 kilo.json 与配置环境；缺少 `--pure` 时补充。 |
| kimi | 隔离 KIMI_CODE_HOME/config.toml；不额外传审批参数。 |
| kiro | 隔离模型环境；追加 `--agent-engine v3 --auth-method cli`。 |
| opencode | 配置与模型环境；不追加审批参数。 |
| openclaw | 私有 openclaw.json；启动 `gateway run --port <port> --bind loopback`；ACP 桥追加 `--url <loopback> --token-file <file> --no-prefix-cwd --session <id>`。 |
| pi | 隔离 models.json/settings.json 与引擎环境；设置 session_model_id，后续通过 ACP 选择模型。 |

Goose API 首次查询返回 HTTP 403（rate limit）；替代官方 releases/latest 页面成功解析到 v1.57.0，上述条目记录替代来源摘要。Kimi 当前发布查询采用官方 npm 包元数据，固定 2.5.0 manifest 只用于基线身份。

## 证据采集规范与检查

后续每份实机证据应包含日期、OS/架构、逻辑会话的脱敏关联、普通/托管、cli_local/session_model、adapter/engine 版本及可执行文件 SHA256、官方固定版本 URL/commit、原始/补丁制品区别、initialize 能力、new/load/resume、配置/模式读写结果、工具调用及审批往返次数、开启→关闭→恢复结果。明确列出失败、未执行和不适用项；不得写入 token、完整环境、账号或敏感载荷。

本次检查：`git diff --check`；解析注册表 stable key 与矩阵两张表逐项对照，22/22 且无重复；检查仓库相对链接存在、源码 SHA256 一致、22 个官方查询来源齐全。官方查询失败项均明确保留未知；若有替代来源成功，单独注明。没有权限实机验收，不能用于宣布原生免审批支持。
