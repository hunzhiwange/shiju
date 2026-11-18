# OpenClaw 动态挂载 RustCodeGraph

2026-11-19，已阻塞。OpenClaw 的 ACP 桥在 `session/new` 拒绝会话级 MCP。独立探针验证了私有 OpenClaw 网关配置的工具执行和撤除隔离，宿主权限接线仍未验证。

## 环境

- ShiJu 现有开发构建 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。受管 OpenClaw 为 2026.10.10、Darwin arm64，入口 `openclaw.mjs` 的 SHA-256 为 `469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902`，工作区技能补丁摘要与仓库合约一致。
- 从 RueJS `main` 导出干净提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32`。服务 `ruejs_graph_probe` 使用原生 stdio 及 `--no-watch`。
- 复用 `189d22dc1` 的 [验收请求](../ACP验收.md#可复用的验收请求)，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户 `10001 / 100010005`，指定模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 私有 ShiJu 网关使用开发 SQLite 快照。健康、配置、预设及能力接口 HTTP 200，三轮聊天均 HTTP 200 / curl 0，但流内均为 `ProviderAuthError`，提示 `ACP session_model requires Provider credentials`。随后在已启用目标预设的授权实例复跑，继续使用私有工作区及每轮新建的受管 OpenClaw 回环网关。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef871dae0012qDs4fwrRjXQPK` | 200 / 0 / 71.575144 | `chat.done`，`MCP_NOT_AVAILABLE`。34 条原生工具事件，其中 4 个调用失败 |
| 挂载 | `ses_1ef883641001wCLmMq7IcurW6C` | 200 / 0 / 17.600383 | `chat.error`，`acp new_session failed`，无工具事件和答复 |
| 撤除 | `ses_1ef887bac001oGLFwz6WFy24qz` | 200 / 0 / 43.418021 | `chat.done`，`MCP_NOT_AVAILABLE`。14 条原生工具事件，其中 1 个调用失败 |

三轮 summary 均确认 `acp-openclaw / openclaw` 与 `full_access=true`。对照轮运行时 `connected=true`、权限状态 `client_only`，选模读回为 `session_model / alibaba-token-plan-cn / alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，结束后连接释放，绑定仍可读。挂载轮未连接，未写入已选模型。对照轮均无目标 MCP 事件，额外原生工具及失败调用使对照未通过。挂载轮无三种工具输出，无法核验 RueJS 调查答复。

首个后端断点是 ACP 桥的 `assertSupportedSessionSetup`。独立运行同版原生网关和受管 CLI，`initialize` 返回 ACP v1，`mcpCapabilities.http=false`、`sse=false`。直接发送含真实 stdio 定义的 `session/new`，收到 JSON-RPC `-32603`，明确拒绝会话级 MCP，与真实挂载轮一致。该检查针对任何非空列表，上游文档仍标注此能力不受支持。 [OpenClaw ACP 桥](https://github.com/openclaw/openclaw/blob/main/docs/cli/acp.md)

## 私有 OpenClaw 网关探针

将同一 stdio 定义写入私有网关的 `mcp.servers`，向 ACP 桥发送空列表。使用回环模型替身，关闭探针的 Tool Search 并提高模型上下文窗口，观测工具定义和续轮。挂载时有 8 个服务工具，撤除后新起的独立进程中为 0，两轮均为 `end_turn`。 [OpenClaw Tool Search](https://docs.openclaw.ai/tools/tool-search)

| 真实 stdio 工具 | ACP 与中继回读 |
| --- | --- |
| `rustcodegraph_query` | `in_progress → completed`，返回 `createRueSiteApp`（`app/app.tsx:32`） |
| `rustcodegraph_callers` | `in_progress → completed`，返回 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | `in_progress → completed`，回读挂载函数，含缺少 `#app` 的异常、快照清空和 SSR 分支 |

结果均进入探针模型续轮，ACP `session/request_permission` 回调数为零。把选中项直接写入网关配置前，仍需验证宿主拒绝、会话切换及撤销语义。当前保留原有门禁，未修改 Rust、受管补丁或权限策略，未执行 Cargo。

临时 MCP 配置、数据库和密钥副本已清理，自有进程已退出，授权实例仍健康。RueJS 副本逐项对比 5122 个归档条目，排除索引后无差异。首次私有 ShiJu 网关还打开了开发路径下的工作区记忆库，该库变化未核验。原始 SSE、握手和中继观测留在本机 0700 私有目录。
