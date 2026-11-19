# Junie 动态挂载 RustCodeGraph

2026-11-19，受管 Junie v2 按新版标准通过网关三轮验收。官方工具沿原权限共存。三种挂载工具均实际执行，结果进入模型续轮。

## 环境

- Darwin arm64，受管 Junie 26.11.2（4100.5）。原生入口 SHA-256 为 `4be1f51971aa638ebd9c3caa93e63d2a235e3a4bd133011476f6cbbbcfcc565e`，启动器 SHA-256 为 `db3c895af5065c401f7798385d27619e0aa4ef5c227b048b84e0b090c71f2fad`，均与仓库契约一致。原启动器通过签名检查。独立握手返回 ACP v1、`@jetbrains/junie` 26.11.2（4100.5）、`http=true` 和 `sse=true`。
- 私有回环网关复用 `shiju 0.6.0`，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。仅在私有实例启用预设，使用租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `session_model`。
- 请求取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出私有副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载选用原生 stdio。 [复用请求](../ACP验收.md#可复用的验收请求)

## 三轮真实请求

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与可见答复 | 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efb082c0001OSlx7UueleK4qk` | 200 / 0 / 14.237481 | `chat.done`，`MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1efb0bc070017eUNfft1Vx1Pj2` | 200 / 0 / 13.300183 | `chat.done`，`MCP_NOT_AVAILABLE` | 1 条原生服务启动失败事件 |
| 撤除 | `ses_1efb0f0a300124c3V500RLZlGT` | 200 / 0 / 13.985039 | `chat.done`，`MCP_NOT_AVAILABLE` | 0 |

三轮均无 `chat.error`。运行中和终态的 UI summary 均确认 `acp-junie / junie` 与 `full_access=true`。ACP options 在运行中为 `connected=true`，权限状态为 `client_only`，终态连接释放。两阶段均读回指定模型、提供商和 `session_model` 来源。受管会话进入 prompt 前须确认 `brave_mode=off` 和模型，三轮均进入推理，未出现原生审批事件。

挂载轮宿主的 RustCodeGraph 客户端已完成初始化。Junie 随后为同名服务发出工具事件 `dea786f3-4f30-4e69-9a85-ba15ab1f4248`，标题为 `ruejs_graph_probe failed`。输出报告 `state: FAILED`、无法执行 RustCodeGraph 和 `posix_spawn failed`。事件外层状态为 `completed`，对应服务失败通知已完成。三种目标工具均无调用或结果，答复没有调用链、文件行号或快照分析。挂载轮未达到验收标准。

## 首个失败边界

- 受管启动器禁用默认 MCP 发现。JetBrains 支持 ACP 会话传入的服务及显式 `--mcp-location`，两条路径的 stdio 服务都需要启动子进程。现有 Seatbelt 策略只允许执行固定的 Junie 原生入口，并将出站连接限定为会话模型中继端口。 [JetBrains MCP 配置](https://junie.jetbrains.com/docs/junie-cli-mcp-configuration.html)
- 原生 HTTP 和 SSE 能力已通过握手核实。当前 RustCodeGraph `serve --help` 仅提供 stdio，现有模型中继端口只承载模型协议。仓库中的测试桥接夹具转发原始 stdio 字节。当前缺少能在受管权限与会话范围内转发 RustCodeGraph 的可用 HTTP 或 SSE 入口。
- 未修改生产代码或审批策略。编号任务给出的生命周期回归未运行，此次失败发生在原生子进程启动边界。私有网关已退出，端口关闭，撤除轮前已删除挂载配置。后续在具备会话级兼容入口后复跑相同三轮。

## 共享工作区复验

复用较新的共享构建 `shiju 0.6.0`，SHA-256 为 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`。使用同一受审 Junie、RueJS 提交、指定租户与模型，在新的隔离网关重跑原请求。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f03a0d75001x2IXsTSYQYLSO6` | 200 / 0 / 14.562266 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1f03a47a2001t1afQs2L7DTJHk` | 200 / 0 / 13.702464 | `chat.done`，`MCP_NOT_AVAILABLE`，一条 `ruejs_graph_probe failed`（`posix_spawn failed`） |
| 撤除 | `ses_1f03a7ed6001OEPCZYNzbwvl5e` | 200 / 0 / 11.882137 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

三轮无 `chat.error`。UI summary 和 ACP options 均读回原后端、`full_access=true` 和原选模绑定，连接在终态释放。独立宿主 stdio 连接实际调用三个工具，分别读回 `createRueSiteApp` 所在的 `app/app.tsx`、调用者 `mountRueSiteApp` 及 `useApp` 对 `_$createApp` 的调用。Junie 挂载轮未读回这些结果。

- 用私有 `--mcp-location` 启动同一已签名原生程序，沿用原 Seatbelt 策略，显式 stdio 配置同样报 `posix_spawn failed`。因此配置来源对这个失败没有影响。
- 在原 Seatbelt 策略下，用临时回环 HTTP 夹具复验。配置为模型端口时，Junie 完成了 MCP `initialize`、`notifications/initialized` 和 `tools/list`，并向合成模型声明了该工具。配置为其他回环端口时，夹具收到零个 MCP 请求，模型未收到该工具声明。夹具仅用于传输诊断。
- 同端口夹具用 `brave_mode=off` 复验原生审批。Junie 针对相同参数和调用 ID 发出 `session/request_permission`。选择 `reject_once` 时收到零个 HTTP `tools/call`，原生工具事件为 `failed`。选择 `allow_once` 时收到一个 HTTP `tools/call`，原生工具事件为 `completed`。这些事件属于诊断工具。真实三轮的 RustCodeGraph 执行仍阻塞在 stdio 启动。
- 复核当前 Cortex 和 DeepAgents 的受管桥。它们分别使用 Cortex 专属 `_meta.clientTools` 与 DeepAgents 受补丁工具入口。独立调用 Cortex 的 stdio 执行组件时，在其私有 HOME 下连接成功但本机 RustCodeGraph 工具目录为空。模型中继当前没有承载 MCP 的路由。官方 ACP 正式发行仍列出 4100.5，较新的 ACP staging 与通用 CLI nightly 未作为受审运行时安装。 [ACP 发行](https://github.com/JetBrains/junie-acp-release/releases) · [CLI nightly](https://github.com/JetBrains/junie/releases/tag/4165.1)
- 后续需要决定是否扩展现有模型中继，在同一端口提供会话级 HTTP MCP 转换，或提供符合相同约束的受审入口。转换须连接同一个实际 RustCodeGraph，并核验工具策略、审批、撤销和会话销毁。当前网关和诊断监听已关闭，私有预设与凭据没有写入共享实例。

## 受管 v2 补丁

新版启动器把模型请求和显式会话 MCP 放在同一个私有回环监听端口。Junie 原生进程仍只可出站访问一个端口，原生子进程执行限制、默认配置关闭和 Safe 控制沿用。模型请求只转给同一次启动绑定的中继。显式 stdio 工具在独立的只读沙箱执行，保留项目目录边界、宿主工具允许及拒绝规则，并仅接受当前 ACP 提示中同名同参、批准一次的调用。取消、关会话和清空挂载会撤销批准。

在已签名 Junie 4100.5 与真实 RustCodeGraph 1.2.10 上，以回环模型夹具验证四个新会话。挂载前、撤除后无 MCP 声明或调用。挂载批准轮完成 `query`、`callers`、`node`，Junie 续轮获得 `app/app.tsx:32`、`mountRueSiteApp`、`useApp` 调用 `_$createApp` 的结果。挂载拒绝轮捕获三次原生审批，三个工具状态均为失败。四轮返回 `end_turn`。MCP 只读沙箱需要图谱的 WAL 和 SHM，探针在私有副本保留这两个文件。

`python3 patches/acp-model-source/junie/mcp_bridge_tests.py` 运行 4 项，通过同参一次授权、拒绝优先、迟到审批及活动取消、撤除后的 URL 失效。真实指定模型以新版网关三轮结果为准。

### 指定模型原生三轮

同一受审签名和安装内容，RustCodeGraph 1.2.11，原提示和 `deepseek-v4.1-flash-1017` 通过私有中继直连 ACP。上游凭据由中继持有。受管 Junie 只收到回环令牌，三种工具沿用同一原生审批。三个进程均退出 0。

| 轮次 | 原生会话 | 终态 | 工具 |
| --- | --- | --- | --- |
| 无挂载 | `session-261119-091935-14wt` | `end_turn`，`MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `session-261119-091948-thcl` | `end_turn` | query 25、callers 6、node 34 |
| 撤除 | `session-261119-092558-1pff` | `end_turn`，`MCP_NOT_AVAILABLE` | 0 |

挂载轮 65 次原生审批和对应的工具完成事件，无其他工具事件。首组 query、callers、node 调用 ID 是 `41509a2e-5a16-48ee-a125-12d974bc3f52`、`ef7b2cb2-109d-487b-b4cb-481de3af210b`、`972bbdcc-df4e-47bb-8547-d003fe5f2109`。模型续轮读回 `app/app.tsx:32`、`mountRueSiteApp`、运行时 `_$createApp` 及 `app/router/index.ts:787`。答复覆盖调用链、浏览器和 SSR、缺少容器时抛错、路由和 i18n、布局分支、快照残留推断及两项复验设想。根组件的 `/rust-layers` 条件属实。该路径未在此提交的路由表中直接注册，第二项场景仍需验证可达性。

定向命令 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_junie acp_onboarding_junie_session_lifecycle` 在槽位 10 运行，两条缓存路径均为仓库下 `target/rust-slot-10`。退出 0，1 项通过，2 项过滤。同槽位随后开始的重复网关构建已取消并释放。槽位 6 的共享网关构建已编译包含 Junie v2 摘要的受管注册库，随后因同期 MiMo trait 参数不一致在 `vw-agent` 报 `E0050`，退出 101，未生成新网关。旧产物没有 v2 摘要。原生三轮没有网关 SSE、UI summary 或 ACP options，正式网关三轮待回归。[共享构建记录](poolside.md#受管启动器补丁)

## 新版网关三轮验收

复用包含 Junie v2 安装摘要和工具策略投影的共享 `shiju 0.6.0`，二进制 SHA-256 为 `109066ea0b79ce6edfc4399624715f0dafa80d397adabe7a1ae46158407e7cba`。补丁为 `ad5f58fd5`，继续使用签名 Junie 4100.5、RustCodeGraph 1.2.11 及同一 RueJS 提交。原生 HTTP 连接到受管桥，桥使用实际 RustCodeGraph stdio。三轮复用新版提示，SHA-256 为 `18806f1a927d3587ee5c685107c89b5dbaeea09614efe9a327e293b649967f38`。该版本请求通过 `agent=acp-junie` 选择预设。初次复用旧请求体时，已移除的 `acp_agent` 字段触发 HTTP 422，未创建会话。改用新版请求后重新执行完整三轮。

| 轮次 | 网关会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f192a3ba001nm8W9GLETrOAFq` | 200 / 0 / 45.222375 | `chat.done`，原生 MCP 目录为空，目标调用和结果为零 |
| 挂载 | `ses_1f19355b4001OK6TEW4UIrN5qJ` | 200 / 0 / 67.384814 | `chat.done`，query 2、callers 1、node 3 次完成 |
| 撤除 | `ses_1f1945e4c001NAcvpRpZ9WIvbE` | 200 / 0 / 47.843102 | `chat.done`，原生 MCP 目录为空，目标调用和结果为零 |

三轮均无 `chat.error`。UI summary 均读回 `acp-junie / junie` 与 `full_access=true`。ACP options 在运行中为 `client_only`、`connected=true`，终态连接释放。两阶段均读回 `session_model` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

挂载轮的六个 SSE 调用均有开始和完成事件，服务身份为 `ruejs_graph_probe`。下面的调用结果与 Junie 模型续轮工具内容的原生记录一致。

| 工具 | SSE 调用 ID | 结果 |
| --- | --- | --- |
| `rustcodegraph_query` | `62ce35e1-c928-4a0a-9e2e-5c3b7d8d0509` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `67a77162-4f72-46bc-9bf6-493a10d2d08b` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `7fe4426e-342a-44e3-9854-250fde1edf1b` | 读回 `mountRueSiteApp` 节点，后续 node 读回 `useApp.ts:4` 和运行时 `AppFactory` |

挂载答复引用了上述图谱结果，并结合官方文件读取核对了 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`。挂载轮同时完成四次官方文件读取。对照轮仍使用了官方搜索和文件读取。撤除轮答复记录了尝试启动 CLI 时的 `posix_spawn failed`，未执行目标 MCP 工具。网关退出后端口关闭，私有项目挂载已移除，私有凭据已清理。本轮复用已记录的 Rust 及 Python 定向验证。
