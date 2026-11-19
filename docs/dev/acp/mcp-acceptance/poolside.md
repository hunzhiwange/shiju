# Poolside 动态挂载 RustCodeGraph

2026-11-19，验收阻塞于受管 Poolside 的 `session/new` 门禁。无挂载和撤除对照通过，挂载轮在发起模型推理前被拒绝。

## 验收环境

- Darwin arm64，受管 Poolside 1.0.16。原生程序 SHA-256 为 `5dcb0162fd21a535e4f67117125e7d205e27a6881862aa223d0424082848f874`，启动器为 `a52c42721d5badf57aff54da940a4e44e9173f20da415376404609a29c98c646`，均与受管清单一致。私有网关回读已安装。
- 私有回环网关复用 shiju 0.6.0 构建，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。私有实例启用 `acp-poolside`，使用租户 `10001 / 100010005`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model` 和 `full_access=true`。
- 提示取自提交 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本取自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载轮使用原生 stdio `ruejs_graph_probe`，关闭文件监听。[复用请求](../ACP验收.md#可复用的验收请求)

## 同一请求的三轮结果

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 答复及工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efbd85b00016VRIZd96kiztzU` | 200 / 0 / 10.708320 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |
| 挂载 | `ses_1efbdb0c8001TWslP4dFBXsHnu` | 200 / 0 / 1.528858 | `chat.error` ×1 | 建会话失败，无答复，0 条工具事件 |
| 撤除 | `ses_1efbdb832001yddpC784AHDjq1` | 200 / 0 / 10.330277 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |

三轮终态 UI summary 都回读 `acp-poolside / poolside` 与 `full_access=true`。两轮对照在运行中回读 `connected=true` 和 `access_status.support=client_only`，运行中及终态的 ACP options 均确认指定提供商、完整模型 ID 和 `session_model`。终态连接释放。挂载轮 ACP options 的 `connected=false`、`selected_model=null`，未触发原生选模或工具审批。三种目标工具 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均零调用和零输出，调用链、行号与静态快照分析无可验收答复。

## 失败边界与复跑条件

- 宿主在挂载轮完成 stdio MCP 初始化。受管启动器在处理 `session/new` 时拒绝非空 MCP 列表。SSE 原始错误包含 `Poolside requires the reviewed workspace and no external MCP servers`。隔离探针固定相同的规范工作目录，向受审启动器实际发送非空 `mcpServers`，复现 JSON-RPC `-32000`，私有回环模型未收到推理请求。
- 独立原生探针使用同一启动器生成的私有设置、必需的 Docker 沙箱和回环模型替身。ACP v1 握手报告 `pool-acp` 1.0.16、`loadSession=true`、`mcpCapabilities={}`。分别发送空列表与同一 RustCodeGraph stdio 定义，原生端均建会话并以 `end_turn` 完成短提示，`/v1/chat/completions` 请求均只声明 `read` 和 `write`。原生端接受请求的行为不足以证明工具可用，移除门禁也无法使这条路径向模型声明目标工具。
- 保留受管 Docker 只读工作区、宿主写入审批和模型中继约束。此次三轮无文件或终端工具请求。复跑需可验证的模型工具入口，并在该入口中保留会话、租户、审批和撤销边界。

网关和探针均已退出，回环端口关闭。本轮两个已退出的沙箱容器按私有工作目录的挂载来源核实后移除。私有挂载已撤除，RueJS 副本的 4108 个普通文件与导出内容相同，私有租户配置及数据库副本已清理。原始 SSE 与脱敏探针摘要保存在本机私有目录。本次只更新文档，生命周期夹具未覆盖受管启动器的拒绝分支，未运行 Cargo 或桌面构建。

## 共享工作区复验

使用更新的 shiju 0.6.0 构建，SHA-256 为 `1a033a19a5c5e2c115e90abbd336f898c0df3ac101a9824ffe8b36f11b0bcc47`。仍使用相同受审 Poolside、RueJS 提交、请求提示、租户和指定模型。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f048fdf10017Goz22vBo8l292` | 200 / 0 / 52.268240 | `chat.error`，原生选模时代理初始化失败 |
| 挂载 | `ses_1f049cbb70015GcjLk1KQ8PDLI` | 200 / 0 / 1.374166 | `chat.error`，受管启动器拒绝非空 MCP 列表 |
| 撤除 | `ses_1f049d1ca0010DnpB94HhR585p` | 200 / 0 / 48.909404 | `chat.done`，仅返回 `MCP_NOT_AVAILABLE` |

三轮均零工具事件，UI summary 回读原后端及 `full_access=true`。挂载轮宿主再次初始化 RustCodeGraph 成功。无挂载和挂载轮的 ACP options 终态都没有选中模型。撤除轮选模回读原提供商、完整模型 ID 和 `session_model`。无挂载轮的新故障发生在模型配置阶段，挂载拒绝在建会话阶段独立复现。

### 原生设置、传输与审批

- 公开发行的最新版仍为 1.0.16。原生 `mcp_servers` 设置支持 stdio 和远程 HTTP。将宿主 stdio 加入原生设置时，Poolside 在 Docker 内解析命令，macOS 可执行文件在容器不可用。另核验并在相同 ARM64 镜像内执行 RustCodeGraph 1.2.10 的 Linux ARM64 发行，成功索引 3478 个文件。宿主绝对路径和工作区相对路径均无法启动 stdio。使用原生沙箱的只读挂载路径后，模型收到图谱工具定义，但首个工具回合在探针预算内未收到审批或返回结果。该路径需要另配 Linux 可执行文件和容器路径。 [Poolside 固定版配置说明](https://github.com/poolsideai/pool/blob/v1.0.16/README.md#mcp-servers) · [RustCodeGraph 1.2.10 发行](https://github.com/hunzhiwange/rustcodegraph/releases/tag/v1.2.10)
- 私有 Streamable HTTP 诊断使用独立诊断令牌，桥接宿主侧实际 RustCodeGraph。原生 ACP 保留 Docker 沙箱与原模型回环配置。仅在 `initialize` 后、`session/new` 前将 HTTP 条目加入私有设置，原生端仍能向模型声明目标工具。宿主取消审批时收到零次 `tools/call`。逐次批准时，三个原生调用 ID 分别关联以下工具和真实模型续轮输入。诊断模型替身以 `end_turn` 退出，未触发写入。

| 调用 ID | 工具 | 真实结果 |
| --- | --- | --- |
| `probe-query` | `rustcodegraph_query` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `probe-callers` | `rustcodegraph_callers` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `probe-node` | `rustcodegraph_node` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp(factory)` |

HTTP 探针证明原生工具与权限回传可用。正式接入仍依赖经审查的宿主侧会话级 MCP 转发入口，须以宿主的租户、挂载选择、工具策略、单次审批和撤销控制调用。当前通用模型中继仅提供模型协议，Cortex 和 DeepAgents 的受管 stdio 实现使用各自的工具交接，未提供 Poolside 所需的 HTTP 入口。需在公共层协调该入口的归属和契约，随后接入 Poolside 私有设置并复验原模型。诊断入口只在私有探针中存在。

私有网关、HTTP 和 stdio 探针已退出，回环网关端口关闭。根据挂载路径核实并清理本轮自有沙箱容器，已删除临时网关租户配置和数据库。未运行 Cargo。

## 受管启动器补丁

Poolside v2 使用每个受管进程独有的回环 HTTP 桥，将新 ACP 会话选中的 stdio 服务交给原生 `mcp_servers`。服务目录按宿主 `allowedTools` 和 `deniedTools` 过滤。stdio 子进程复用受审的只读隔离及空环境，原生 Docker 工作区和模型中继不变。桥只接受宿主对当前会话、工具和完整参数的单次批准。取消、结束或关闭清除授权，完成或取消调用后终止 stdio 子进程。挂载通过新会话，已有挂载进程拒绝加载或接入另一会话。

在私有目录用仓库安装脚本安装 Poolside 1.0.16 v2，四项受审摘要通过。使用真实 Poolside 和 RustCodeGraph、回环模型完成四个场景。无挂载和撤除只提供原生 `read`、`write`。批准轮只发布策略选中的三种图谱工具，三次单次批准后，续轮读回 `app/app.tsx:32`、`app/app.tsx:39` 与 `_$createApp(factory)`。拒绝轮收到审批请求和拒绝结果，无图谱输出。四个提示均以 `end_turn` 结束。使用私有 RueJS 索引的只读 SQLite 事务保留 WAL 和 SHM，沿用既有沙箱权限。

本地 Python 定向测试 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s patches/acp-model-source/poolside -p '*test.py'` 通过 3 项。已验证批准与完整参数绑定、不可重复使用及取消后的迟到批准失效。真实阿里模型仍需携带 v2 安装契约的新网关完成三轮，回环模型证据不计入真实模型验收。
