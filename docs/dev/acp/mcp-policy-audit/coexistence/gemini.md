# Gemini CLI 0.66.0 官方工具与 MCP 共存回执

2026-11-19。在 Darwin 27.0 arm64、Node.js 24.22.3 复验。空信任文件下，v2 和原包均保留官方 `read_file`，但在 MCP 服务发现前触发目录信任门禁。当前缺已授权的本轮测试目录信任，受信正向共存及单次拒绝复验阻塞。

## 制品、入口与权限

- 内置注册 key 为 `gemini`。通用入口是本机 `gemini` 或 `gemini-cli` 加 `--experimental-acp`，通用 npx 回退参数为 `@google/gemini-cli@latest --experimental-acp`。本轮固定的是独立的受管合约 `@google/gemini-cli@0.66.0`。仓库受管安装脚本在空私有运行目录成功安装 v2，退出 0。原包来自同次安装的私有 npm 缓存，独立下载超时后改从缓存提取，归档 SHA-512 与合约一致。原版及 v2 的七项入口及 bundle 摘要全部比对通过，两者均自报 `0.66.0`。入口 `bundle/gemini.js` SHA-256 为 `f48ad00f929e699361d442440e89a70ab08342f9042ea22f99f0e9ff4bf60ba6`，审批相关 chunk `bundle/gemini-EOCRFAPZ.js` 原版为 `6fb39e04f8e909efd5848ae9530487f91d9518bfc4d894b0d18c6a8b8c9c920b`，v2 为 `64eb440e91f6f595d7d324cc50935254f0164337a14547d60418e3c625400b35`。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [受管合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 两组都直接运行私有安装中的 `node_modules/.bin/gemini --experimental-acp`，v2 追加 `--shiju-model-source`，设置私有历史和空技能根。每轮使用临时工作区、私有 HOME 与空的 `GEMINI_CLI_TRUSTED_FOLDERS_PATH`，使用一次性回环占位认证，实际模型请求均命中回环上的 `gemini-2.5-flash:streamGenerateContent`。ACP 回读原生权限模式为 `default`。无跳过信任、免审批参数或永久批准。直接 ACP 探针覆盖适配器行为，ShiJu 宿主权限桥本轮按源码核对。
- 原生入口保留选模。受管选模要求核验入口并锁定私有回环模型路由，管理员模型或认证配置冲突时启动前拒绝。v2 的 MCP 审批标记由受管入口附加，宿主校验受审制品、活动提示、会话内服务及工具字段，沿单次审批路径执行，保留原工作流门禁。本轮未运行管理员冲突对照、通用 npx 入口、Linux、Windows 或真实供应商模型。ACP 声明支持 HTTP 和 SSE，本轮运行仅覆盖 stdio 挂载形状。[选模驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/gemini.rs) · [宿主桥](../../../../../crates/libs/acp-client/src/client/event_client.rs) · [补丁说明](../../../../../patches/acp-model-source/gemini/README.md)

## 回环三轮与原包对照

各轮新建进程及会话。挂载轮向 `session/new` 传递只读 stdio 服务 `coexist_probe`，服务会记录握手、枚举及调用的 RPC ID。另两轮发送空服务列表。模型仅在收到文件的真实随机标记后续轮。六轮均由 Gemini 原生执行 `read_file`，ACP 工具 ID 为 `read_file__fixture-native`，状态从 `in_progress` 到 `completed`，续轮的 `functionResponse.id` 为 `fixture-native` 且含真实标记。六轮各有两次模型请求、零审批，模型均未见 `mcp_coexist_probe_read_probe` 声明。

| 入口 | 轮次 | ACP 会话 ID | MCP 握手 / 调用 | ACP 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| v2 | 无挂载 | `3d2402b5-6139-44a2-82ea-1e465ad2eab4` | 0 / 0 | `end_turn` / 0 |
| v2 | 挂载未受信 | `a51fb8b4-e4e2-4032-ad6f-4f0d45516f56` | 0 / 0 | `end_turn` / 0 |
| v2 | 撤除后的新会话 | `49098ca9-cdf6-4366-b8ca-2391a1bb5b10` | 0 / 0 | `end_turn` / 0 |
| 原版 | 无挂载 | `c1975aef-1544-4e5c-bc82-8cda6b4b9ac0` | 0 / 0 | `end_turn` / -15 |
| 原版 | 挂载未受信 | `4ef6f92a-b7dd-48d6-9268-25d91ce0e2e3` | 0 / 0 | `end_turn` / -15 |
| 原版 | 撤除后的新会话 | `a5561cb2-1c1e-4514-a3bc-d2e6c00844b6` | 0 / 0 | `end_turn` / 0 |

两个 `-15` 来自探针收到 ACP 终态后关闭 stdin、等待 3 秒并向仍存活的进程发送 SIGTERM。两个未受信挂载轮都收到 Gemini 的目录未受信诊断，固定 bundle 内的 `startConfiguredMcpServers` 在枚举服务前检查 `isTrustedFolder()`。最早缺失的证据是 MCP 握手，因而没有 MCP 调用 ID 或审批结果。撤除轮的零调用属于未受信对照。

v2 在模型请求等待中收到 `session/cancel`，会话 `eba42831-c1a5-4052-9f05-da6d1f22d2d5` 返回 `cancelled`，零工具事件、零服务请求，进程退出 0。另一次无挂载会话 `c5f89a8c-297d-4787-9564-0fc756efd510` 完成同 ID 的原生读和续轮后，同一进程 `session/load` 成功，回读 `default` 及 `gemini-2.5-flash`，退出 0。挂载加载和跨进程恢复待验证，初始化未声明独立 resume 能力。

## 历史证据与待验证项

此前的 v2 已获当次目录信任，真实网关挂载轮有 57 次目标调用和一次官方主题调用，前后零目标事件。历史回环也证实 v2 的单次允许、拒绝及撤除。那些临时信任条目已按当次流程清理，本轮缺可用的已授权信任目录。获得精确的合成夹具目录授权后，可在私有信任文件下对同一目录执行 v2 与原包的受信对照，继续核对授权服务和工具元数据、拒绝时零 `tools/call`、撤除后新会话及加载行为。当前未发现适配器无意裁剪的证据，未修改代码或权限策略，未运行 Cargo。探针和脱敏结果保存在本机私有目录 `shiju-gemini-task15.4seJ5eVf`。[历史验收](../../mcp-acceptance/gemini.md#正式-v2-网关复验)
