# Pi 0.73.1 原生工具与 MCP 共存回执

2026-11-19 在 Darwin 27.0.0 arm64 直连受管 ACP 复验。三个独立新会话均通过。挂载轮在同一提示内实际执行 Pi 原生 `read` 和 RustCodeGraph 查询，回环模型按原 ID 接收两个真实结果并继续一轮。未发现适配器裁剪原生工具的缺陷。

## 入口和权限

- 注册 key 为 `pi`，别名 `pi-acp`。通用入口为 `pi-acp` 或 `npx -y pi-acp@^0.0.22`。原版 Pi 支持原生模型配置。原版适配器虽保存 `mcpServers`，启动原生 RPC 时未将其注册到工具目录。该断点沿用历史证据。[分组审计](../5.md#pi) · [原版验收](../../mcp-acceptance/pi.md#首个失败边界)
- 当前复验使用 Node 24.22.3，直接运行受审 `pi-acp/dist/index.js`，无参数。ACP 初始化自报适配器 0.0.22、协议 v1、`loadSession=true`，HTTP/SSE MCP 能力均为 false。引擎 `@mariozechner/pi-coding-agent/dist/cli.js --version` 返回 0.73.1。以下 SHA-256 与受管合约和引擎驱动一致。[制品合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json) · [模型驱动](../../../../../crates/libs/llm-runtime/src/acp/model_source/drivers/pi.rs)

| 文件 | SHA-256 |
| --- | --- |
| `dist/index.js` | `579df420c0a8d19f72bf34343e592174ff50d40776fda6311c04f488a44dad60` |
| `dist/shiju-mcp-extension.mjs` | `ec5c5f34a37d3d3d0767e3341420c95c57878f1abf308f531545dde5cfab8c8e` |
| `dist/shiju-mcp.py` | `ede7bf31f0eafe51f6dd0572d0cea482fe3be3eb715534b2f566d87e5c8aa915` |
| `dist/shiju-stdio.py` | `e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a` |
| 原生 Pi `dist/cli.js` | `e959f463b06ddd15ed882783ac02f39ecaeef950ef967da86380f39dda6595fa` |

- 每轮使用私有 `HOME`、`PI_ACP_STATE_DIR`、`PI_CODING_AGENT_DIR`，设置 `PI_OFFLINE=1` 和空 `SHIJU_PI_SKILL_ROOT`。由 `PI_ACP_PI_COMMAND` 指向已核摘要的引擎。制品的 spawn 路径为 `<Pi cli.js> --mode rpc --no-themes --no-skills --no-extensions`，挂载目录非空时追加 `--extension <shiju-mcp-extension.mjs>`。加载已有会话时带原生 `--session`。私有 `models.json` 与 `settings.json` 选择 `fixture/pi-mcp-probe`，回环 Chat Completions 回读模型 ID。认证使用探针占位值。
- 原生 `read` 由 Pi RPC 执行，探针未授予原生编辑或命令权限，也未请求此类操作。宿主逐次审批只覆盖 MCP 调用，桌面完全访问及宿主全拒绝门禁仍适用。MCP 扩展绑定原生调用 ID、工具名和参数，获 `allow_once` 后由 Darwin stdio 沙箱单次执行。服务使用绝对路径、空环境覆盖、私有 HOME 和只读工作区，禁止网络和派生进程。[受审补丁](../../../../../patches/acp-model-source/pi/README.md) · [桌面门禁](../../../../../crates/libs/acp-client/src/client/process.rs)
- 受管安装限 macOS arm64，挂载另依赖 Darwin 沙箱。Linux 缺本仓受审产物，Windows 私有状态路径的 ACL 校验显式拒绝。其他平台与通用入口挂载仅核对现有边界。[分组审计](../5.md#pi)

## 三轮新会话

服务为原生 RustCodeGraph 1.2.11，SHA-256 为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`。使用私有 RueJS 及 SQLite 索引副本。探针为每轮写入新的只读测试内容，随机值只存在于工作区测试文件和实际返回结果。所有轮次均保留原生 `read`、`bash`、`edit`、`write` 的声明，挂载轮另声明八个图谱工具。

| 轮次 | ACP 会话 ID | 工具名、模型与 ACP 共用 ID、实际执行方 | ACP 终态 / 探针与适配器退出 |
| --- | --- | --- | --- |
| 无挂载 | `01a1f2ca-6144-76af-8cd7-6221183219a6` | `read` / `call_native_read` / Pi RPC；目标 MCP 0 次 | `end_turn` / 0、0 |
| 挂载 | `01a1f2ca-65ea-754f-a1c5-ba490e8c4ffd` | `read` / `call_native_read` / Pi RPC；`ruejs_graph_probe__rustcodegraph_query` / `call_query` / 沙箱 RustCodeGraph stdio | `end_turn` / 0、0 |
| 撤除后新会话 | `01a1f2ca-6b75-7378-931d-0a3e44b8b482` | `read` / `call_native_read` / Pi RPC；目标 MCP 0 次 | `end_turn` / 0、0 |

各轮 `read` 均从 `pending` 到 `completed`，续轮携带匹配 ID 和文件随机值。挂载轮 `call_query` 经宿主批准，ACP 同 ID 报告 `completed`，返回 `createRueSiteApp`（`app/app.tsx:32`），第三次带工具表的模型请求包含两个原始 ID 及结果。两个对照用空 `mcpServers` 建会话，无目标服务配置、扩展目录、MCP 审批或调用，适配器在启动服务前即返回空目录。

## 拒绝、取消与加载

| 场景 | ACP 会话 ID | 调用与结果 | ACP 终态 / 探针与适配器退出 |
| --- | --- | --- | --- |
| 仅拒绝名单 | `01a1f2ca-700f-757f-b3d9-e02049dc113a` | 只拒绝 `mcp__ruejs_graph_probe__rustcodegraph_status`。模型目录移除 `status`，保留四个原生工具和其余七个 MCP 工具。Pi RPC 完成 `call_native_read`，沙箱图谱完成获准的 `call_query`，模型收到两项结果 | `end_turn` / 0、0 |
| 宿主拒绝 | `01a1f2ca-765b-7315-9ea3-f40233e2d9fc` | Pi RPC 完成 `call_native_read`。宿主拒绝 `call_query`，适配器在执行前结束该调用，ACP 标记 `failed`，续轮在同 ID 收到拒绝结果 | `end_turn` / 0、0 |
| 审批等待时取消 | `01a1f2ca-7b4d-70bb-bee9-89f283bcf884` | Pi RPC 完成 `call_native_read`。等待 `call_query` 审批时发送 `session/cancel`，未批准，ACP 同 ID 报告 `failed`，无成功 MCP 结果或后续模型请求 | `cancelled` / 0、0 |

原仓探针在当前图谱的 `useApp` 多定义摘要处退出 1。临时副本给 `node` 加入 `file=packages/runtime/src/hooks/useApp.ts` 并断言实际返回的定义位置。该副本七项通过，覆盖空挂载、批准和拒绝、撤除、`session/load` 挂载再清除、审批等待时取消，以及同一工具同时在允许与拒绝名单时拒绝优先。加载轮中 `call_query`、`call_callers`、`call_node` 经批准由真实图谱执行。同会话再次以空列表加载后，工具表回到四个原生工具，提示以 `end_turn` 结束。会话恢复能力以该制品实际声明的 `session/load` 为准。

历史正式网关的 113 次成功调用保留为真实模型证据。本次未启动网关、调用供应商模型、运行 Cargo 或执行 stdio 在途取消。宿主策略注入在直连探针中以 ACP 元数据复验。[历史正式网关验收](../../mcp-acceptance/pi.md#最终集成回归通过)

探针和脱敏轨迹存于本机私有目录 `shiju-pi-coexist.8nqIy8`。参考原仓 `patches/acp-model-source/pi/probe.py` 的回环协议。复跑用私有工作区及带 WAL 边车的图谱副本，依次设置 `PI_PROBE_STAGE=before`、`mounted`、`removed`、`reject-only`、`reject-approval`、`cancel`，每次执行 `python3 <私有目录>/probe.py <受审 ACP 入口> <RustCodeGraph> <私有工作区>`。补充探针 SHA-256 为 `08aadbdd945e3ee3c7f172d25ad6c4344ee7fc42af1eb8674c65c9fe43c6e933`。各轮私有状态、监听及子进程已退出或清理。
