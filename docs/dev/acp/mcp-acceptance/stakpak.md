# Stakpak 动态挂载 RustCodeGraph

2026-11-19，验收阻塞于 Stakpak 0.3.88 的会话 MCP 工具发现。无挂载和撤除对照通过。挂载轮宿主已初始化 RustCodeGraph，模型仍报告工具不可用。

## 验收环境

- Darwin arm64，私有回环网关使用 shiju 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。通过私有实例安装受管 Stakpak，安装状态回读为 true。原生程序 SHA-256 为 `70c559eff250a5be0dcdb19f31f13dca86847ee57648dba78e531506bb482ebc`，启动器为 `bff6d5b2b49b92ce759be1e00cb6955cc0ec7bf181122ae553544197721aca30`，均与受管清单一致。
- 私有实例启用 `acp-stakpak`，沿用租户 `10001 / 100010005` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。提示取自提交 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)
- RueJS 私有副本基于 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 工作目录。RustCodeGraph 1.2.10 索引 3686 个文件，定位到 `app/app.tsx:32` 的 `createRueSiteApp`。挂载轮使用 `ruejs_graph_probe` 原生 stdio，关闭文件监听。

## 三轮真实请求

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 答复及工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efc35095001trBAX333LuO2or` | 200 / 0 / 5.265442 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |
| 挂载 | `ses_1efc3669a001l5ld7pmbuhC8w4` | 200 / 0 / 5.927033 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` 及工具不可用说明，0 条工具事件 |
| 撤除 | `ses_1efc37eb60010rQGICF2qF3PYG` | 200 / 0 / 5.043792 | `chat.done` ×1 | `MCP_NOT_AVAILABLE`，0 条工具事件 |

三轮均无 `chat.error`。终态 UI summary 回读绑定 `acp-stakpak / stakpak` 和 `full_access=true`。运行中 ACP options 显示 `connected=true`、`access_status.support=client_only`、后端版本 `0.3.88`，结束后连接释放。运行中及终态均回读 `session_model`、指定提供商和完整模型 ID。三种目标工具 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均无调用或输出。挂载轮的调用链、行号和静态快照分析均无答复可核对。三轮也没有其他工具请求或审批事件。

## 第一处失败边界与复跑条件

- 挂载轮宿主完成了 RustCodeGraph MCP 初始化。仓库 ACP 新会话路径将所选服务带入 `session/new.params.mcpServers`。隔离探针使用相同摘要的受管启动器，记录实际发送的 JSON-RPC 帧和回环模型入口收到的工具定义。ACP 握手为 v1，`loadSession=true`，声明 `http=true` 和 `sse=true`。
- 探针分别发送空列表、同一 RustCodeGraph stdio 定义和两个指向回环探针的 HTTP、SSE 定义。四次均成功建会话并以 `end_turn` 结束，回环只收到模型请求。每次模型请求都仅有 `stakpak__view`、`stakpak__create` 和 `stakpak__str_replace`，没有探针 MCP 连接或工具声明。固定版源码的 `new_session` 没有读取 `mcp_servers`，`prompt` 使用进程启动时装载的工具集。[Stakpak 0.3.88 ACP 实现](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/commands/acp/server.rs)
- 另在隔离的 `SHIJU_STAKPAK_RUNTIME` 写入启动期 MCP 配置。原生模型请求仍只包含三个内置工具。使用同一 sandbox profile 单独验证 RustCodeGraph 进程启动，`sandbox-exec` 退出 71 并报告 `Operation not permitted`。本机 RustCodeGraph 的 `serve --help` 只列出 stdio MCP，现有受管路径未找到可将其接入 Stakpak 且保留会话、审批和撤销的回环转换。复跑需具备这些边界的受审原生版本或兼容入口。[Stakpak 启动期 MCP 实现](https://github.com/stakpak/agent/blob/v0.3.88/cli/src/commands/agent/run/mcp_init.rs)

自有网关和探针已退出，端口已关闭。已撤除私有挂载并清理租户配置、凭据和数据库副本。原始 SSE 和探针证据保存于本机私有目录。此次只更新验收记录，现有生命周期夹具未覆盖该原生工具发现边界，未运行 Cargo 或桌面构建。

## 共享构建和启动期 HTTP 复验

共享 shiju 0.6.0（SHA-256 `1a033a19a5c5e2c115e90abbd336f898c0df3ac101a9824ffe8b36f11b0bcc47`）复用同一受审 Stakpak 0.3.88、原始提示、租户、模型和 RueJS 固定提交。挂载通过 Goose 和 Hermes 已核对的 stdio 转发，只公布三种目标工具，保留分页参数、真实入口元数据、调用参数及图谱返回。会话 `ses_1f0528659001rK16uQJ5yWKTuI` 为 HTTP 200、curl 0、5.426560 秒。宿主初始化真实 RustCodeGraph 1.2.10，随后收到一条 `chat.done`、零 `chat.error`、零工具事件，答复仅为 `MCP_NOT_AVAILABLE`。UI summary 和 ACP options 回读原绑定、`full_access=true`、运行中 `client_only`、指定模型和 `session_model`，终态连接释放。沿用首次通过的无挂载与撤除对照。

保留原受管启动器、沙箱和私有 `SHIJU_STAKPAK_RUNTIME`，在其中配置启动期 Streamable HTTP MCP。诊断 MCP 和回环模型共用监听端口，使用不同的仅诊断令牌。HTTP 入口转发到工作区外启动的限定 stdio 目录，最终由 RueJS 副本的真实 RustCodeGraph 处理。Stakpak 只向回环模型替身声明三种图谱工具。模型依次请求三种工具，原生提示轮次以 `end_turn` 结束。两轮均收到三次原生 `session/request_permission`。全取消时 HTTP 入口只收到初始化与列表请求，零次 `tools/call`，模型续轮收到三个 `TOOL_CALL_REJECTED`。逐次批准时，原生工具调用和模型续轮按 ID 配对。

| 原生调用 ID | 工具 | 真实结果 |
| --- | --- | --- |
| `stakpak_probe_0` | `rustcodegraph_query` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `stakpak_probe_1` | `rustcodegraph_callers` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `stakpak_probe_2` | `rustcodegraph_node` | `packages/runtime/src/hooks/useApp.ts:4` 转调 `_$createApp(factory)` |

启动期 HTTP 路径能进入 Stakpak 的原生工具与审批循环。正式接入依赖公共宿主提供经审查的会话级 HTTP MCP 转发入口，由宿主绑定所选租户和挂载、执行授权与撤销、持有并释放后端连接。当前模型中继没有这条路由，Goose 和 Hermes 的 stdio 目录也不提供 HTTP 服务。待公共入口可用，再将选中的入口绑定到 Stakpak 私有启动配置，使用指定阿里模型复验三轮。

本轮仅变更验收文档，未运行 Cargo。私有网关、转发器和探针均已退出，挂载、私有配置及数据库已清理。原始 SSE 和诊断留在本机私有目录。

### 宿主单次授权补丁探针

在同一受审 Stakpak 0.3.88 和原沙箱下，给私有 HTTP 转发探针加上独立随机令牌及宿主审批门禁。收到 ACP 的 `allow_once` 后，转发端只允许对应工具和完全一致的参数通过一次。探针核对原生会话 ID，并用三种固定工具的标题映射验证原型。正式接线需要以受审的结构化工具身份绑定授权。

- 原生批准轮成功转发三次真实图谱调用，三个输出按原调用 ID 回到模型续轮。原生取消轮收到三次权限请求、零图谱调用，并返回三个 `TOOL_CALL_REJECTED`。两轮以 `end_turn` 收尾。
- 错误令牌返回 HTTP 401。持正确令牌但提前调用、改变获批参数或重放已消费调用均返回 403。会话撤销后，以原令牌请求返回 410。退出后删除临时 MCP 配置。
- 此次只运行隔离补丁探针。生产接线仍依赖宿主会话级转发和结构化审批绑定。真实阿里模型三轮和 Cargo 定向测试尚未复跑。

### 受管启动器接线

Stakpak 0.3.88 v2 复用已受审的 Poolside 回环桥和 Cortex stdio 沙箱。宿主投影选中服务的工具策略。挂载发生在首个新会话时，启动器用私有 HTTP 配置重新启动原生进程，复核原生 ACP 握手，再创建会话。挂载进程的模型目录仅声明通过策略的 MCP 工具，以避免 Stakpak 原生工具标题冲突。无挂载和撤除仍声明原有三种内置工具。挂载仅限新会话，退出时撤销连接并清除私有 MCP 配置。

受管原生探针在旧启动器下未取得三种图谱输出。以固定发行包安装 v2 校验五个文件摘要后，四个原生场景均以 `end_turn` 结束。无挂载与撤除零图谱声明。批准轮三个真实结果分别为 `app/app.tsx:32`、`app/app.tsx:39` 和 `_$createApp(factory)`。拒绝轮保留三条原生审批和三个拒绝结果。定向 Python 测试一项通过，覆盖会话及工具调用 ID 约束、参数匹配、单次消费、迟到批准和撤销。RustCodeGraph 验证使用私有索引的只读 SQLite 事务保留 WAL 和 SHM。

发行下载超时后，使用 SHA-256 相同的本机归档复验安装，保留解包及所有摘要校验。共享仓库已有重型 Cargo 验证运行，本轮未启动并行 Rust 构建。指定阿里模型三轮需包含 v2 的共享网关产物。

### 受管 v2 指定模型原生复测

使用五个摘要匹配的 Stakpak 0.3.88 v2 安装、固定 RueJS 提交、原提示和指定阿里模型。诊断中继仅将固定模型转发到既有供应商端点，Stakpak 持有私有回环令牌。无挂载和撤除分别在原生会话 `2e6331ab-0ef3-406a-acb6-5f5785280329`、`bfdf907f-987f-4878-b888-2caa053f7000` 获得 `end_turn)，精确回复 `MCP_NOT_AVAILABLE`，均为零工具和零审批。

| 挂载轮 | 原生会话 | query / callers / node 完成并返回模型 | 终态 |
| --- | --- | --- | --- |
| RustCodeGraph 1.2.10 | `7237949a-aa09-4582-bc0e-1b394075d70d` | 185 / 42 / 191 | 720 秒内未收到提示终态 |
| RustCodeGraph 1.2.11 | `67fef109-992a-47c4-9754-d237fcba9186` | 109 / 44 / 310 | 720 秒内未收到提示终态 |
| 1.2.11 加限定工具元数据 | `a8d837ba-c486-4cf6-9962-642b4e8147bb` | 63 / 55 / 311 | 720 秒内未收到提示终态 |

三个挂载轮仅声明并调用三种指定工具。模型收到原图谱结果，反复查找已读过的路由与应用工厂。第三轮在私有中继的模型目录补充分页及现有入口说明，明确 `projectPath` 可省略且只应指向索引根。调用参数、工具返回和原提示均保持原值。一些工具的原生状态为已完成，实际返回仍可能是找不到文件或索引的说明。挂载轮未给出完整调查，因此真实模型严格验收未通过。

另复跑单次审批 Python 用例，1 项通过。以上原生结果无 ShiJu 网关 SSE、UI summary 或 ACP options。

### 官方工具共存与网关启动目录

共享网关 SHA-256 `109066ea0b79ce6edfc4399624715f0dafa80d397adabe7a1ae46158407e7cba` 从仓库根启动时，v2 在 `session/new` 拒绝挂载。独立启动观察确认进程未进入私有 MCP 配置步骤。根因是宿主未将项目 cwd 交给受管 Stakpak 进程。将同一私有网关从 RueJS 工作目录启动后，旧提示挂载会话 `ses_1f191fc7d0010CewV1uaUghjRJ` 成功建立，网关记录三种图谱工具完成及输出。旧提示的挂载流后续被取消，采用新版验收请求接续验证。

补丁让宿主将受管 Stakpak 启动目录绑定到会话 cwd。v3 在挂载时继续声明原有 `view`、`create`、`str_replace`，官方工具原生审批和文件回调沿用原处理。MCP 调用仍需匹配活动会话、调用 ID 和获批参数，只能领取一次执行。拒绝标题和原生文件工具身份冲突的外部目录。固定安装包验证五项摘要，四个原生夹具场景及两项 Python 聚焦测试通过。

新版提示 SHA-256 为 `18806f1a927d3587ee5c685107c89b5dbaeea09614efe9a327e293b649967f38`。Stakpak v3 原生真实会话 `cdf880b5-a2e2-4b1e-a65c-c7c089711041` 同时声明三个官方工具和三种图谱工具。挂载轮实际完成并回传 query 1 次、callers 1 次和 node 9 次，并以 `end_turn` 结束。答复引用 `app/app.tsx:32` 的工厂、同文件第 39 行的挂载入口、`useApp.ts:4` 对 `_$createApp` 的转调。撤除原生会话 `86b89be4-a6d5-4a07-8301-0fb71b1cd47c` 使用五次官方查看，无图谱注册或调用，正常结束。无挂载原生探针因官方查看尝试读取探针拒绝的路径而终止。该探针为诊断，不代替新版网关三轮。
