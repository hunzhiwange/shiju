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

## 保留原生门禁的补丁原型

在私有目录复制审核版 1.1.65，按原 bundle 摘要和唯一替换位置应用原型。补丁仅在 `requestHostTool` 发出 `session/host_tool/call` 前插入原有的 `permission/request`。审批仅接受 `allow_once`，并在发出工具回调前再次检查会话的工具授权。补丁保留对直接 MCP 服务注册及调用的原生门禁。补丁后 bundle 的 SHA-256 为 `468bc10944e3b67974562278c5c304d112eb0fdbbb9696a4b5bd67d9d32354df`，`node --check` 退出 0。

受控模型在各自的新 ACP 连接发起工具调用。探针宿主核对审批与回调的工具名、参数和调用 ID。允许时由探针宿主运行真实 stdio RustCodeGraph 调用，核对续轮收到的具体工具结果。

| 场景 | 宿主审批数 | 实际 MCP 调用数 | 模型续轮或目录 |
| --- | ---: | ---: | --- |
| 允许 `rustcodegraph_query` | 1 | 1 | 收到 `createRueSiteApp`（`app/app.tsx:32`）。 |
| 允许 `rustcodegraph_callers` | 1 | 1 | 收到 `mountRueSiteApp`（`app/app.tsx:39`）。 |
| 允许 `rustcodegraph_node` | 1 | 1 | 收到 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）及 `_$createApp`。 |
| 拒绝查询 | 1 | 0 | 收到拒绝结果。 |
| 新连接撤除目录 | 0 | 0 | 工具定义不可见。 |

另用打过补丁的入口复跑三种传输的直接挂载拒绝，均返回 `-32602`。原生编辑仍收到单次审批，拒绝后返回 `rejected`，标记文件保持原内容。受控模型与 MCP 进程均已退出。

原型验证了补丁可在不移除门禁的情况下建立经单次审批的工具闭环。网关仍需把显式挂载解析到独占目录，并在宿主回调处理器中绑定活动回合、会话、调用 ID、参数和一次性审批，拒绝重放并取消进行中的操作。上述工作与指定阿里模型三轮仍待验收。本次仅修改私有 bundle 和文档，未运行 Cargo 测试。

## 受管 v2 接线

Nova 继续固定在 npm 1.1.65，受管安装目录升到 v2。宿主只把显式绝对路径、无环境变量覆盖的 stdio 服务交给匹配审核摘要的会话模型入口。目录发现与实际工具调用复用现有工作区只读 stdio 沙箱，进程环境不含模型凭据。目录按连接绑定到单个会话，工具策略使用宿主解析后的允许与拒绝列表。同一受管连接内的第二次建会话明确拒绝。

每次执行前用原生 `permission/request` 请求 `allow_once`，执行前再次核对活动回合、调用 ID 与会话工具资格。取消和关闭会话撤销操作。直接注册 MCP 的原生门禁继续生效，受管入口只把选中的目录投影到 `compass.host-tools.v1`。HTTP、SSE 和未审入口保留拒绝。宿主会话工具允许列表与逐服务工具策略取交集，Nova 官方工具沿用原有权限。

在隔离副本重建 RustCodeGraph 1.2.11 索引。使用宿主 SQLite 读事务维持索引的 WAL / SHM；早先缺少边车文件时，只读沙箱的工具目录为空。当前入口在受控模型上通过三个实时查询、拒绝及撤除五项，查询续轮分别读到 `createRueSiteApp`、`mountRueSiteApp`、`useApp`。允许列表每轮只登记一个目标工具，重复建会话被拒绝。此验证使用系统安装脚本相同的补丁和帮助文件，并逐项匹配受审契约摘要。

冷安装通过仓库受管安装脚本，六项摘要匹配，对冷安装入口复跑五项成功。同一探针在 v1 首个带挂载的建会话请求返回 `-32602`，退出 1。额外探针在批准等待中发送取消，返回 `cancelled`，未获得查询结果。在新连接恢复已有历史时，显式重新挂载对应工具能继续申请审批并查询，空列表恢复仍保留历史且模型目录没有该工具。

定向测试 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_nova acp_onboarding_nova` 退出 0，7 项通过、3 项需独立 CLI 环境的用例仍为 ignored。缺少必需审批能力和未受管挂载均明确拒绝。槽位 9 的 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-9`。

### 指定模型原生复验

冷安装 v2 在私有回环中继使用指定阿里模型及原提示。独立无挂载会话 `2f51dd87-3962-467c-ac00-1a1e073acf05` 和撤除对照 `d110b2cf-3f7c-443d-95c5-5fd1a47feb46` 均收到 `end_turn`，可见答复为 `MCP_NOT_AVAILABLE`，目标工具事件为零。撤除对照另有 4 次 Nova 原生 `tool_capabilities` 调用。

挂载会话 `2e04e51a-afc2-4377-8dcc-aa5e27c9b3d4` 只调用三种目标工具。全部 217 次调用都经过单次审批，其中 215 次成功。查询 `123f75d8-f8a4-4bd5-9445-74bedf8fc2c2` 返回 `createRueSiteApp`（`app/app.tsx:32`），调用者 `b456eb7a-4795-44b8-9af0-5573965cd21f` 返回 `mountRueSiteApp`（`app/app.tsx:39`），节点 `f0eec7dc-bf70-48fc-b50f-12a00694e7d1` 返回工厂定义。两次查询因模型把文件或未索引子目录作为 `projectPath` 而失败。模型持续查询超过 15 分钟，已结束私有诊断进程，未取得挂载轮最终答复。因此原生复验提供真实工具及审批证据，严格三轮验收仍待完成。

首次用包含 v2 的共享网关执行三轮。由于当前 HTTP 请求结构移除了 `acp_agent`，仅省略此字段，仍由 `agent=acp-nova` 选择后端。两个对照收到 `chat.done`，挂载轮在宿主门禁得到 `chat.error`。根因为网关自动生成的会话工具允许列表被早期门禁误判为不受支持。修订后 ACP 会话元数据传递已解析的 ID 列表，由受管入口与逐服务允许及拒绝规则取交集。

当前验收允许官方工具沿用原权限共存。是否通过由挂载服务的实际执行与结果回传判定，并核对审批和撤除。前面以无官方工具调用及答复格式衡量的结果保留为历史记录。

允许列表修订后，定向测试 `cargo test -j 1 -p acp-client --lib nova_mount_carries_resolved_tool_allowlist` 经槽位脚本在槽位 9 退出 0，1 项通过、321 项过滤。两个缓存路径均为仓库的 `target/rust-slot-9`。受控模型与真工具五项对照再次通过。指定阿里模型通过新的目录策略挂载时，三种目标工具均进行了实际调用并回传内容。记录的首个查询、调用者和节点调用分别为 `44f27f95-acf3-4901-a84e-6a2de061bfb4`、`98068baa-3ae4-466c-a8e1-df3adff1fd06` 和 `da9ff134-ee4b-4a8b-afc1-088be875530c`。此证据来自受管入口的原生 ACP 事件，网关 SSE 需复验。

另做了结果回传探针，使用同一提示、模型、受审摘要和宿主工具允许列表。挂载会话 `cbe086fe-80ed-432d-9059-139e954c0f26` 的 query 2 次、callers 1 次、node 3 次均逐次获批并成功。实际发往指定模型的续轮请求包含三种真实输出。探针收到所需回传证据后主动取消该轮。撤除对照 `c068f13e-4fdc-4049-ace7-8641f968912d` 在新连接结束，答复为 `MCP_NOT_AVAILABLE`，模型工具表无该服务声明且目标事件为零。此项已验证新标准的执行与回传，独立于网关 SSE 验收。

### 网关回归状态

最新源码的 `cargo build -j 1 --locked -p vw-cli --bin shiju` 在槽位 9 退出 101。Nova 对应注册、协议和定向测试已编译。共享工作区的 `vw-agent` 同时读到 MiMo 与 VT Code 的新调用和旧版依赖产物，报缺少 `ManagedMcpApproval`、`prepare_stdio_mcp_launch`、`verify_vtcode_mcp_executable` 等六处编译错误。本次构建未产出更新后的网关，修订后的 SSE 三轮待回归。现有网关和用户全局 Nova 配置未改，探针和临时网关均已退出。

## 新标准网关回归

共享构建 SHA-256 为 `28793fbd08778fb50798533905a8aeb09dea69d4cdfd1589c7f9f76fa5255419`，包含前次 Nova 允许列表修复，隔离安装的六个文件均匹配契约。无挂载 `ses_1f1e33067001tl0MO0TtL1PcHR` 与撤除 `ses_1f1e36c5e001LFUIIA3egp9nYz` 均收到 `chat.done`，官方能力查询保持可用。挂载会话 `ses_1f1e363a6001OpuAhtXpUOhUhz` 在投影目录时收到 `Selected Nova MCP servers have no permitted tools`。三轮 HTTP 均 200。

对失败轮使用相同沙箱、目录、图谱二进制与索引独立执行 `tools/list`，成功发现 8 个图谱工具。问题在会话 runner 把本地目录 ID 写进 `acp_allowed_tools`，其中没有 Nova 在受管连接中才发现的工具。修复仅对 Nova 保留进入 runner 时已有的显式 ACP 列表，继续通过受管入口执行会话限制和服务级允许与拒绝策略。

槽位 6 构建已编译变更后的 `session-engine`，随后在共享工作区的 `gateway-mcp` 退出 101。目录管理的进行中修改引用了尚未同步的 `McpSafeExportResponse`、`McpCatalogStore::safe_export`、`update_record` 和更新字段。运行时无新增网关可用，修复后三轮等待可构建的共享代码。槽位的两个缓存路径均为仓库 `target/rust-slot-6`，构建命令为 `python3 scripts/cargo_slot.py cargo build -j 1 --locked -p vw-cli --bin shiju`。
