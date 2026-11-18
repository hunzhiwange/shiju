# Nova 动态挂载 RustCodeGraph

2026-11-19，验收已阻塞。受管 Nova 1.1.65 的 owner approval 模式拒绝显式 MCP 服务。现有宿主门禁在发送 `session/new` 前拒绝非空服务列表。

## 环境与请求

- Darwin arm64，私有回环网关使用 ShiJu 0.6.0，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。预跑三轮均停在受审可执行文件校验。通过私有网关安装接口安装 `@compass-ai/nova@1.1.65` 后重新跑三轮，启动入口、CLI bundle 和模型绑定辅助文件均匹配仓库审核摘要。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32`。挂载轮通过私有项目的 `.vwacprc.json` 选择原生 stdio 服务 `ruejs_graph_probe`，进程参数包括 `--no-watch`。
- 三轮使用同一 curl 请求，每轮新建会话。租户为 `10001 / 100010005`，绑定 `acp-nova / nova`，指定 `session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`full_access=true`。提示来自 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

## 三轮网关结果

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态 | 答复和工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f007172b001NDd3HrUvRMIGRA` | 200 / 0 / 8.387 | 一个 `chat.done` | 包含 `MCP_NOT_AVAILABLE` 和额外文字，3 次原生 `tool_capabilities` 调用，6 条工具事件。 |
| 挂载 | `ses_1f007394b001rFvgngq1fl2OSc` | 200 / 0 / 1.314 | 一个 `chat.error` | 无模型答复，零工具事件。 |
| 撤除 | `ses_1f0073f20001jHpOoaFwUlnc2F` | 200 / 0 / 8.484 | 一个 `chat.done` | 包含 `MCP_NOT_AVAILABLE` 和额外文字，3 次原生 `tool_capabilities` 调用，6 条工具事件。 |

挂载轮先收到 `chat.model_recovery`，`phase=stopped`，再收到 `chat.error`。首个失败原因为 `Nova owner approval mode does not support MCP servers`。模型未收到 RustCodeGraph 工具定义，三种目标工具均无调用和输出，源代码调用链、分支、快照影响和验证场景均无答复。对照轮目标工具事件均为零。两轮的额外文字和原生工具调用违反原提示。

三轮 UI summary 均确认绑定和 `full_access=true`。两组对照运行中在 ACP options 确认 `connected=true`、指定模型、Nova 自报版本 `1.0.0` 和权限 `support=client_only`。会话结束后连接释放，仍可回读模型。挂载轮无有效连接、选模或审批结果。

## 原生边界与受管替代入口

- 用匹配审核摘要的 1.1.65 bundle 直接启动隔离 ACP 进程。握手确认 ACP v1、`ownerApprovalRequired=true`，并声明 HTTP 和 SSE。以同一必需审批元数据分别发送 stdio、HTTP 和 SSE 三个 `session/new` 请求，均返回 `-32602` 和 `MCP servers are unavailable in owner approval mode`。stdio 启动标记未生成，三个请求均未创建会话。1.1.66 npm 发行包静态检查仍有相同的新建会话和直接调用门禁。[Nova 1.1.66 发行包](https://www.npmjs.com/package/@compass-ai/nova/v/1.1.66)
- 同一受审入口上，空服务列表建会话成功。受控回环模型请求编辑私有标记文件，Nova 发出 `permission/request` 并提供 `allow_once` 和 `reject_once`。宿主选择 `reject_once` 后，原生终态为 `rejected`，文件仍为 `before`。初次探针预期模型名与回环收到的模型名不符，调整夹具后复跑通过。此项证明原生编辑审批，真实模型证据来自上述网关轮次。
- 核对 1.1.65 的 `compass.host-tools.v1` 扩展。受控宿主声明带 `requiresApproval=true` 的单个查询定义，启用扩展的会话向模型提供该定义，经 `session/host_tool/call` 取得预先独立查询的结果并回传给模型。该调用没有发出原生 `permission/request`。另一个只在连接声明目录、会话未启用扩展的探针仍向模型展示相同工具定义，调用未进入宿主回调。当前 ShiJu ACP 事件客户端对扩展请求返回 method not found。采用此路径需要宿主接入按会话管理的目录和工具执行、逐次审批、撤除及结果事件。现有路径缺少这些接线。
- 独立 stdio 直连 RustCodeGraph，`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三个调用均成功。这些独立结果未进入真实模型会话。

保留 Nova 驱动、协议门禁及现有断言。原生握手已确认必需审批能力，本轮未涉及缺失能力的握手路径，未运行 `acp_onboarding_nova_unverified_approval_capability_is_rejected`，也未编译。等待可按会话注册和撤除 MCP、经宿主审批处理每次调用的 Nova 入口后，复跑三轮和受影响的定向测试。

私有网关及原生探针均已退出。临时挂载文件、授权副本和 Nova 同意记录副本已清理。原始证据留在本机 0700 验收目录，用户网关健康检查通过。

## 共享工作区复验

在当前共享工作区复用已有 ShiJu 0.6.0 构建，摘要与首轮一致。Nova 驱动、协议门禁和事件客户端相对首轮提交无差异。重新创建隔离网关、配置和 RueJS 副本，使用同一指定模型、租户、提示和审核版 Nova 1.1.65。

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f00aae9d001jzwshHnyq2KDHv` | 200 / 0 / 11.390 | 一个 `chat.done`，包含 `MCP_NOT_AVAILABLE` 和额外文字，3 次原生 `tool_capabilities` 调用。 |
| 挂载 | `ses_1f00adc5300142vTosohNf4CtO` | 200 / 0 / 1.230 | 一个 `chat.error`，原门禁及 `phase=stopped` 重现。 |
| 撤除 | `ses_1f00ae2420013Ggh7mzv7FkyiP` | 200 / 0 / 10.203 | 一个 `chat.done`，包含 `MCP_NOT_AVAILABLE` 和额外文字，4 次原生 `tool_capabilities` 调用。 |

三轮目标工具事件均为零。两组对照在线确认指定模型和 `client_only`。挂载轮未连接到有效会话，未选定模型，也未发起工具审批。

- 将官方 1.1.66 安装到私有探针目录，运行 bundle 的 SHA-256 为 `89f35c4dfc643f7606972b2f406b87e7e39e93624b50fa0c5159c2e7f3f85f28`。原生 ACP 动态复现 stdio、HTTP、SSE 的 `-32602` 拒绝。只缩小宿主门禁无法改变这一结果。发行包还在会话组件初始化和直接调用路径拦截 MCP。空列表下的编辑拒绝对照再次收到 `permission/request`，选择 `reject_once` 后返回 `rejected`，文件未改变。
- 1.1.66 的扩展探针复现了 1.1.65 的审批与可见性行为。声明 `requiresApproval=true` 的查询经宿主扩展回调返回结果，未触发现有审批回调。只关闭会话选项时模型仍能看到连接声明的工具，调用无法到达宿主。启动另一条未声明工具目录的独立连接后，模型侧工具表不含该查询。因此，按连接隔离可以解决该探针中的目录残留，宿主逐次审批仍需接线。另核对 1.1.66 的 `acp --allowed-tools` 和 `acp --only-tools`，两项均在握手前以未知选项退出 1。
- 对照仓库现有转换路径。Snow 的补丁依赖其原生 MCP 注册与调用循环。OpenClaw 的私有网关原生执行缺少本任务要求的逐次 ACP 审批。Cortex 的在研转换器在标准权限请求后执行工具，但依赖 Cortex 的会话级 `clientTools`、通知和回调格式。Nova 的目录必须在连接初始化前声明，现有 Cortex 入口不能接管。复用标准审批和 MCP 客户端需要接入 Nova 扩展协议并绑定工具目录、当前会话、审批决定和取消生命周期。
- 下一步需要支持 owner approval 的上游会话级 MCP 入口，或确认将任务扩展为新增经审查的宿主代理执行路径。宿主方案需在独占连接隔离目录，识别和拒绝越权回调，并确认每次执行前沿用同一审批通道。获得其中一个条件后，复跑原三轮和审批回归。

复验沿用现有二进制和原生受控探针，未修改产品代码或运行 Cargo 测试。私有网关及原生探针已退出，挂载文件、授权副本和私有同意记录副本已清理。
