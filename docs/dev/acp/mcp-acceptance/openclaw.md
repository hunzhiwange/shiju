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

## 适配后的指定模型验收

槽位 3 的 `cargo build -j 1 --locked -p vw-cli --bin shiju` 退出 0，18 分 11 秒。二进制 SHA-256 为 `f46699dd46d8520d8a7cfc3324132ccd5a2e05857e89fde160052fdcf75add3c`。私有实例从本地授权读取同一模型，安装接口 HTTP 200，实际启动经摘要校验的受管 v2 入口。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f023649c001Ge0luJ65pVhMwD` | 200 / 0 / 16.501349 | `chat.done`，含 `MCP_NOT_AVAILABLE`、两段说明及 4 次原生 tool search，目标调用为零 |
| 挂载 | `ses_1f023a66b001X5G6P62PlKWuy2` | 200 / 28 / 300.002255，续传 200 / 0 / 201.989752 | 游标 325 续传，获单个 `chat.done`，无 `chat.error`，396 条工具事件 |
| 撤除 | `ses_1f02b5355001L7aYPviB84yNt0` | 200 / 0 / 16.144983 | `chat.done`，含 `MCP_NOT_AVAILABLE`、额外说明及 6 次原生 tool search，目标调用为零 |
| 无完全访问权限 | `ses_1f02b51b3001ZMvfazO7AKZA7z` | 200 / 0 / 0.408323 | 独立拒绝对照，`chat.error` 要求当前完全访问授权，零模型工具事件 |

前三轮运行中回读 `acp-openclaw / openclaw`、`full_access=true`、`connected=true` 与 `client_only`，指定模型来源和完整 ID 均一致。结束后连接释放，仍保留同一模型绑定。

挂载轮 query、callers、node 成功次数为 105、7、67。query 返回 `createRueSiteApp`（`app/app.tsx:32`），callers 返回 `mountRueSiteApp`（`app/app.tsx:39`），node 回读该函数、`useApp` 及 `_$createApp`。工具的开始和完成可按调用 ID 匹配。首个额外调用是 status，随后还调用 files、explore、callees、impact。两次 query 把 `projectPath` 指向索引根以外的目录，均收到未初始化错误。

答复重建主调用链，引用了路由工厂和运行时行号，解释了 SSR、缺少 `#app`、路由及 i18n 安装、`/rust-layers` 布局绕过，并给出浏览器重复 DOM 与构建时预置节点两个预期场景。它把静态快照通道称为唯一自动执行清空的场景，同时又正确描述了浏览器入口的自动挂载，前一处说法与 `app/app.tsx:39-56` 矛盾。重复节点和稳定窗口影响属于推断。三轮有真实挂载与撤除证据，因答复格式、额外调用和两次读取失败，仍未满足严格合同。

### 私有三工具复验

同一构建、提示、模型和租户。临时 stdio 转发器只公布 query、callers、node，传递真实 RustCodeGraph 结果，并拒绝更换索引目录的参数。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f02b9bc0001op6W02gLecLt25` | 200 / 0 / 18.133280 | `chat.done`，仅输出 `MCP_NOT_AVAILABLE`。一次原生分发失败，目标调用为零 |
| 挂载 | `ses_1f02be354001rtS7QwRERvMIFf` | 200 / 28 / 300.000616，续传 200 / 0 / 210.921239 | 游标 320 续传，单个 `chat.done`，355 条工具事件 |
| 撤除 | `ses_1f033b3eb001shKKpuYtsbLLLt` | 200 / 0 / 34.200576 | `chat.done`，含 `MCP_NOT_AVAILABLE`、说明及原生查询失败，目标调用为零 |

三种工具分别成功 94、5、70 次，另有六次工具失败。模型请求了缺失的路由文件、把目录当文件读取，并传入与既定索引根不同的 `projectPath`。挂载轮还出现原生 `exec`（`echo skip`）和 `progress_card`。答复引用了实际主链、SSR、路由与 i18n 分支、布局判断和两个预期场景。独立无完全访问请求同样被宿主拒绝。终态回读保留指定模型，三个会话已断开。

适配和聚焦验证已完成。严格验收仍取决于指定模型在固定提示下满足答复格式、工具范围和零失败要求。现有实现保留原生工具和宿主权限语义，验收条件保持不变。两个私有 ShiJu 网关退出 0，端口关闭，临时挂载、凭据和 SQLite 副本已清理。RueJS 副本的 5122 个归档条目与基准一致。已授权实例健康接口仍为 HTTP 200。

## RustCodeGraph 1.2.11 与分页元数据复验

沿用已核验的 ShiJu 构建 `f46699dd46d8520d8a7cfc3324132ccd5a2e05857e89fde160052fdcf75add3c`、受管 OpenClaw 2026.10.10 v2、原提示、原租户、原模型及同一 RueJS 提交。切换至 RustCodeGraph 1.2.11，发行 SHA-256 为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`，索引可直接定位 `createAppRouter`（`app/router/index.ts:787`）。

复用已验证的私有 stdio 转发器。原生目录有 8 项，限定目录只公布三种原名工具，描述中标明索引根、已存在的文件和 `node.offset` / `node.limit`。协议对照中，函数查询、调用者查询及 `app/router/index.ts` 从第 780 行开始的分页源码结果均与直连相等。额外工具及越界读取收到拒绝。OpenClaw 当前宿主路径只接收选中的服务，共享 `allowedTools` 投影针对其他受管后端。本轮使用现有私有转发器，未改变公共接口。 [Goose 的转发与目录证据](goose.md) · [Pi 的发行摘要及原生筛选证据](pi.md) · [DeepAgents 的宿主策略证据](deepagents.md) · [Qoder 的直接声明证据](qoder.md)

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0f756f6001jz7MLP37YiV6ER` | 200 / 0 / 21.398017 | 精确 `MCP_NOT_AVAILABLE`，4 次成功的原生工具目录搜索，目标调用为零 |
| 挂载 | `ses_1f0f7abb1001ugqNLqyTZCTL2Z` | 200 / 0 / 279.348165 | 55 次 query、4 次 callers、54 次 node，全部成功，额外工具为零 |
| 撤除 | `ses_1f0fbf2c6001K15idt9ntaIFAy` | 200 / 0 / 27.277170 | 精确 `MCP_NOT_AVAILABLE`，4 次成功的原生工具目录搜索，目标调用为零 |

三轮各一个 `chat.done`，无失败工具、中继恢复或 `chat.error`，无需续传。summary 均读回 `acp-openclaw / openclaw` 与 `full_access=true`，options 均读回指定模型和 `session_model`。在线权限为 `client_only`，终态连接已释放。另发无完全访问请求 `ses_1f0fbefe20012IPZ30i0a5Xhzq`，HTTP 200 / curl 0 / 0.725205 秒，宿主拒绝，目标调用为零。

挂载轮 226 条事件均可按调用 ID 匹配开始和成功完成。`call_57cdf32f6a9d4bb5bb5e66d7` 返回 `createRueSiteApp`（`app/app.tsx:32`），`call_07fbfde71af84393a1f1662e` 返回 `mountRueSiteApp`（同文件第 39 行），`call_5e510f6e09f84c68b9d9f98b` 实际回读挂载函数与 SSR 守卫。`call_9adfed470d10497b87e8efff` 命中新索引的路由工厂。答复追踪到 `useApp.ts:4` 和 `compiler-runtime/app.ts:57`，覆盖缺少容器、插件安装、布局绕过与快照残留，并给出 jsdom 和静态产物预览两个场景。

本轮 1 次新路径复验解决了旧版的额外工具及无效读取问题。严格答复验收仍阻塞在第二个场景。答复要求在已加载首页 `/` 时点击首页链接，并断言 `location.pathname` 发生变化。实际路由为 `/`（`app/router/index.ts:647`），该链接仍指向 `/`（`app/pages/site/components/Layout.tsx:15`），所以这条断言在保留清空的基线也无法成立。场景中的 DOM 去重检查有源码依据，浏览器场景未实际运行。复跑条件为指定模型在固定提示下给出两项预期均与源码相符的场景。

本轮只更新验收文档，未运行 Cargo。私有网关退出 0，端口关闭，挂载、授权和数据副本已清理，RueJS 的 5122 个归档条目与基准一致。原始证据留在本机 0700 私有目录，原授权实例健康接口返回 HTTP 200。

## 工具说明补丁复验

在同一私有三工具目录的 `rustcodegraph_node` 说明中增加通用核验提示。提出路径变化的预期前，从路由和链接组件分别核对起点及目标。提出快照回归场景时，区分源码前置条件和浏览器预测，选择能体现差异的可观察结果。协议对照确认三种工具的 schema 和实际结果完全相同，仅 `node` 描述改变。原提示、模型、租户、文件、审批和运行态补丁沿用 1.2.11 复验。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f15581aa0013VCYudq6mA5Kb0` | 200 / 0 / 18.627917 | 精确 `MCP_NOT_AVAILABLE`，目标调用为零 |
| 挂载 | `ses_1f155cc14001s67Nbin0KhAZBg` | 200 / 28 / 300.004787，续传 200 / 28 / 300.006134、200 / 0 / 175.204396 | 游标 159、233，取得单个 `chat.done` |
| 撤除 | `ses_1f161a6a4001KcATTiZzdTVck7` | 200 / 0 / 18.455140 | 含 `MCP_NOT_AVAILABLE` 和额外解释，目标调用为零 |

挂载轮 query、callers、node 分别成功 55、2、74 次，其他工具和失败均为零。`call_06692a00965043f1bf99632e` 返回 `createRueSiteApp`（`app/app.tsx:32`），`call_7852404509824a88b5c9b654` 返回 `mountRueSiteApp`（第 39 行），`call_ea721a5d64ee4883b537d006` 读回挂载及 SSR 源码。三轮均收到 `chat.done`、无 `chat.error`，终态保持指定模型、原后端绑定和 `full_access=true`，连接均已释放。无完全访问权限的独立请求 `ses_1f161a451001GDJTvSHu3pqpnu` 被拒绝。

本次答复没有复现首页跳到首页却要求路径变化的断言，改为预渲染页 DOM 去重及仅加载 docs 入口的对照。但它把 `/design/button` 列为显式 `clientMode: 'app'` 的候选，实际路由声明第 490 行没有该元数据。构建脚本还会根据实际渲染结果决定客户端入口，见 `scripts/app-static-build.mjs:594-606`。撤除答复也多于约定的单一标记。这一组严格验收仍未通过。

本轮只改私有验收工具说明并提交记录，未改产品代码或运行 Cargo。私有网关退出 0，端口、凭据和挂载已清理，RueJS 的 5122 个归档条目与基准一致。
