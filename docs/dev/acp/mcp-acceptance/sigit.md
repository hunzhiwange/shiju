# siGit Code 动态挂载 RustCodeGraph

2026-11-19，首轮验收阻塞。受审 siGit 1.7.10 在建会话前被宿主拒绝。原生探针确认，开启显式 stdio MCP 会在审批前启动服务并继承中继环境，获批调用仍缺少受审执行路径。

## 环境与请求

- Darwin arm64，私有回环网关使用已有 ShiJu 0.6.0 构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。从已有受审安装复制原生入口及其收据至私有网关。入口 SHA-256 为 `b98bdfdc85884922297704df67d5b9441a8fd399352457bfd75cc09f86934520`，与收据一致。源码与补丁摘要匹配受管清单，原生握手自报 1.7.10。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本。RustCodeGraph 1.2.10 索引了 3478 个文件。挂载轮通过副本的 `.vwacprc.json` 选择原生 stdio 服务 `ruejs_graph_probe`，参数包含 `--no-watch`。
- 每轮使用同一 curl 请求独立建会话。租户为 `10001 / 100010005`，绑定 `acp-sigit / sigit`，指定 `session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 和 `full_access=true`。提示取自 `189d22dc1`，UTF-8 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

## 三轮网关结果

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f00ed092001W2TaJMm8jLlBOI` | 200 / 0 / 7.455 | 一个 `chat.done`，去除 `<think>` 段后最终答复恰为 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1f00eefa4001lAx8ZqOiDeL57T` | 200 / 0 / 4.632 | `chat.model_recovery` 的 `phase=stopped`，随后一个 `chat.error`。无模型答复。 |
| 撤除 | `ses_1f00f02af001e4NxDf6yNva8R1` | 200 / 0 / 6.925 | 一个 `chat.done`，去除 `<think>` 段后最终答复恰为 `MCP_NOT_AVAILABLE`。 |

挂载轮首个失败边界为 `validate_mcp_contract`，原因为 `siGit reviewed host-only runtime does not support MCP servers`。网关未发送携带 MCP 的 `session/new`。三轮全部为零工具事件，目标查询、调用方查询和节点查询均未在指定模型会话中执行，无工具结果或调查答复可核对。

三轮 UI summary 均确认绑定及 `full_access=true`。两轮对照的 ACP options 在线确认 `connected=true`、指定模型以及 `support=client_only`。挂载轮仅确认受审发行版本，`connected=false`，无实际选模及工具审批。

## 原生边界和独立工具证据

- 以相同摘要的受审入口直连隔离 ACP 宿主，模型使用私有回环夹具。握手为 ACP v1，声明 HTTP MCP 和 `sse=false`。保留驱动的 `SIGIT_MCP=off` 时，显式 stdio 列表可进入原生建会话，但服务未启动，模型只收到五种受审宿主工具。固定源码的 `connect_session_servers` 在该环境值下忽略显式服务。
- 仅在隔离探针中移除该环境开关后，真实 stdio 服务在 `session/new` 内启动，早于模型提示和工具审批。启动探针确认子进程继承了模拟中继 bearer。模型收到 RustCodeGraph 的工具定义并请求查询，siGit 发出 `session/request_permission`。宿主选择 `allow_once` 后，原生通知虽标记 `completed`，模型续轮收到的实际结果为 `Error: tool mcp__ruejs_graph_probe__rustcodegraph_query requires a verified host route; native fallback is disabled`。HTTP 使用同一受审工具分发路径。现有 Cortex 转换依赖 Cortex 的 `clientTools` 回调，siGit 没有对应宿主执行入口。
- 新起进程以空列表建会话，模型侧目录不含 MCP 工具。夹具请求 `read_file`，宿主选择 `reject_once`。原生工具状态为 `failed`，模型收到拒绝原因，宿主未收到文件读取回调。
- 独立直连同一个真实 stdio 服务，`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三次调用成功。这些输出仅来自独立 MCP 客户端。

保留驱动、受管补丁、协议门禁及现有断言。后续需要受审的会话级执行入口，确保 MCP 启动不继承中继凭据、调用使用宿主审批，并将实际结果及错误状态回传模型。达到这些条件后复跑三轮与审批回归。

私有网关、原生入口及工具进程已退出，临时挂载与私有凭据副本已清理。原始记录留在本机 0700 目录，用户网关健康检查通过。

## 定向回归

`python3 scripts/cargo_slot.py cargo test -j 1 -p acp-client --lib acp_onboarding_sigit_mcp_selection_rejects_before_new_or_load` 退出码 0，实际执行 1 项并通过，318 项被过滤。该用例确认非空选择在 `session/new` 和 `session/load` 发送前失败。复用 Cargo 槽位 3，`CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR` 均指向仓库根目录下的 `target/rust-slot-3`。槽位已释放，缓存保留。

## 共享工作区复验

在独立网关换用共享工作区已有 ShiJu 0.6.0 构建，SHA-256 为 `f46699dd46d8520d8a7cfc3324132ccd5a2e05857e89fde160052fdcf75add3c`。沿用相同审核版 siGit 入口、提示、模型、租户和 RueJS 提交。同一 stdio 服务仅在挂载轮启用。

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态与答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0260df7001V6lGifrPQPXs42` | 200 / 0 / 8.007 | `chat.done`，最终答复为 `MCP_NOT_AVAILABLE`。 |
| 挂载 | `ses_1f0262ebd001GELGCS4sul0INc` | 200 / 0 / 4.742 | `chat.model_recovery` 的 `phase=stopped`，随后 `chat.error`，未发送携带 MCP 的建会话请求。 |
| 撤除 | `ses_1f026437a0017PuXweykgbMSYJ` | 200 / 0 / 7.219 | `chat.done`，最终答复为 `MCP_NOT_AVAILABLE`。 |

复现了 `siGit reviewed host-only runtime does not support MCP servers`。三轮均无工具事件，UI summary 均确认绑定和完全访问请求。两轮对照在线确认指定模型及 `client_only`。挂载轮没有有效连接、选模、工具结果或调查答复。GitHub 的最新发布仍为 v1.7.10。[上游发行记录](https://github.com/getsigit/sigit/releases/tag/v1.7.10)

- 独立调用现有 Cortex 的 stdio 沙箱执行器。新建索引缺少 SQLite WAL/SHM 边车文件时，真实 RustCodeGraph 握手成功，`tools/list` 返回空列表。保持同一只读策略，由宿主持有私有索引的 SQLite 读事务后，三种工具均成功并返回首轮记录的相同符号和路径。传入工作区之外另一份已索引副本的 `projectPath` 未读到查询结果。仅为定位原因，另在私有副本中验证放行两个 SQLite 边车文件的写入也能恢复三个工具，产品未采用该授权。
- 直接借用 Cortex 或 DeepAgents 的执行器仍缺少 siGit 侧注册工具及回传结果的回调。Cortex 使用 `_meta.clientTools`，DeepAgents 使用自己的工具注册和分发路径。siGit 受审版本收到了 MCP 定义以后，只能在原生端启动连接，且获批后在工具分发处拒绝原生执行。原生 stdio 库增加配置环境项时延续父进程环境，缺少隔离启动入口。HTTP 路径共享工具分发限制，同一个 RustCodeGraph 二进制的 MCP 启动入口只提供 stdio。
- 挂载仍需一个经审查的宿主持有执行入口，绑定会话的工具清单与每次审批，隔离服务进程环境，在撤除、取消和关闭时回收，并返回原始工具结果。可在支持该合同的新版受审运行时到位后复验，或扩展本计划范围，新增 siGit 至 ShiJu 宿主的工具注册及执行回调。本轮复用现有构建及本机探针。协议门禁与断言无改动，定向测试结果沿用上节。

私有网关、沙箱服务及 SQLite 读事务已关闭，临时挂载和凭据副本已清理，用户网关健康检查通过。

## 受管 v2 补丁与原生聚焦验证

受管 siGit 1.7.10 的 macOS 会话模型连接接受显式、绝对路径且无环境覆盖的 stdio 服务。只读沙箱以内启动服务并清理环境，拒绝写入、联网、派生进程及工作区外的私有读操作。工具调用沿用逐次 ACP 审批，后端错误保留失败状态。原有官方工具继续按原权限共存。宿主拒绝远程传输及未审启动器。

仓库安装脚本在私有环境中完成固定归档的下载、校验、打补丁、构建和落盘。受管 v2 入口 SHA-256 为 `419345c5123bc8a5b6c76c115739921a5a195db2d912f9e45ac3cf5598d00c7f`，回读与构建收据一致。补丁 SHA-256 为 `8869d176e615e95b4fddd315a99462881755368fc3d88002b1307470310f3248`。

- 同一回环模型夹具在旧入口的挂载轮确认五种官方工具可见，缺少三个目标工具，断言失败。受管 v2 入口通过无挂载、批准挂载、拒绝挂载、移除、空列表重新加载、挂载列表重新加载、原有读取拒绝七个场景。
- 批准挂载轮各发出一次单次审批。查询命中 `createRueSiteApp`（`app/app.tsx:32`），调用方命中 `mountRueSiteApp`（`app/app.tsx:39`），节点返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。三次调用均收到 `completed`，模型续轮读到三份真实输出。同一挂载会话额外请求官方 `read_file` 并获宿主拒绝，未发出文件读取回调。拒绝挂载轮的三个 MCP 调用均为 `failed`，模型得到拒绝结果。
- RustCodeGraph 1.2.10 的只读索引需要 SQLite WAL/SHM 边车文件。探针在私有索引使用宿主持有的读事务准备边车，不向 MCP 服务授予写权限。
- `python3 scripts/cargo_slot.py cargo test --locked -j 1 -p acp-client --lib acp_onboarding_sigit_mcp_selection_rejects_before_new_or_load` 退出 0，实际 1 项通过，322 项过滤。无效 HTTP 及未受审 stdio 启动器均在建会话和加载前被拒绝。安装和定向测试复用槽位 3，两个 Cargo 缓存路径均为 `target/rust-slot-3`。

上述聚焦结果来自实际安装入口和回环模型。

## 指定真实模型网关验收

2026-11-19，受管 v2 按新版共存标准通过三轮。共享工作区定向构建的 ShiJu 0.6.0 二进制 SHA-256 为 `28793fbd08778fb50798533905a8aeb09dea69d4cdfd1589c7f9f76fa5255419`。`cargo build --locked -j 1 -p vw-cli --bin shiju` 使用槽位 3，退出 0，耗时 17 分 28 秒。私有网关使用已核验的 v2 入口、相同 RueJS 提交和 3478 文件索引。宿主持有私有 SQLite 读事务，服务使用原生只读 stdio。

三轮使用新版提示，UTF-8 SHA-256 为 `18806f1a927d3587ee5c685107c89b5dbaeea09614efe9a327e293b649967f38`。三轮均确认 `acp-sigit / sigit`、`full_access=true`、运行时连接、`client_only` 和指定 `session_model` 的 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态 | 成功工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f1d5bea50016iOK9tVa8xCDdJ` | 200 / 0 / 245.129 | 一个 `chat.done` | 26 次官方工具，目标服务零次。 |
| 挂载 | `ses_1f1d97e73001H7GmtIOYH6RntR` | 200 / 0 / 23.637 | 一个 `chat.done` | 9 次目标服务调用，1 次官方读取。 |
| 撤除 | `ses_1f1d9dc2d00117ciekmCKxArQ6` | 200 / 0 / 45.386 | 一个 `chat.done` | 8 次官方工具，目标服务零次。 |

全部轮次无 `chat.error`。两轮对照没有提供挂载定义。官方 shell 工具曾在本机配置和历史日志内检索服务名，撤除轮由此读到历史记录。用当前会话调用 ID 和完全匹配的服务工具标题判断本轮执行，两组对照均无目标工具事件。

挂载轮 `ruejs_graph_probe` 的目标调用均有同 ID 的启动和完成事件，`payload.output` 保留返回结果。

| 工具 | 调用 ID | 原始结果 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_1555ebaf83644d3685810a51` | `function createRueSiteApp app/app.tsx:32` |
| `rustcodegraph_callers` | `call_7f7745d90aaf47f59b35db44` | `function mountRueSiteApp app/app.tsx:39` |
| `rustcodegraph_node` | `call_ad84d9bc618445149bd2c412` | `useApp` 位于 `packages/runtime/src/hooks/useApp.ts:4`，实现转调 `_$createApp`。后续节点调用返回运行时工厂源码。 |

模型答复引用了三个结果及实际行号，追踪了 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`，并说明了浏览器分支、容器缺失、快照清理和插件安装。挂载轮的官方 `read_file` 调用成功读取 `useApp` 源码，原生聚焦轮验证同一工具也可由宿主拒绝。

私有源文件和符号链接与归档逐项一致。临时挂载和凭据副本已清理，私有网关与目标服务已退出，用户网关健康。原始证据保留在本机 0700 目录，构建槽位已释放。

## 受管 v3 自动准备 RustCodeGraph 索引

siGit 启动显式挂载的 RustCodeGraph 前，会自动检查所选工作区的现有索引并补齐缺少的 SQLite 辅助文件。系统 SQLite 进程仅能写入两个辅助文件，查询完成后即退出。MCP 服务沿用只读沙箱、隔离环境和原有宿主审批。工作区现有 MCP 配置可以直接复用。

固定源码的私有安装使用槽位 3，两个 Cargo 缓存路径均为 `target/rust-slot-3`。v3 入口 SHA-256 为 `322287c3fc4b2b88434f66464f56ea55f086d151b0c2682a44633defe90d34ea`，安装收据回读一致。补丁 SHA-256 为 `3884ccd9580594a612ebf06de5636b62bd749ed1098b89b0c4c8a7e5c2440c4e`。

- v2 对照使用刚生成、没有 SQLite 辅助文件的索引。无挂载通过，挂载在 `session/new` 返回 `siGit MCP service failed to expose tools in the read-only sandbox`。本轮夹具没有打开 SQLite 读事务。
- 使用同一索引和夹具，实装 v3 自动准备后，三个工具均获批、执行并把真实符号和源码返回给模型。无挂载、拒绝挂载、撤除、空列表及挂载列表重载、官方读取拒绝均通过。额外验证将 WAL 路径替换为指向工作区外文件的符号链接时，建会话拒绝，目标文件内容保持一致。运行前后主索引 SHA-256 一致。
