# Gemini CLI 动态挂载 MCP 验收

2026-11-19，已阻塞。Gemini 0.66.0 接收了独立探针的 ACP stdio 服务定义，但其工作区信任检查在注册工具前停止发现。真实模型挂载轮返回 `MCP_NOT_AVAILABLE`。保留原有信任和审批策略。

## 环境

- 网关为槽位 6 已有的 shiju 0.6.0 构建，二进制 SHA-256 为 `ac9b96bce8a3aee0cb6102dc98a9b365efdfccfe0cddc4901df7860685b56877`。本轮未重新构建。
- 受管 Gemini CLI 自报 `0.66.0`，Darwin arm64。ACP 入口文件 `bundle/gemini-EOCRFAPZ.js` 的 SHA-256 为 `ed0b9375d8b0219b3845a2fa97943e619b9b9914d73c4a61cd84f721320e3a33`，与仓库受管契约一致。
- RueJS 从 `main` 的 `14fe287b` 提取干净副本；本地工作区已有未提交修改。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `createRueSiteApp`（`app/app.tsx:32`）和 `mountRueSiteApp`（`app/app.tsx:39`）。挂载项是 `ruejs_graph_probe`，使用原生 stdio 及 `--no-watch`。
- 私有回环网关从开发 SQLite 副本读取租户 `10001 / 100010005`。健康、配置、预设及模型能力端点均为 HTTP 200；`acp-gemini` 已启用，`session_model` 受支持。请求提示来自 `189d22dc1`，核对 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | 对照结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef7de47f001H4Tk96jZUo8c6a` | 200 / 0 / 8.481 | 含 `MCP_NOT_AVAILABLE`，另有说明和一个原生 `update_topic` 调用 |
| 挂载 | `ses_1ef7e073c001f4nOV245p1kYq0` | 200 / 0 / 5.922 | 仅回复 `MCP_NOT_AVAILABLE`，SSE 工具事件为零 |
| 撤除 | `ses_1ef7e1fd8001rW0Wabsli23rqr` | 200 / 0 / 8.364 | 含 `MCP_NOT_AVAILABLE`，另有说明和一个原生 `update_topic` 调用 |

- 三轮各有一个 `chat.done`，无 `chat.error`。两个对照轮的 `update_topic` 均有开始及完成事件，且回复多于约定的单一字串，未通过严格对照。网关的独立标题生成记录到另一模型的 HTTP 403，聊天请求仍使用指定模型。
- `ui/summary` 确认 `acp-gemini / gemini`、`full_access=true`。运行时 `acp-options` 确认 `connected=true`、`access_status.support=client_only`、来源 `session_model`、provider `alibaba-token-plan-cn`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。终态保留绑定与选模，连接释放。
- 三轮均无 `rustcodegraph_query`、`rustcodegraph_callers` 或 `rustcodegraph_node` 调用与输出。因此没有模型端的调用链、行号、静态快照影响或回归情景可验收。

## 首个已确认的阻断边界

- 真实挂载轮的宿主成功初始化了 RustCodeGraph。该轮保留了 SSE 和会话读回证据，网关未采集原始 ACP 线包。
- 独立运行同版受管入口，直接发送 `initialize` 和含真实 RustCodeGraph stdio 定义的 `session/new`。初始化声明支持 HTTP 和 SSE。会话创建成功。只读观测确认该服务进入 Gemini 会话配置，`mcpEnabled=true` 且 `isTrustedFolder()=false`。随后回环模型请求的工具声明只有内置工具。回环探针使用本地占位认证及独立目录，原生 CLI 还报告拒绝加载非 root 所有的探针系统设置文件；真实模型绑定另由网关读回确认。
- 上游 `McpClientManager::startConfiguredMcpServers` 在遍历已配置服务前检查工作区信任，适用于 stdio、HTTP 和 SSE。stdio 创建还有自己的信任检查。换传输无法越过前置门禁。待受管 ACP 能应用用户明确授予的工作区信任后，使用原请求和相同模型复验。未修改 Rust、受管补丁或权限设置，未运行 Cargo 测试。

自有网关及回环端口已关闭，私有副本的临时 MCP 配置和凭据副本已清理。原 RueJS 和运行中的用户网关未改配置。私有网关启动时仍打开了开发环境的共享工作区记忆数据库，未验证其内容差异。原始日志和事件证据留在本机私有目录。

## 共享构建复验

使用共享槽位 3 已有的 shiju 0.6.0 构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。依旧使用受管 Gemini 0.66.0、同一 RueJS 提交、提示哈希、租户和指定模型。回环网关同时隔离 HOME、运行根及工作目录，核对打开的记忆 SQLite 文件位于私有目录。

| 轮次 | 新会话 | HTTP / curl / 秒 | 答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff933c4001lmv06aFS6ipo2l` | 200 / 0 / 6.664 | `MCP_NOT_AVAILABLE` 加说明 |
| 挂载 | `ses_1eff94f4a001LqMRyB413WEGPe` | 200 / 0 / 4.913 | `MCP_NOT_AVAILABLE` |
| 撤除 | `ses_1eff9643d001heYNOkbFIUTHWF` | 200 / 0 / 5.030 | `MCP_NOT_AVAILABLE` |

三轮各有一次 `chat.done`，无 `chat.error` 或 `chat.tool`。运行中与结束后读回确认原绑定、`full_access=true` 和 `session_model` 模型；连接由运行中的 `true` 转为 `false`，运行时权限仍为 `client_only`。无工具输出或源码答复可核对。另一模型的独立标题生成再次返回 HTTP 403。

复验以只读观测记录实际 Gemini 进程初始化时的三个字段。挂载轮的会话配置确有目标 stdio 服务，`mcpEnabled=true`、`isTrustedFolder()=false`。两轮对照均没有该服务。观测只记录服务是否存在、传输和信任判断，继续执行原生初始化。由此将实际请求的首个阻断边界定位到 Gemini 的工具发现。

另以空白的私有原生信任文件，分别用 HTTP 与 SSE 直接创建 ACP 会话。回环模型请求只声明内置工具，两个 MCP 路径均未收到请求。上游主线仍在枚举配置服务前检查文件夹信任，升级无法排除此门禁。[Gemini CLI MCP 发现实现](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/tools/mcp-client-manager.ts)

已核对受管 0.66.0 支持从 `GEMINI_CLI_TRUSTED_FOLDERS_PATH` 读取原生文件夹授权。本机默认信任文件不存在，私有副本也无既有授权。继续验收需要用户明确授权测试副本的准确路径，使受管进程读取私有原生信任文件。该授权覆盖文件夹级能力，应保留原生工具审批。工作区信任覆盖开关、关闭信任检查及服务器免审批均未使用。[Gemini CLI 信任文档](https://geminicli.com/docs/cli/trusted-folders/)

复验网关已退出，临时挂载及凭据副本已清理。没有涉及 Rust 公共转发改动，未运行 Cargo。保留 `已阻塞`，待明确授权后复验挂载和严格对照。
