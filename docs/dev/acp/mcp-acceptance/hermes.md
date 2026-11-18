# Hermes Agent 动态挂载 MCP 验收

2026-11-19。Hermes 动态挂载和撤除已实测。严格验收阻塞于额外工具调用及答复中的验证场景。受管安装已补齐原生 MCP SDK。

## 环境与首个失败边界

- 在独立回环网关复用 shiju 0.6.0，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。仅在隔离数据副本启用 `acp-hermes`。最初三轮创建了宿主会话，HTTP 200、curl 0，均以 `chat.error` 结束，报 `ACP session_model requires Provider credentials`。将既有阿里授权文件中对应提供商条目加入隔离网关后复跑，原模型可用。
- 使用 [可复用的验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。保持租户 `10001 / 100010005`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model`、`full_access=true` 和服务名 `ruejs_graph_probe`。RueJS 使用 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的干净副本。RustCodeGraph 1.2.10 索引 3478 个文件。
- Hermes 自报 0.19.0，Darwin arm64、Python 3.12.16。受管 PyPI wheel SHA-256 为 `bd0bac012aee38a60894781f4597dc29ee7bedb3448540249921f10d3bef327f`。独立 ACP 探针返回协议 v1，声明 load、resume、fork 和 list，没有声明 HTTP 或 SSE MCP。实际选择原生 stdio，加 `--no-watch`。探针的 `session/new` 发出非空 `mcpServers` 并收到新原生会话 ID。真实网关轮由宿主解析工作区挂载，未开启原始 ACP 线包日志。
- 修复授权后，原受管安装的三轮都有 `chat.done`、零 `chat.error`、零工具事件。挂载会话 `ses_1efad001f001H9Kcv4qGwsm7ld` 在宿主侧已初始化 RustCodeGraph，但模型仍报告 MCP 不可用。独立探针进入 Hermes 的注册与工具列表刷新路径，只得到 15 个内置工具。其 Python 环境没有 `mcp`，`tools.mcp_tool._MCP_AVAILABLE` 为 false，原生 `register_mcp_servers` 在连接前提前返回。这解释了挂载定义已被接收却没有进入模型工具列表。
- 将 Hermes 发行安装从 `[acp,cli]` 改为 `[acp,cli,mcp]`。发行元数据已固定 SDK 为 1.26.0。使用独立的受管 v2 安装路径识别旧发行，并校验 SDK 入口摘要。实际执行仓库安装脚本冷安装 v2，退出 0，十个契约文件的摘要相符。用冷安装入口重复独立 ACP 探针，原生注册 8 个 RustCodeGraph 工具，挂载会话刷新后有 23 个工具。Hermes 按所选会话工具集暴露工具，原生连接注册表按进程保存。本轮实测采用独立 ACP 进程和私有 `HERMES_HOME`，撤除后通过新会话复验。
- 模型驱动继续使用每次请求生成的私有 `HERMES_HOME` 和模型回环中继。仅网关使用阿里授权，探针用本地占位配置。挂载通过原生 `session/new`，因此无需把工作区 MCP 写进用户或私有 Hermes 配置文件。未修改恢复、全访问校验或审批路径。

## SDK 补齐后的真实三轮

真实模型复验使用旧网关已校验的 Hermes 入口，在其私有发行副本安装与冷安装相同的 SDK 1.26.0。该复验与冷安装探针分别记录。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 与可见答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efaf3d0d001isCzrQU2OFD4Rf` | 200 / 0 / 7.128 | `chat.done`，去除推理标签后仅 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efaf5919001UvhxXAQQuJfYCn` | 200 / 0 / 164.238 | `chat.done`，48 次工具调用均配对记录开始和完成 |
| 撤除 | `ses_1efb1dadd001qFiwRbB8YJI1gu` | 200 / 0 / 6.579 | `chat.done`，去除推理标签后仅 `MCP_NOT_AVAILABLE`，零工具事件 |

三轮各一条 `chat.done`、零 `chat.error`。对照轮的原始 `chat.delta` 仍含推理标签及内容。挂载轮 48 次调用全部来自 `ruejs_graph_probe`。query 5 次、callers 4 次、node 30 次，另调用 files 4 次、explore 4 次和 status 1 次。未观察到其他服务、内置工具或 shell 调用。使用同一证据断言验证目标三类工具有配对及输出、对照无调用，原安装失败，补齐 SDK 后通过。严格合同仍拒绝挂载轮的额外图谱工具。

| 目标工具 | 代表性 SSE 调用 ID | 实际输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `tc-c92f91b4dfe9` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `tc-36532217eb50` | 调用者 `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `tc-374343bf62ae` | 返回入口实现，含 `#app` 判空、清空、挂载和浏览器守卫 |
| `rustcodegraph_node` | `tc-90c04d645f19` | `packages/runtime/src/hooks/useApp.ts:4` 转调 `_$createApp` |
| `rustcodegraph_node` | `tc-bb21d5f977a1` | `packages/runtime/src/compiler-runtime/app.ts:57` 的应用工厂及插件安装 |

答复的主链、浏览器与 SSR 入口、容器缺失抛错、插件在根组件前安装及 `/rust-layers` 布局条件均可对回源码。移除 `replaceChildren()` 后，完整客户端入口会把新根块追加到现有快照，属于源码推断。答复给了静态文档页 DOM 唯一性和客户端布局绕过两项验证设想，两项均未运行。第二项涉及的 `/rust-layers` 在当前路由表无注册，且需在初始 `#app` 中预放旧快照并加载完整客户端入口，答复未满足可直接执行的条件。答复对所有文档入口都会重复挂载的预测也超出了证据范围。

## 权限和收尾

三轮 `ui/summary` 均为 HTTP 200，回读 `acp-hermes / hermes` 及 `full_access=true`。`acp-options` 回读原模型和来源 `session_model`。挂载轮进行中为 `connected=true`、`access_status.support=partial`，权限队列为空。原生探针初始模式为 `default`。宿主分别处理 Hermes 编辑审批和终端权限，本次实际只调用了只读图谱工具，未实测编辑或终端审批。三轮结束后各自释放 ACP 连接。网关另有标题生成模型的 HTTP 403，三轮使用的会话模型均返回正文和终态。

仓库的公共转发与会话驱动未改，未运行 llm-runtime 定向测试或 Cargo 构建。验证覆盖 installer 冷安装、契约文件校验、独立 ACP 探针、真实模型对照和 `git diff --check`。私有网关及其 RustCodeGraph 子进程已退出，端口关闭，凭据及租户数据库副本已清理。RueJS 副本源码及符号链接与归档一致，临时挂载配置已撤除。现有网关继续运行，其 Hermes 预设仍停用。原始事件和诊断日志保存在本机私有目录。
