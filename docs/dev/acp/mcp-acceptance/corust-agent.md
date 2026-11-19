# Corust Agent 动态挂载 RustCodeGraph

2026-11-19，已阻塞。指定阿里模型的三轮会话均收到正常终态。挂载轮的宿主已连接 RustCodeGraph，Corust 仍答复 `MCP_NOT_AVAILABLE`。独立 ACP 探针把首个可复现断点定位在原生建会话收到 MCP 配置后、向模型声明工具前。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境

- 私有回环网关使用已有 `shiju 0.6.0` 构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`，健康检查 HTTP 200。仅在私有实例启用预设和配置已授权租户，使用 `10001 / 100010005`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model`、`full_access=true`。
- Darwin arm64 上复用受管 Corust v0.6.0。启动器 SHA-256 为 `1411b71e5562a579d862572affe8283e5b55bd639fa9f4f4cce7501fd93608c5`，原生程序为 `a2a644a851ee03c583ed9f113cc3acbf54d99ff9f8aaaa782535dbd576d97a2e`，两者与受管清单一致。官方发布列表将 v0.6.0 标为最新版，本轮复用仓库固定版本。[Corust 官方发布列表](https://github.com/rustingai/corust-agent-release/releases)
- 请求提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本来自 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c`，索引 3478 个文件，`createRueSiteApp` 命中 `app/app.tsx:32`。挂载轮使用原 RustCodeGraph stdio 命令与服务名 `ruejs_graph_probe`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复 / 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef96c4dc001vgWxjvsD5qeOy4` | 200 / 0 / 3.776 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` / 0 |
| 挂载 | `ses_1ef96d4bf001qY2Y4V9N2ZgmOP` | 200 / 0 / 5.598 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` / 0 |
| 撤除 | `ses_1ef96ebc7001BEpysD2Rca9HOz` | 200 / 0 / 3.990 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` / 0 |

三轮均无 `chat.error`。UI summary 回读 `acp-corust-agent / corust-agent`、`full_access=true`。ACP options 在运行中和终态都回读指定提供商、完整模型 ID 及 `session_model`。连接期间权限为 `client_only`，终态连接关闭。撤除对照满足标记及零工具要求。挂载轮无 `query`、`callers`、`node` 调用或输出，未取得调查答复和准确性证据。

挂载轮网关记录宿主 MCP 客户端初始化 RustCodeGraph 成功。客户端实现通过 `NewSessionRequest::mcp_servers` 转发会话配置。本轮网关未抓取原始 ACP 请求体。

## 原生探针与权限

- 用同一受审启动器和独立回环模型替身捕获模型侧工具声明。原生 `initialize` 返回 ACP v1，标识 `corust-acp` 0.1.0，`loadSession=false`，`mcpCapabilities.http=false`、`sse=false`。
- 实际向每个新 ACP 进程发送 `session/new`，分别携带空列表、同一 RustCodeGraph stdio 项、HTTP 项和 SSE 项。四次均返回新会话和默认审批模式，短提示均以 `end_turn` 结束。请求落到 `/v1/onerouter/chat/completions`，四轮各声明相同的 13 个内置工具，目标图谱工具为零。两个远程变体的测试 MCP 路径收到零请求。探针验证工具声明边界，真实模型结果以三轮网关证据为准。
- 受管入口禁止派生子进程，出站网络限于请求绑定的模型中继。另用固定回环工具调用复测原生审批，三次均收到 `session/request_permission`。批准工作区内写入后成功，拒绝后未写入，批准工作区外路径后仍未写入。受管启动和审批策略保持原样。

## 复跑条件与清理

需有可在受审沙箱内将会话工具声明交给模型的 Corust 发行版或受管入口，再按原请求跑三轮。当前网关能初始化宿主 stdio 服务，公开原生版本在受管模式下缺少可用的 MCP 工具入口。

网关已退出，端口关闭。私有副本的挂载配置已撤除，与导出快照比对的 4108 个普通文件内容一致。私有租户、配置及运行目录已清理，未发现本次 RustCodeGraph 子进程残留。原始证据留在本机 0700 目录。提交仅含验收文档，未修改 ACP 生命周期或转发代码，未运行 Cargo 或桌面构建。

## 共享工作区复验

复用共享槽位已有的较新 `shiju 0.6.0` 构建副本，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。Corust 入口及受管摘要与首轮相同。官方 release API 仍公布 v0.6.0（Darwin arm64 资产 `419147804`）。[Corust v0.6.0](https://github.com/rustingai/corust-agent-release/releases/tag/v0.6.0)

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复 / 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f00eb2cc001ACDsHJqi4LfziX` | 200 / 0 / 2.793 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` / 0 |
| 挂载 | `ses_1f00ebe6f001t8BI372OJ9Mzlb` | 200 / 0 / 3.458 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` / 0 |
| 撤除 | `ses_1f00eccdc001BpOxiv2TVVp1JQ` | 200 / 0 / 2.585 秒 | `chat.done` ×1 | `MCP_NOT_AVAILABLE` / 0 |

三轮 summary 重读确认 Corust 绑定和 `full_access=true`，终态 options 均保留指定模型，连接已释放。挂载轮宿主初始化 RustCodeGraph 1.2.10 成功，活动会话权限为 `client_only`。挂载配置已撤除，私有网关、MCP 连接与原生探针均已退出，私有租户配置已清理。

- Cortex 的受管桥接依赖原生 `_meta.clientTools` 注册和执行回调。保留 Corust 沙箱，在私有回环探针向其 `session/new` 发送同样结构。模型请求仍只有原生 13 个工具。回环模型返回显式的图谱工具调用后，Corust 将 `ToolNotFoundError` 送回下一轮模型请求，未发出该工具的 ACP 开始事件或审批回调。正对照 `read_file` 有完成事件，下一轮读回 `app/app.tsx` 内容。Snow 和 Cursor 的受管补丁同样依赖各自可修改的原生工具注册与执行路径。Corust 的固定发行仓库存放发行资产和扩展配置，目前缺少可复用的原生注册与结果回调入口。
- 复用 Cortex 的独立 MCP 沙箱做图谱探针。源码与索引均只读时，本次副本的 RustCodeGraph 返回空工具目录并报告索引不可用。仅在临时探针中把一次性源码副本的派生 `.rustcodegraph` 索引设为额外可写后，三个真实图谱调用均成功。query 返回 `createRueSiteApp`（`app/app.tsx:32`），callers 返回 `mountRueSiteApp`（`app/app.tsx:39`），node 返回 `useApp` 调用 `_$createApp` 的源码。对副本外原 RueJS 索引的请求返回数据库不可打开。源码仍只读，子进程和出站网络继续禁用。独立调用的结果未进入 Corust，原受管沙箱未改。
- 将未知工具直接加到模型中继的工具声明无法接上 Corust 原生的调用、审批和结果处理。实现中继自行执行并合成回合会改写本任务的原生执行边界。保留已阻塞，所需外部条件为支持会话级工具注册、审批回调和结果续轮的 Corust 发行或可审查的原生接口。取得该入口后可复用独立 stdio 进程，按原三轮验证。此次仅更新证据，未运行 Cargo。

## 原生二进制补丁探针

官方 v0.6.0 标签树含发布工作流、扩展配置和许可证。固定 Darwin arm64 资产中的可执行文件保留了 ACP 建会话、Rig 工具注册及执行符号。未找到对应的 Corust Rust 源码或可加载的工具插件入口。[官方 v0.6.0 标签](https://github.com/rustingai/corust-agent-release/tree/v0.6.0)

在私有副本上做等长工具名补丁并本地重签。仍通过现有沙箱启动器执行，模型只连接回环夹具。原二进制和受管清单未改。对照二进制向模型声明 `read_file`，读取测试文件后有配对的 ACP 开始与完成事件，内容进入下一轮模型请求。补丁二进制向模型声明测试别名，别名同样读取文件并成功回传结果，ACP 工具事件为零。说明模型工具声明和执行路径可随静态注册名变化，ACP 事件处理还有单独的依赖。

两组都尝试在建会话的 `_meta.clientTools` 加入图谱工具；模型请求均未声明它，显式调用均返回原生 `ToolNotFoundError`。本探针未接入新的图谱处理器。仅替换工具名或能力声明无法提供会话注册、独立审批、撤销及可核对的事件。任务继续等待能同时接入原生工具注册、执行和 ACP 事件的源码或扩展接口。所有探针均在本机私有目录完成，本轮未改运行时代码、未跑 Cargo 或真实模型三轮。

## 受管 v3 补丁候选

补丁在 Corust 受管入口内接入请求级回环中继。挂载轮用已索引工作区的私有 SQLite 快照运行独立 RustCodeGraph stdio，只发布三个只读工具。原生进程仍执行原沙箱。新工具调用通过 ACP 宿主单次审批，拒绝、只读模式和过期审批均不能执行。原生工具走原有路径。达到工具续轮上限时，模型从实际工具证据生成最终答复，并保留原用户请求。

原版受审入口的回环探针复现工具未注册。候选版通过无挂载、批准挂载、拒绝挂载、审批中取消、只读模式及撤除六个场景。三种工具的开始、审批和结果事件匹配调用 ID，模型续轮收到图谱结果。取消后到达的批准不再触发工具。私有运行目录经正式安装脚本校验后发起测试。

同一提示与指定阿里模型的独立 Corust ACP 诊断中，三轮均收到 `end_turn`。挂载轮由 12 次图谱调用取得 11 个成功结果和一次不存在的 `app/router.tsx` 错误，三个必需工具均成功。回复覆盖调用链、浏览器与 SSR 守卫、挂载容器、插件、布局和快照影响，明确标注路由实现没有验证。无挂载答复在标记后附加说明，撤除答复为 `MCP_NOT_AVAILABLE`。真实模型诊断未走正式网关，严格三轮验收仍待完成。候选变更保留在工作区，未提交。

最终候选经安装脚本再次验证四个文件摘要，六个聚焦场景通过。真实模型复跑仍有三轮原生 `end_turn`，挂载轮再次完成三种工具、共 11 次成功和 1 次相同的错误路径调用。此次无挂载对照只答标记，撤除对照附加说明。两次诊断都使用同一固定提示，保留实际模型输出。

正式网关复验使用共享构建产物，内嵌摘要与当时的四个候选安装文件完全匹配。新版请求 DTO 拒绝旧顶层 `acp_agent`，原请求三轮 HTTP 422。去掉这个字段后，由 `agent=acp-corust-agent` 绑定相同后端，原提示、租户、模型、目录及权限均相同。三轮 HTTP 200，各收到一次 `chat.done`。挂载轮仍无工具，三轮均答 `MCP_NOT_AVAILABLE`。summary 与 ACP options 确认了正确绑定及模型。

用同一网关的私有回环供应商再次定位断点。无挂载与撤除轮的真实出站请求各保留四个原生工具，挂载轮出站工具列表为空。Corust 模型中继的原有白名单过滤了三种新声明。工作区内补上三个受管图谱名称，并保留原生四项白名单及其他工具拒绝。宿主的 `allowedTools` 和 `deniedTools` 经 ACP 元数据转发，受管入口只声明选中的只读工具。正式安装后的七个聚焦场景通过，包括宿主显式拒绝 node 时只向模型声明和执行其余两项。修正后的真实网关挂载轮仍待确认，按用户新标准以执行和结果回传为准。未提交。
