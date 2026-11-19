# VT Code 0.183.0 原生工具与 MCP 共存回执

2026-11-19，macOS 27.0 arm64。受管网关的无挂载、挂载和撤除三轮均完成。挂载轮在同一 ACP 提示内执行原生 `list_files` 和 RustCodeGraph `rustcodegraph_query`，回环模型按原调用 ID 收到两项结果。挂载配合 `DenyAll` 时，网关拒绝请求。此次复验使用确定性回环模型和私有工作区，历史真实模型结果另见原验收。（[分组审计](../6.md#vtcode) · [历史验收](../../mcp-acceptance/vtcode.md#受管网关三轮)）

## 制品和权限

- 注册 key 为 `vtcode`，通用入口为 `vtcode acp`，子进程环境包含 `VT_ACP_ENABLED=1` 与 `VT_ACP_ZED_ENABLED=1`。受管调用实际使用 `/usr/bin/sandbox-exec -f <私有策略> <受管 vtcode> --config <私有 TOML> acp`。沙箱禁止子进程，VT Code 只能访问模型和 MCP 共用的回环端口。将原生写入范围收窄至私有工作区内的空目录。三轮使用受管 `session_model`、工具可用的回环 Chat Completions 模型和私有租户的 `full_access=true` 请求，拒绝轮显式传 `acp_permission_mode=deny-all`、`full_access=false`。原生独立探针仅逐次批准获准的只读调用。（[注册表](../../../../../crates/libs/acp-agent-registry/src/agent_registry.rs#L279-L288) · [受管启动](../../../../../crates/libs/acp-client/src/client/process.rs#L215)）
- 在原私有验收构建目录找到与 ShiJu 同级的 `vtcode-acp`。复制到本次私有网关的同级路径后，从 `POST /v1/desktop/acp-install/vtcode` 安装。相邻文件及安装后的 `config/acp-runtime/vtcode-0.183.0-model-source-v2/vtcode` 的 SHA-256 均为 `8b8f6c5204e1125b20d63c7c9637b027821b22d206517ebd00db5f513890cdfb`，匹配仓库合约。复用的 ShiJu SHA-256 为 `1d37a1d46ad916b7671e7d1cb9c2be65d0cf951e7e7fd516b32d7c03afa91ada`。本机 `/Applications/ShiJu.app/Contents/MacOS` 和当前 PATH 上的 ShiJu 同级均未找到该配套文件。此次未复验 Intel Mac、Linux、Windows 或手装通用入口。（[安装合约](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/contracts.json#L792) · [安装器](../../../../../crates/libs/acp-agent-registry/src/managed_runtime/install.py#L261)）
- 另一私有网关仅缓存受审 v1 原包，SHA-256 为 `88cbc99b8aac0c3a9b8596976c610026aebef14d89f9943735c38016a44919a4`。空挂载使用私有 TOML 选定回环模型，`call_before_read` 由 VT Code 完成并按 ID 续轮，终态 `chat.done`。挂载请求在模型调用前以 `VT Code MCP requires the reviewed patched runtime` 拒绝。直接运行同一原包则观察到历史断点，两个空挂载对照成功，原版允许 `session/new` 携带列表，但首个模型请求缺少 MCP 定义，探针退出 1。本次复用摘要匹配的原包缓存，未重跑下载回退。
- 宿主中继允许 `read_file`、`list_files`、`code_search` 和 `apply_patch`。当前受管启动的实际目录在两组对照均为后三项，挂载后保留这三项并追加服务名单允许的 `mcp__ruejs_graph_probe__rustcodegraph_query`。原生 VT Code 仅在客户端声明 `fs.readTextFile` 时公布 `read_file`，受管客户端沿现有策略关闭该宿主回调。三轮目录差异符合现有权限边界，未观察到挂载引入的裁剪。（[能力协商](../../../../../crates/libs/acp-client/src/client/protocol.rs#L13) · [模型中继](../../../../../crates/libs/model-relay-http/src/server.rs#L514)）

## 受管回环三轮与拒绝

私有 RueJS 副本及其索引取自既有固定验收样本。后端是实际运行的 RustCodeGraph 1.2.10 stdio 服务，宿主以服务级名单仅放行 `rustcodegraph_query`，再投影至已授权的回环 HTTP。挂载轮先收到 `list_files` 的 `app/app.tsx`，再查询 `createRueSiteApp` 并收到 `app/app.tsx:32`。回环模型断言两个结果均匹配各自的调用 ID 后才结束。

| 轮次 | 网关新会话 | 工具、执行方及调用 ID | HTTP / curl / SSE |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f2dbd1ae001DW5IwWHkBmbXV5` | VT Code `list_files`，`call_before_read`，`completed`。无目标服务配置或调用 | 200 / 0 / `chat.done` |
| 挂载 | `ses_1f2dbf0ec001i64Pb4hfL1GkEs` | VT Code `list_files`，`call_mounted_read`，`completed`。RustCodeGraph `rustcodegraph_query`，`call_mounted_graph`，`completed` | 200 / 0 / `chat.done` |
| DenyAll | `ses_1f2dc10d60018hhRs6KDmfjQnc` | 无工具事件和模型请求。拒绝原因 `VT Code MCP requires a new managed session with tools enabled` | 200 / 0 / `chat.error` |
| 撤除 | `ses_1f2dc2372001ebMYdUvfeXtg7g` | VT Code `list_files`，`call_removed_read`，`completed`。已移除 `.vwacprc.json`，无目标调用 | 200 / 0 / `chat.done` |

两个空挂载对照各有两次模型请求，挂载轮三次。直接 stdio 路径取得成功结果，网关轨迹记录完整的 ACP 工具状态及模型续轮。额外的 stdio 记录包装启动时连接关闭，故网关本轮缺少独立 stdio JSON-RPC ID 和 DenyAll 轮服务生命周期完整记录。原生探针独立观察到拒绝与撤除的服务调用数为零。网关另外启动了内置标题生成分支，标题结果不计入回环验收，未给该分支注入外部凭据。

## 原生 ACP 权限和生命周期

复用仓库 `probe.py`、摘要匹配的 v2 和同一图谱样本，四个场景退出 0。各轮独立启动 ACP 进程，提示终态均为 `end_turn`。

| 轮次 | 工具、执行方和 ID | 工具结果及模型请求数 |
| --- | --- | --- |
| 无挂载 | 无 | 零服务调用，1 次 |
| 单次批准 | VT Code `list_files` / `probe_0`。RustCodeGraph `rustcodegraph_query` / `probe_1`、`rustcodegraph_callers` / `probe_2`、`rustcodegraph_node` / `probe_3` | 四项 `completed`，三项服务执行，5 次，每次续轮核对原调用 ID 和输出 |
| 单次拒绝 | 请求 `rustcodegraph_query` / `probe_0` | `failed`，零服务调用，2 次，同 ID 回传 `permission denied` |
| 撤除 | 无 | 零服务调用，1 次 |

等待首个 MCP 审批时发送 `session/cancel`，终态 `cancelled`，服务零调用。对已挂载会话发送 `session/load` 和 `session/resume`，即使请求服务列表为空，均返回 `-32602` 并要求新会话。原生回环桥在有挂载时对错误令牌返回 401、对已消耗授权的重放返回 403。模型调用参数和单次审批由原生探针核对，实际受管桥的服务级名单只验证了允许单项的路径。直接 ACP Graph 进程退出 0，探针收回原生进程时退出 -15，私有网关退出 0。

## 复验入口

原生运行 `python3 patches/acp-model-source/vtcode/probe.py <已校验的 vtcode-acp> <rustcodegraph> <已建索引的 RueJS 私有副本>`。本机私有证据位于 `/private/tmp/shiju-vtcode-task38.9TlPum`，`gateway-direct` 保存本次受管 v2 的 SSE 和模型续轮摘要，`gateway-v1` 保存原包对照，`lifecycle.log` 保存原生取消及加载和恢复记录。私有网关均已停止并清理临时授权，未运行 Cargo、桌面打包或真实上游模型三轮。
