# CodeBuddy 动态挂载 RustCodeGraph

2026-11-19 使用 [可复用的验收请求](../ACP验收.md#可复用的验收请求) 的 `189d22dc1` 版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。各轮使用租户 `10001 / 100010005`、`acp-codebuddy / codebuddy`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

私有回环网关复用现有 shiju 0.6.0 构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。使用开发数据的私有 SQLite 备份及同一授权，仅在私有网关启用内置 CodeBuddy 预设并指向受管安装的副本。Darwin arm64 上的入口自报 `2.179.3`，三个发行 JS 文件及入口与仓库的固定摘要一致。诊断 `initialize` 返回 ACP v1、`loadSession=true` 和 MCP HTTP、SSE 支持。真实请求使用原 RustCodeGraph 1.2.10 的 stdio 与 `--no-watch`。

RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出到私有目录，索引 3478 个文件，命中 `createRueSiteApp`（`app/app.tsx:32`）。挂载轮仅在副本中启用 `ruejs_graph_probe`，第三轮前撤除。挂载期间观察到 CodeBuddy 进程启动原 RustCodeGraph stdio 子进程。下文以 SSE 同一调用 ID 下的工具参数与输出核对挂载结果。

独立 ACP 探针也用同一受管入口向 `session/new` 发送含一个 stdio 服务的 `params.mcpServers`，得到新原生会话 ID。探针只验证握手及建会话。正式轮次的推理均使用指定阿里模型。宿主 `new_session_info` 将当前 MCP 选择传入原生 `session/new`。CodeBuddy 模型驱动仍使用隔离的配置目录和会话级回环模型中继。

## 三轮结果

首个不符合合同的边界是无挂载轮的模型工具选择。没有目标 MCP 调用，但 CodeBuddy 执行了内置 `ToolSearch` 三次和 `WaitForMcpServers` 一次。可见答复包含 `MCP_NOT_AVAILABLE` 及额外解释与建议，未满足只返回标记的要求。

| 轮次 | 新会话 | HTTP / curl | 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef92710c001OhVNsqNxQF7sr7` | 200 / 0，25.744 秒 | `chat.done` |
| 挂载 | `ses_1ef92d633001DITOWTbnovqqC6` | 200 / 28，300.001 秒；续传 200 / 0，194.740 秒 | 续传收到 `chat.done` |
| 撤除 | `ses_1ef9a653f001bsRSPNVdlXU5Rr` | 200 / 0，24.751 秒 | `chat.done` |

挂载轮按原流请求 ID 和 `Last-Event-ID: 2543` 从 2544 继续，没有重复提交请求。三轮均无 `chat.error`。网关日志另有会话标题生成错误，主推理仍返回终态及答复。撤除轮没有目标工具调用，再次执行三个内置搜索及一次就绪检查，可见答复包含标记和额外解释。

| 工具 | 代表性 SSE 调用 ID | 实际输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_88def52b395d4013bd059546` | 找到 `createRueSiteApp`（`app/app.tsx:32`） |
| `rustcodegraph_callers` | `call_5531a2da9a9f481db66dc27e` | 返回调用者 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | `call_aaa2b84931864ffe941511ce` | 返回 `app/app.tsx`，含浏览器挂载条件、容器判空、清空及挂载 |
| `rustcodegraph_node` | `call_6a0b97dea7d94bd3b5507b22` | 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）及对 `_$createApp` 的转调 |

这些调用均有开始和完成事件。目标工具通过 `DeferExecuteTool` 调用，SSE 的结构化参数带完整 `mcp__ruejs_graph_probe__rustcodegraph_*` 名称。完成事件包含 query 41 次、callers 10 次和 node 76 次。node 的两次完成事件在输出中报告文件不存在，涉及 `app/router.ts` 和 `app/router.tsx`。同一服务另收到 status 1 次、files 10 次、explore 7 次，CodeBuddy 内置 `ToolSearch` 另完成 2 次。未观察到 shell、写入或其他 MCP 服务调用。

挂载答复的主链、浏览器判定、`#app` 缺失抛错、挂载阶段安装路由与 i18n、`/rust-layers` 条件绕过布局均可回对源码。路由表没有注册该路径，答复也指出顶层调用与部分符号存在图谱缺边或漏索引。移除清空逻辑的主要推断成立，运行时会把根块追加到已有快照后。答复提出产物冷启动和预填容器的二次快照验证，均未实际执行。第二个场景的 `article.prose` 选择器与 `GuideDocDetail.tsx` 实现不符，应使用源码中的 `#doc-body` 定位。产物重复 DOM 的推断适用于加载完整客户端入口的页面。

三轮的 UI summary 均回读绑定 `acp-codebuddy / codebuddy`、`full_access=true`。ACP options 在运行中及结束后均保留指定模型和 `session_model`，连接在每轮结束后释放。运行中报告 `access_status.support=partial`。挂载期间读取权限队列为空，三轮没有人工审批回复。独立 ACP 探针读到默认模式为 `default`，公开 `bypassPermissions` 和 `fullAccess` 两个不同选项。宿主策略按 `bypassPermissions` 处理该访问级别，本次未调用可能触发危险命令检查的工具。

## 原生配置复验

仅在私有租户配置中设置 `CODEBUDDY_DEFER_TOOL_LOADING=0` 与 `CODEBUDDY_WAIT_FOR_MCP_SERVERS_ENABLED=0`，用相同请求重跑。SSE 的工具标题直接显示原生 MCP 工具名。此配置不改模型、挂载范围或权限。

| 轮次 | 新会话 | HTTP / curl | 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef9acc55001WpBl8nHRqo2uCq` | 200 / 0，11.843 秒 | `chat.done`，可见答复仅 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1ef9b4cef001Dz2yBMqKQlENdj` | 四段均 200，curl 依次为 28、28、28、0 | 同一请求续传收到 `chat.done` |
| 撤除 | `ses_1efa91240001TbZoCuQ6uZzuU0` | 200 / 0，12.545 秒 | `chat.done`，可见答复仅 `MCP_NOT_AVAILABLE`，零工具事件 |

复验均无 `chat.error`。挂载轮从原请求游标续传三次，最后一次耗时 0.003 秒。三种目标工具均成功，query 171 次、callers 10 次、node 53 次。同一 MCP 另运行 files 6 次、explore 41 次、status 和 callees 各 1 次。仍能核对主调用链和清空快照的推断。答复中的导航和布局计数预测仍需在实际加载完整客户端入口的静态页上验证。复验也回读到相同绑定、模型及运行中的 `partial` 权限状态，挂载时读取权限队列为空。

## 结论与收尾

原生动态挂载和撤除隔离已有真实证据。严格合同受额外工具调用阻塞，复验配置也未解决挂载轮的工具范围偏离，故未写入代码默认值。未改驱动或受管补丁，无转发故障需要修复，未运行 Cargo。

副本撤除了临时 MCP 配置，与归档的文件和符号链接比较无差异（索引除外）。私有网关和属于本任务的 MCP 子进程均已退出，端口已关闭，凭据副本已清理。现有用户网关继续运行，读回 CodeBuddy 预设仍停用。脱敏记录之外的原始 SSE 与诊断日志仅保存在本机 0700 目录。
