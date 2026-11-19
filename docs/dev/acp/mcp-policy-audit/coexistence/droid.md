# Droid 官方工具与 MCP 共存复验

2026-11-19，Darwin arm64 的受管 Droid 1.9.0 v2 直接 ACP 复验通过。沿用当前适配器与权限实现。[共存验收](../../ACP验收.md) · [分组审计](../6.md#droid) · [历史验收](../../mcp-acceptance/droid.md)

## 入口与权限

- 注册 key 为 `droid`，命令为 `droid exec --output-format acp`。私有副本取自受管 `droid-1.9.0-model-source-v2/droid`，`--version` 回读 `1.9.0`，`codesign --verify` 退出 0，SHA-256 为 `0858a4f7411b0159780246598fc6170a17d695556e4da66ab939bff82b58050b`，与当前合约一致。[注册](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs) · [合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json)
- 按当前模型驱动参数追加私有 `--settings`、`--model custom:Session-Selected-0` 和同值 `--spec-model`。保留 Airgap、私有 `HOME` 和 `FACTORY_HOME_OVERRIDE`、禁用 keyring 和动态配置，清空代理变量并保留回环例外。关闭 hooks、云同步和 IDE 自动连接。ACP v1 回读选定模型，实际请求进入私有回环 `/v1/chat/completions`，模型名为 `fixture-model`。每次新连接回读原生 `auto-medium`，保留该模式。原生 Read 自动获准，MCP 挂载轮经 ACP `allow_once` 才执行。未传入跳过审批参数。[驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/droid.rs)
- 使用私有空工作区和工作区外附件，每轮重置两个随机标记。ACP 的 `mcpServers` 依次传空表、单个 stdio 服务 `coexist_probe`、空表。服务复用仓库验收脚本的真实只读处理函数，并记录 MCP JSON-RPC 方法、调用 ID 与审计增量。模型夹具只决定调用，结果由两个执行器读取。原始探针与日志保存在本机 0700 私有目录 `shiju-droid-task14.vQn8Ns/verified`。[stdio 验收夹具](../../../../../scripts/acp_mount_integration.py)

## 三轮直接 ACP

成功工具均从 ACP `pending` 到 `completed`。表内 ID 为 Droid 归一化后的调用 ID，同一 ID 出现在 ACP 更新、Chat Completions 续轮的助手工具调用和对应的 `tool_call_id`。挂载轮最终续轮同时收到两个本轮标记。

| 轮次 | 新 ACP 会话（前缀） | 工具、执行方与调用 ID | 服务执行数 | 终态 / 进程退出 |
| --- | --- | --- | --- | --- |
| 无挂载 | `8c12263f` | `Read`，Droid，`call_c3145870403dac63_0` | 0 | `end_turn` / 0 |
| 挂载 | `8d2ff240` | `Read`，Droid，`call_1dec65c246ff6314_0`；`coexist_probe___read_probe`，stdio 服务，`call_1007347a422096b8_0` | 1 | `end_turn` / 0 |
| 撤除 | `ee6a0043` | `Read`，Droid，`call_8cebe1ea80f19f39_0` | 0 | `end_turn` / 0 |

挂载轮实际发生 MCP `initialize`（RPC ID `0`）、`tools/list`（`1`）与 `tools/call read_probe`（`2`）。两个对照轮均无 MCP 握手、枚举、调用或返回。挂载首个带工具的模型请求直列 `coexist_probe___read_probe`，两次对照无该项。挂载前后 15 个官方工具名称一致，包括 `Read` 与 `ToolSearch`。补丁只让 MCP 越过延迟目录，非 MCP 仍走原发现逻辑。提醒中的限制性枚举只出现在模型消息中，清单之外的官方 `Read` 实际完成，并且 MCP 仍需审批。保留当前提醒和补丁。[Droid 补丁](../../../../../patches/acp-model-source/droid/apply.py)

## 拒绝、取消与恢复

- 独立拒绝轮中，Droid 的 `Read`（`call_5d34cd4b6331f2f9_0`）完成并进入续轮。宿主对 MCP `call_590b0eedadfe908c_0` 回 `reject_once`，ACP 报 `failed`，服务只握手和列目录，执行为零。提示终态 `cancelled`，进程退出 0。
- 独立取消轮在首个带工具目录的模型请求尚未响应时发送 `session/cancel`。提示终态 `cancelled`，服务调用为零，进程退出 0。
- `initialize` 声明支持 load 与 resume。每次先关闭并等候上一进程退出，再从新进程使用挂载轮原 ACP 会话 ID 和当前 `mcpServers` 恢复。四次提示均为 `end_turn`，进程均退出 0。以下只列本轮新调用，历史工具结果仍可随会话重放。

| 恢复入口 | 新 Droid Read ID | 新 stdio MCP ID | 服务执行数 |
| --- | --- | --- | --- |
| `session/load` 重传 | `call_49cc43c5020a7523_0` | `call_3a7dc75502659eed_0` | 1 |
| `session/load` 空表 | `call_c8d5662131fc55d1_0` | 无 | 0 |
| `session/resume` 重传 | `call_b25373eb2949c8bb_0` | `call_da3b84d467430b56_0` | 1 |
| `session/resume` 空表 | `call_d464d7386edd202b_0` | 无 | 0 |

恢复重传时新 Read 和新 MCP 结果均按同 ID 进入续轮。空表恢复后当前目录无目标工具，目标服务无新请求，Read 返回本轮标记。

## 证据边界

- 当前合约在 v2 缺失时才回退受审 v1，并使用供应商摘要 `b62ce8de0ed7ffaada1abb2a017fe182021003694e85fe7f3b1136ad43628a5d`。v2 存在但摘要不符时拒绝启动。原版 1.9.0 的 Airgap/BYOK 选模和 ToolSearch 延迟发现、单次审批与拒绝沿用历史实测。本轮使用 v2。macOS Intel、Linux、Windows 的原版入口仍待实机复验。[回退实现](../../../../../crates/libs/acp-agent-registry/src/managed_runtime.rs) · [历史原版边界](../../mcp-acceptance/droid.md#原生边界与权限)
- 当前直接 ACP 证据覆盖 Darwin arm64、Chat Completions 回环和 stdio。网关绑定、当前租户 `full_access` 门禁、会话与服务名单投影及 HTTP/SSE 执行未在本轮复跑。宿主名单验证的首个缺证点是网关有效服务投影，本轮从原生 `session/new` 开始。历史网关真实模型证据见原验收。未运行 Cargo 或桌面编译。[历史网关验收](../../mcp-acceptance/droid.md#新版共存标准验收)
