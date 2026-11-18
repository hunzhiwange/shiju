# DimCode 动态挂载 RustCodeGraph

2026-11-19，验证固定版 DimCode 0.5.28 的 ACP 会话。宿主明确拒绝 stdio 挂载，原生 Streamable HTTP 路径完成了三种真实只读工具调用。

## 环境与请求

- Darwin arm64 私有回环网关复用已验收的 ShiJu 0.6.0 构建，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。使用现有固定 npm 发行版的 `dim acp`，原生二进制 SHA-256 为 `0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0`。在隔离 HOME 单独握手读到 ACP v1、`loadSession=true`、`http=true` 和 `sse=true`。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本，以 RustCodeGraph 1.2.10 索引 3478 个文件。`createRueSiteApp` 位于 `app/app.tsx:32`。
- 请求复用 `189d22dc1` 中的提示，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户为 `10001 / 100010005`，使用 `acp-dimcode / dimcode`、`session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `full_access=true`。每轮通过同一 `curl` 请求另建会话。[复用请求](../ACP验收.md#可复用的验收请求)

## stdio 三轮

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efcd6c480016DgDGwaY6oWoZw` | 200 / 0 / 6.480 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efcd85de001f81asB1dXYxMSr` | 200 / 0 / 1.357 | `chat.error`，零工具事件 |
| 撤除 | `ses_1efcd8bb40012q8b2kKpnnFDxo` | 200 / 0 / 5.896 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

挂载轮收到一个 `chat.model_recovery`（`phase=stopped`）及一个 `chat.error`，原因为 `DimCode does not support ACP stdio MCP; select an HTTP or SSE server`。`validate_mcp_contract` 在发出 `session/new` 前拒绝该传输。UI summary 保留请求绑定和 `full_access=true`，options 无连接和选模结果。对照轮运行中选模回读匹配，权限为 `client_only`，终态释放连接。

## Streamable HTTP

采用仓库已有 Copilot 验收路径，在私有目录使用 Supergateway 4.2.2，将回环 Streamable HTTP 代理到同一个实际 RustCodeGraph stdio 进程，仅公开 query、callers 和 node。桥接只使用自己的短期 Bearer，匿名请求返回 HTTP 401。桥接进程使用独立 HOME，环境只包含 HOME 与 PATH。上游模型凭据用于网关，DimCode 使用现有请求私有模型中继。首次配置将 `headers` 写成对象，HTTP 400 发生在工作区配置解析，修正为 `name/value` 数组后发出后续请求。[Copilot 的桥接验收](copilot.md#stdio-拒绝与-http-路径) · [Supergateway 文档](https://github.com/supercorp-ai/supergateway/blob/main/README.md)

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efcff0e00019YurDLNE21WcCG` | 200 / 0 / 5.733 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efd007b3001UnHOwfmo4wLSdj` | 首段 200 / 28 / 300.003，续传 200 / 0 / 88.909 | 原请求及游标续传后收到一个 `chat.done`，无 `chat.error` |
| 撤除 | `ses_1efd5f8fe001zedYK6rNqserkm` | 200 / 0 / 5.405 | 桥接仍在线，`chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

三轮运行中回读 `acp-dimcode / dimcode`、`full_access=true`、`connected=true`、`support=client_only` 与指定模型，终态连接释放且模型绑定仍可回读。既有 `dimcode.rs` 和 `dimcode_access.rs` 分别配置会话私有模型及同步受宿主约束的权限档位。HTTP 挂载轮的 178 个 MCP 调用均有 `Permission allowed once` 事件。

| 工具 | 首轮代表调用 ID | 回执 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_f78dd42027374f9490c471d4` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_b70203339216406b9f75ffc1` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_247a8d7a71dd42f0a058ad23` | `packages/runtime/src/hooks/useApp.ts:4` 的 `useApp` 将根工厂传给 `_$createApp` |

首轮完成回执含 query 120 次、callers 2 次、node 56 次、原生 `grep` 18 次。桥接原始结果有 5 次 `isError=true`，其中 2 次尝试读取不存在的文件，3 次把子包路径当成单独索引项目。DimCode 将这 5 次报告为 `completed`，回执正文仍可识别错误。挂载闭环已证实，首轮违背了只使用目标 MCP 和无工具失败两项要求。

首轮答复的主调用链、浏览器与 SSR 分支、`#app` 缺失处理、插件安装顺序和 `/rust-layers` 的布局分支与源码相符。快照残留与提前就绪属于源码推断。答复给出的第二个场景只描述客户端入口写入内容，需用实际 `mountRueSiteApp` 路径才可检验清空逻辑。两项场景均未运行。

从 RueJS 副本目录启动桥接后重新挂载，会话 `ses_1efd619710012ExtVPzgHswn9n` 首段 HTTP 200、curl 28、300.010 秒。随后使用原请求标识及游标续传，HTTP 200，仍有目标工具结果。两段共完成 query 115 次、callers 2 次、node 84 次，桥接记录 15 次 `isError=true`。也出现了 `glob`、`skill` 与 `grep`。确认复跑已违反合同后关闭私有网关，续传以 curl 18 结束，未收到 `chat.done` 或 `chat.error`。该复跑未再发撤除轮，撤除证据来自首组已完成的 HTTP 三轮。

## 结论与验证

验收已阻塞。DimCode 0.5.28 可经原生 Streamable HTTP 发现、执行并读回三种 RustCodeGraph 工具。两次挂载均有意外工具调用及失败结果。保留 stdio 拒绝、现有审批行为及测试断言。复验须用同一模型和提示，在 HTTP 路径上取得三种真实工具结果、合规答复及零失败终态。

定向命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_dimcode acp_onboarding_dimcode_rejects_stdio_mcp_instead_of_silently_ignoring_it` 使用槽位 3，两个缓存路径均为仓库的 `target/rust-slot-3`。检测到同仓 rust-analyzer 发起的 workspace Cargo 检查仍在编译后，为保持重型编译串行，终止自有槽位入口，退出 130，执行零项测试。复跑前发现另一个 Cargo 编译已启动。已确认自有编译子进程退出、槽位锁释放，保留缓存。真实 stdio 三轮提供了独立门禁证据。

私有网关、桥接和监听端口已清理。已删除私有网关上游授权副本、一次性桥接密钥及临时 MCP 配置。RueJS 副本 4109 个 Git blob 与源提交一致，原工作区和用户全局配置未写入。原网关健康检查仍为 HTTP 200。原始日志保存在本机 0700 目录。
