# OpenClaw 动态挂载 RustCodeGraph

2026-11-19。原始 ACP 桥在 `session/new` 拒绝会话级 MCP。适配复用请求独占的 OpenClaw 网关，并在宿主权限检查通过后配置选中的 stdio 服务。

## 环境

- ShiJu 现有开发构建 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。受管 OpenClaw 为 2026.10.10、Darwin arm64，入口 `openclaw.mjs` 的 SHA-256 为 `469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902`，工作区技能补丁摘要与仓库合约一致。
- 从 RueJS `main` 导出干净提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32`。服务 `ruejs_graph_probe` 使用原生 stdio 及 `--no-watch`。
- 复用 `189d22dc1` 的 [验收请求](../ACP验收.md#可复用的验收请求)，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户 `10001 / 100010005`，指定模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 私有 ShiJu 网关使用开发 SQLite 快照。健康、配置、预设及能力接口 HTTP 200，三轮聊天均 HTTP 200 / curl 0，但流内均为 `ProviderAuthError`，提示 `ACP session_model requires Provider credentials`。随后在已启用目标预设的授权实例复跑，继续使用私有工作区及每轮新建的受管 OpenClaw 回环网关。

## 适配前三轮真实模型实测

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

结果均进入探针模型续轮，ACP `session/request_permission` 回调数为零。这一组探针用于定位原生执行边界，宿主接线和生命周期复验见下文。

临时 MCP 配置、数据库和密钥副本已清理，自有进程已退出，授权实例仍健康。RueJS 副本逐项对比 5122 个归档条目，排除索引后无差异。首次私有 ShiJu 网关还打开了开发路径下的工作区记忆库，该库变化未核验。原始 SSE、握手和中继观测留在本机 0700 私有目录。

## 私有网关适配与生命周期复验

会话模型模式把宿主已选中的 stdio 服务写入本次请求独占的 OpenClaw 网关配置，成功启动后才清空 ACP 线上的 `mcpServers`。原生工具执行缺少 ACP 逐次审批，因此适配要求当前会话具有完全访问权限。只读、拒绝、非交互审批策略、禁用工具及 CLI 本地模型模式均拒绝转换。ShiJu 的 ACP 工具目录选项映射到 Claude 专用元数据，OpenClaw 以宿主解析后的 MCP 服务选择为准。现有转换限字面量绝对命令和参数，拒绝显式环境变量与配置插值。未选中 MCP 时沿用现有配置。

受管安装保留原版本，并在 v2 增加固定摘要的 stdio 进程所有权补丁。原版探针中，忽略 SIGTERM 且在输入关闭后继续运行的 MCP fixture 位于独立进程组，强制终止网关后仍存活。补丁使用原生 `terminateOnHostLoss`，同一 fixture 在网关进程组收到 SIGKILL 后退出，环境探针也未发现 `VW_OPENCLAW_*` 中继凭据。验证过程中所有 fixture 均已退出。

用仓库补丁脚本重新生成私有 v2 运行时，三个受管文件均匹配合约摘要。复跑原生 stdio 探针，query、callers、node 各有一对 ACP 进行中和完成事件，对应结果均进入真实 MCP 至回环模型的续轮，新建撤除轮看不到这三个工具。

另用同一原生会话 ID 在撤除后的独立网关上执行 `session/load` 和 `session/resume`，两者均成功。续轮保留历史工具结果，当前模型工具目录中的目标服务为零，也没有新工具调用。此轮仍使用回环模型替身。

在独立原生网关用指定阿里模型补跑原提示，模型看到 8 个真实 MCP 定义，query、callers、node 成功次数分别为 8、2、12，结果进入模型的后续请求。该轮还使用 status、files、explore，并在读取不存在的 `app/router.tsx` 时失败，原生终态为 `end_turn`，没有最终答复。私有 stdio 转发器只公布三种目标工具的复跑中，仍有 5 次失败和一次额外原生 `exec` 调用，300 秒内未返回终态。两次为独立后端诊断。

首个带宿主接线的共享构建中，私有 ShiJu 网关已读取租户授权。无挂载、撤除轮均能连接指定模型，挂载轮却在启动前被工具清单检查拒绝。追查发现清单由 session engine 为所有 ACP 请求生成，只进入 Claude 扩展元数据。调整后继续使用 OpenClaw 的宿主 MCP 选择，保留完全访问、明确拒绝、非交互审批和禁用工具门禁。

定向命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib agent_acp_defaults_reach_new_session_and_prompt`。槽位 3 的 target 和 build 目录均为 `target/rust-slot-3`。复现时退出 101，1 项通过、OpenClaw 投影断言失败。实现后同一目标退出 0，2 项通过、0 失败。最终构建及 ShiJu 三轮复验另记。
