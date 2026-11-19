# Gemini CLI 0.66.0 官方工具与 MCP 共存回执

2026-11-19。在 Darwin 27.0 arm64、Node.js 24.22.3 复验。v2 和原包均在获准的合成夹具目录内通过原生工具与 MCP 共存、单次拒绝及撤除对照。两组在空信任文件下也保留官方 `read_file`，目录信任门禁在 MCP 握手前生效。

## 制品、入口与权限

- 内置注册 key 为 `gemini`。通用入口是本机 `gemini` 或 `gemini-cli` 加 `--experimental-acp`，通用 npx 回退参数为 `@google/gemini-cli@latest --experimental-acp`。本轮固定的是独立的受管合约 `@google/gemini-cli@0.66.0`。仓库受管安装脚本在空私有运行目录成功安装 v2，退出 0。原包来自同次安装的私有 npm 缓存，独立下载超时后改从缓存提取，归档 SHA-512 与合约一致。原版及 v2 的七项入口及 bundle 摘要全部比对通过，两者均自报 `0.66.0`。入口 `bundle/gemini.js` SHA-256 为 `f48ad00f929e699361d442440e89a70ab08342f9042ea22f99f0e9ff4bf60ba6`，审批相关 chunk `bundle/gemini-EOCRFAPZ.js` 原版为 `6fb39e04f8e909efd5848ae9530487f91d9518bfc4d894b0d18c6a8b8c9c920b`，v2 为 `64eb440e91f6f595d7d324cc50935254f0164337a14547d60418e3c625400b35`。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [受管合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 两组都直接运行私有安装中的 `node_modules/.bin/gemini --experimental-acp`，v2 追加 `--shiju-model-source`，设置私有历史和空技能根。每轮使用私有 HOME、原生 `GEMINI_CLI_TRUSTED_FOLDERS_PATH` 与一次性回环占位认证，实际模型请求均命中回环上的 `gemini-2.5-flash:streamGenerateContent`。ACP 回读原生权限模式为 `default`。无跳过信任、免审批参数或永久批准。直接 ACP 探针覆盖适配器行为，ShiJu 宿主权限桥本轮按源码核对。
- 原生入口保留选模。受管选模要求核验入口并锁定私有回环模型路由，管理员模型或认证配置冲突时启动前拒绝。v2 的 MCP 审批标记由受管入口附加，宿主校验受审制品、活动提示、会话内服务及工具字段，沿单次审批路径执行，保留原工作流门禁。本轮未运行管理员冲突对照、通用 npx 入口、Linux、Windows 或真实供应商模型。ACP 声明支持 HTTP 和 SSE，本轮运行仅覆盖 stdio 挂载形状。[选模驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/gemini.rs) · [宿主桥](../../../../../crates/libs/acp-client/src/client/event_client.rs) · [补丁说明](../../../../../patches/acp-model-source/gemini/README.md)

## 回环三轮与原包对照

空信任组各轮新建临时工作区、进程及会话，并传入空信任文件。挂载轮向 `session/new` 传递只读 stdio 服务 `coexist_probe`，服务会记录握手、枚举及调用的 RPC ID。另两轮发送空服务列表。模型仅在收到文件的真实随机标记后续轮。六轮均由 Gemini 原生执行 `read_file`，ACP 工具 ID 为 `read_file__fixture-native`，状态从 `in_progress` 到 `completed`，续轮的 `functionResponse.id` 为 `fixture-native` 且含真实标记。六轮各有两次模型请求、零审批，模型均未见 `mcp_coexist_probe_read_probe` 声明。

| 入口 | 轮次 | ACP 会话 ID | MCP 握手 / 调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| v2 | 无挂载 | `3d2402b5-6139-44a2-82ea-1e465ad2eab4` | 0 / 0 | `end_turn` / 0 |
| v2 | 挂载未受信 | `a51fb8b4-e4e2-4032-ad6f-4f0d45516f56` | 0 / 0 | `end_turn` / 0 |
| v2 | 撤除后的新会话 | `49098ca9-cdf6-4366-b8ca-2391a1bb5b10` | 0 / 0 | `end_turn` / 0 |
| 原版 | 无挂载 | `c1975aef-1544-4e5c-bc82-8cda6b4b9ac0` | 0 / 0 | `end_turn` / -15 |
| 原版 | 挂载未受信 | `4ef6f92a-b7dd-48d6-9268-25d91ce0e2e3` | 0 / 0 | `end_turn` / -15 |
| 原版 | 撤除后的新会话 | `a5561cb2-1c1e-4514-a3bc-d2e6c00844b6` | 0 / 0 | `end_turn` / 0 |

两个 `-15` 来自探针收到 ACP 终态后关闭 stdin、等待 3 秒并向仍存活的进程发送 SIGTERM。两个未受信挂载轮都收到 Gemini 的目录未受信诊断，固定 bundle 内的 `startConfiguredMcpServers` 在枚举服务前检查 `isTrustedFolder()`。最早缺失的证据是 MCP 握手，因而没有 MCP 调用 ID 或审批结果。撤除轮的零调用属于未受信对照。

v2 在模型请求等待中收到 `session/cancel`，会话 `eba42831-c1a5-4052-9f05-da6d1f22d2d5` 返回 `cancelled`，零工具事件、零服务请求，进程退出 0。另一次无挂载会话 `c5f89a8c-297d-4787-9564-0fc756efd510` 完成同 ID 的原生读和续轮后，同一进程 `session/load` 成功，回读 `default` 及 `gemini-2.5-flash`，退出 0。初始化未声明独立 resume 能力。

## 受信三轮与单次拒绝

用户明确授权本轮私有合成夹具目录后，仅在每个探针的临时信任文件中写入该规范路径的 `TRUST_FOLDER`。各轮新建进程和会话，在同一获准目录生成随机文件标记。MCP 服务只返回合成文本。回环模型先发 `read_file`，读回文件标记后才在挂载和拒绝轮发 `mcp_coexist_probe_read_probe`。

| 入口 | 轮次 | ACP 会话 ID | MCP 握手 / 调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| v2 | 无挂载 | `f1bf6ec0-67f2-4eeb-9244-fdba98127c8d` | 0 / 0 | `end_turn` / 0 |
| v2 | 挂载，单次允许 | `c429478b-e4cf-4754-a301-bc83d2631e2a` | 1 / 1 | `end_turn` / 0 |
| v2 | 挂载，单次拒绝 | `5e1126b2-2104-4a29-b8bf-42248978a03a` | 1 / 0 | `end_turn` / 0 |
| v2 | 撤除后的新会话 | `6de1920a-af07-4acb-b80b-8bfa33631519` | 0 / 0 | `end_turn` / 0 |
| 原版 | 无挂载 | `ebca178c-3c84-4eca-af15-b165566703f0` | 0 / 0 | `end_turn` / 0 |
| 原版 | 挂载，单次允许 | `c58e4abf-ef2d-4f5f-8a4b-4ab86fcfad2a` | 1 / 1 | `end_turn` / 0 |
| 原版 | 挂载，单次拒绝 | `5e0e73ea-4341-41c9-b191-292174aa394c` | 1 / 0 | `end_turn` / 0 |
| 原版 | 撤除后的新会话 | `8e83e945-9dec-4b64-a52d-a59e8f7df9d1` | 0 / 0 | `end_turn` / 0 |

- 八轮均由 Gemini 原生执行 `read_file`，ACP ID `read_file__fixture-native` 完成，模型续轮在 `fixture-native` 下收到真实文件标记。无挂载与撤除轮均有两次模型请求，工具声明无目标服务，服务日志为空。
- 两组挂载允许轮的审批请求 ID 均为 0，ACP 工具 ID `mcp_coexist_probe_read_probe__fixture-mcp` 从 `pending` 到 `completed`。stdio 服务记录 `initialize` ID 0、`tools/list` ID 1、`tools/call` ID 2。第三次模型请求在 `fixture-mcp` 下收到服务响应。拒绝轮同一 ACP 工具 ID 变为 `failed`，服务仅握手和枚举，第三次模型请求在相同 ID 下收到取消结果。
- v2 的成功和拒绝审批均保留 `other`，附 `shiju.gemini/mcp` 元数据，服务名为 `coexist_probe`，工具名为 `read_probe`，选项只有 `allow_once` 与 `reject_once`。原版发泛型 `other`，没有对应元数据，并额外给出 `allow_always`。直接 ACP 探针只选择单次选项，因此原版可以执行；ShiJu 原有无类型工作流门禁在宿主入口仍区分这两种请求。
- v2 另在受信会话 `0e854594-5f7b-488b-a528-31b2b0f53d72` 依次执行新建、携带服务 `session/load`、清空服务再次 `session/load`。两个加载回读都为 `default`，三次提示均为 `end_turn`，进程退出 0。带挂载加载时，`read_file__mounted-fixture-native` 和 `mcp_coexist_probe_read_probe__mounted-fixture-mcp` 均完成，服务请求 2 的结果以 `mounted-fixture-mcp` 进入续轮。清空后仍回放历史 ACP 完成事件，本轮只执行 `read_file__removed-fixture-native`，模型声明无目标工具，服务实际收到零请求。跨进程恢复本轮未执行。

## 结论与历史证据

当前固定 v2 在既有权限内通过本机受信共存复验。正向、拒绝和撤除均保留原生发现与权限路径。历史 v2 网关在当次目录授权下验证了 ShiJu 宿主审批和真实模型续轮，挂载轮有 57 次目标调用和一次官方主题调用，前后零目标事件。本轮通过直接 ACP 核对适配器与审批协议，未重跑宿主网关。没有发现适配器无意裁剪，未修改代码或权限策略，未运行 Cargo。临时信任文件已清理，受信探针及服务进程已退出。探针与脱敏结果保存在本机私有目录 `shiju-gemini-task15.4seJ5eVf`。[历史验收](../../mcp-acceptance/gemini.md#正式-v2-网关复验)
