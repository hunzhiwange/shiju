# Cortex Code 动态挂载 RustCodeGraph

2026-11-19，固定版原生入口忽略 `mcpServers`。ShiJu 受管 v3 已接入会话级 MCP，真实工具成功回传。指定模型的工具参数与答复仍有偏差，严格三轮验收未通过。

## 环境和请求

- Darwin arm64，私有回环网关使用此前验收的 ShiJu 0.6.0，SHA-256 为 `ac9b96bce8a3aee0cb6102dc98a9b365efdfccfe0cddc4901df7860685b56877`。通过已核验的发行归档缓存安装 Cortex Code `1.2.46+175654.6f8561fd271f`（`model-source-v2`），HTTP 200。启动器、发行程序和本地中继组合摘要均与仓库受管清单一致。ACP 握手为协议 v1，agentInfo 版本为 `1.0.0`，支持加载会话，未声明 HTTP 或 SSE MCP 能力。
- 复用 [验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。沿用租户 `10001 / 100010005`、`session_model`、`full_access=true`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。预设与后端为 `acp-cortex-code / cortex-code`。
- RueJS 为 `main` 上 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的工作区副本，含已有未提交变更，索引 3687 个文件。挂载使用名为 `ruejs_graph_probe` 的 RustCodeGraph 1.2.10 原生 stdio 服务和 `--no-watch`。宿主在挂载轮启动了 RustCodeGraph。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 答复 | `chat.tool` 数 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef93aedb0016zt6j3B10JeQn9` | 200 / 0 / 14.268 | 一个 `chat.done` | `MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1ef93e76b001swAJmq6tk2B2bL` | 200 / 0 / 12.081 | 一个 `chat.done` | `MCP_NOT_AVAILABLE` | 0 |
| 撤除 | `ses_1ef9417650014yc9jpkU9ht4Dq` | 200 / 0 / 11.822 | 一个 `chat.done` | `MCP_NOT_AVAILABLE` | 0 |

三轮均无 `chat.error`。UI summary 均确认绑定 Cortex Code 和 `full_access=true`。ACP options 终态均显示 `connected=false`，所选模型、提供商与 `session_model` 来源符合请求。无挂载和撤除答复符合合同。挂载未产生三种工具的开始、完成或失败事件，无调用结果可关联到服务名，答复也没有按合同给出调用链、分支、静态快照推断或回归场景。

## 断点及权限

- 直接向同一受管启动器发送含真实 stdio 定义的 ACP `session/new`，建会话成功，回环模型仅收到 `read`、`write`、`edit`，提示以 `end_turn` 结束。固定发行包的 `newSession` 实现只用 cwd、调用方会话 ID、连接名创建会话，并把 `_meta` 交给会话设置，未读取 `mcpServers`。把同一服务改为指向模型中继同端口的 HTTP 诊断地址，结果相同，监听器没有收到 MCP 请求。
- `--only-explicit-mcp-servers` 保持开启。固定版 `acp serve --help` 未列出 `--mcp-config`。该发行版拥有 `_meta.clientTools` 扩展，私有日志确认显式名称可以注册，但受管本地组合的声明和调用白名单只允许三个原生文件工具。ShiJu 的 ACP 客户端当前也拒绝扩展回调。基于该扩展的适配需要宿主按会话验证和执行工具、保留审批和撤销，并处理加载会话时的工具绑定。原生 stdio 子进程受现有沙箱禁止，网络仅允许绑定的模型中继端口。 [Snowflake ACP 文档](https://docs.snowflake.com/en/user-guide/cortex-code/cortex-code-acp) · [Snowflake CLI 发行记录](https://docs.snowflake.com/en/user-guide/cortex-code/changelog)
- 独立运行同一 RustCodeGraph stdio 服务，三个真实 `tools/call` 均成功。 `rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 命中 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。这些结果来自独立 MCP 连接，Cortex 没有读回。
- 独立权限探针收到一次原生 `write` 的 ACP 审批请求。选 `reject_once` 后目标文件不存在，模型续轮见到 `Denied by user`。三轮真实请求均无工具调用，无法验证 MCP 的审批行为。保持受管门禁，未修改驱动、补丁或公共协议，未运行 Cargo。

网关正常退出，端口关闭，私有租户配置副本和临时 MCP 配置已清理。保留原始 SSE、会话回读与探针在本机 0700 私有验收目录。复跑前需有支持会话级 MCP 且保持现有审批边界的受管入口。

## 受管 v3 适配

官方 stable 仍为受审的 `1.2.46+175654.6f8561fd271f`。在相同发行版上，受管启动器读取 ACP 的显式 stdio 挂载，发现工具后用 Cortex `_meta.clientTools` 注册经命名空间限定的名称。工具声明和回调同时按注册集合校验。宿主只处理当前提示中已经发出开始事件的同名同参工具，并按 ACP `allow_once` 审批后执行。挂载时模型只收到选中服务的工具，空挂载维持已审文件工具。原生沙箱、模型绑定、全局 MCP 隔离沿用。MCP 进程另用工作区只读沙箱、私有临时 HOME、禁止网络及派生子进程，并在提示结束、撤销或断开时清理。当前入口接受绝对路径、无额外环境变量的 stdio 服务。

用仓库安装脚本从已核验归档安装 v3 成功，四项受审文件摘要一致。聚焦命令 `python3 patches/acp-model-source/cortex-code/probe.py <受管入口> <RustCodeGraph> <已索引的 RueJS 副本>` 使用真实 Cortex 和 RustCodeGraph，模型端由回环夹具控制。v2 退出 1，首轮无挂载通过，挂载断言发现模型声明缺少三个目标工具。v3 退出 0，无挂载、批准挂载、拒绝挂载、撤除、加载会话重新绑定和审批期间取消共 6 个场景通过。加载同一会话时可接入工具，再以空列表加载后下一轮只收到原生文件工具。批准时三个原生调用 ID 均收到同名审批，回环续轮读回查询、调用者和节点源码。拒绝时三个续轮收到错误。审批期间取消的提示以 `cancelled` 结束，迟到的批准未执行工具或进入原生连接。独立沙箱探针可查询 RueJS，无法通过 `projectPath` 查询工作区外已有索引。

### 真实模型复验的工具范围

干净 RueJS 副本索引 3478 个文件。独立探针显示，RustCodeGraph 1.2.10 对没有 WAL 辅助文件的只读索引返回空目录。将副本切到 `DELETE` 后可以发现工具，实际查询仍因 RustCodeGraph 试图配置 SQLite 而失败。最终在验收副本用宿主持有只读事务，保留已初始化的 WAL 与 SHM 文件，同一只读 MCP 沙箱内的三个调用均成功。原工作区索引和所有沙箱策略沿用。

开放服务的八个工具时，真实会话 `ses_1f02bc30b00122D3hSRhSFOOmn` 返回 HTTP 200、curl 0、180.057 秒及 `chat.done`。三个目标工具都产生成功结果，模型也调用了 `status`、`files`，部分猜测路径的 `node` 调用失败，最终没有合同要求的完整答复。保留该轮作为开放工具面的对照。

Cortex v3 将宿主已有的 MCP `allowedTools` 和 `deniedTools` 投影到显式工具登记。受管桥在发现时过滤，回调只接受已登记的名称。旧入口运行同一探针因额外工具可见退出 1，更新后共 7 个原生场景通过，包括 deny 优先于 allow。正式复验通过同一 RustCodeGraph 服务的项目配置允许 `query`、`callers`、`node`，保留原请求体和服务名。

### 最终受管入口三轮

从当前共享工作区在槽位 3 定向构建 `vw-cli --bin shiju`，退出 0，耗时 18 分 16 秒。两个 Cargo 缓存路径均为仓库的 `target/rust-slot-3`。网关为 ShiJu 0.6.0，SHA-256 `1a033a19a5c5e2c115e90abbd336f898c0df3ac101a9824ffe8b36f11b0bcc47`。私有网关安装 HTTP 200，已安装的 `mcp_bridge.py` SHA-256 `e6870d58c6c59cb1f1d224a66cdaa8455b34057249912b7bce5b9e21f95a074a` 与清单一致。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 结果 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f0457250001Caspbqjjq1Zj7r` | 200 / 0 / 12.403 | `chat.done` | 仅 `MCP_NOT_AVAILABLE`，工具事件为零 |
| 挂载 | `ses_1f045a369001v32uvvkvpe6YAh` | 200 / 0 / 123.879 | `chat.done` | 三种目标工具共 71 个成功、2 个失败，无其他工具 |
| 撤除 | `ses_1f047880b001pRBdbIJFukgU2m` | 200 / 0 / 10.705 | `chat.done` | 工具事件为零，模型在 `MCP_NOT_AVAILABLE` 后追加说明 |

三轮均无 `chat.error`。UI summary 确认 `acp-cortex-code / cortex-code` 与 `full_access=true`。ACP options 确认 `session_model`、指定提供商和模型，终态都断开连接。挂载轮原生模型请求只声明同一服务的三个工具。真实 SSE 开始和完成事件携带工具名称和同一调用 ID。

| 工具 | 代表调用 ID | 实际结果 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_ed112fe22c27489ba0635c67` | 定位 `createRueSiteApp` 至 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_cf0252f777724277a5e304b8` | 定位调用者 `mountRueSiteApp` 至 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_b1e4cf7c771748988bb41b01` | 读回 `packages/runtime/src/hooks/useApp.ts:4` 的实现和对 `_$createApp` 的调用 |

挂载答复引用浏览器入口、SSR 条件、`#app` 缺失时的异常、路由和 i18n 安装及 `/rust-layers` 绕过布局。它指出调用图缺少源码中模块顶层对 `mountRueSiteApp` 的调用，将移除快照清理后的重复 DOM 与旧页面残留作为推断，分别给出首次启动和路由切换两个验证场景。首轮严格合同未通过，模型用 `node` 查询不存在的 `app/router.tsx` 和 `app/router.ts`，撤除轮也未遵守只回复标记的约定。

### 同一网关和原请求的第二组三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 结果 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f047f653001wtT9YLt0Z06gmV` | 200 / 0 / 11.267 | `chat.done` | 仅 `MCP_NOT_AVAILABLE`，工具事件为零 |
| 挂载 | `ses_1f04822fe001zmz984GUmpJ4Yi` | 200 / 0 / 241.319 | `chat.done` | 三种目标工具共 105 个成功、8 个失败，无其他工具 |
| 撤除 | `ses_1f04bd256001gudUpI0aeZr9mi` | 200 / 0 / 10.260 | `chat.done` | 仅 `MCP_NOT_AVAILABLE`，工具事件为零 |

回读再次确认绑定、权限与选模。第二次挂载有 61 个成功的 `query`、2 个成功的 `callers` 和 42 个成功的 `node`。另外 6 个 `node` 查询模型猜测的不存在路径，2 个 `query` 把包目录当作索引项目根。模型输出了调查意图，但未在终态前给出完整分析。两次挂载均实现动态发现、审批、调用与续轮结果回传，严格合同仍需在同一提示下由指定模型生成有效工具参数和完整答复。

两组网关均正常退出、端口关闭，私有租户副本和实验挂载文件已清理。原始回执、SSE 和原生观测保留在本机 0700 私有目录。

聚焦 Rust 验证使用 `cargo test -j 1 -p session-engine --lib acp_mcp_mount_carries_resolved_tool_policy`，经 `scripts/cargo_slot.py` 在共享槽位 6 执行，两个缓存路径均为仓库的 `target/rust-slot-6`。该夹具覆盖 Cortex 和 Pi。旧过滤名在槽位 3 执行时因共享测试已改名而匹配零项。按实际名称运行旧产物，发现夹具未启用内置预设，1 项失败。提交 `2bd74758a` 修正夹具后，同一断言 1 项通过、0 项失败，既有编译告警仍存在。适配与聚焦验证已完成，任务卡保留指定模型严格验收的外部阻塞条件。
