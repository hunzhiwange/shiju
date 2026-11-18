# AutoHand Code 动态挂载 RustCodeGraph

## 环境与请求

2026-11-19，使用 [固定验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户为 `10001 / 100010005`，会话使用 `acp-autohand / autohand`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。借用已有受审构建启动私有回环网关，只在私有实例启用预设并写入租户认证。网关 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。

Darwin arm64 上已安装的 AutoHand CLI 为 `0.9.11 (46a28b16)`，以 `autohand --acp` 运行，入口 SHA-256 为 `339bdb0c8f4f8b7f9076108434d910b2cd5e14f2593b48c0e5858e04e91950eb`。独立握手返回 ACP v1 并声明 HTTP、SSE MCP 和会话加载、恢复能力。本次使用 `session/new.params.mcpServers` 中的原生 stdio，命令为 RustCodeGraph 的 `serve --mcp --path <私有副本> --no-watch`，服务名 `ruejs_graph_probe`。

RueJS 的 `main` 基准提交为 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。私有副本保留工作区已有源码修改，索引 3687 个文件，`createRueSiteApp` 命中 `app/app.tsx:32`。副本中 `app/app.tsx:10` 仍导入 `./router`，旧路由入口已删除，`app/hooks/router/index.ts:787` 有同名工厂。该工作区差异影响答复中的路径判断。

## 真实工具轨迹

挂载期间观测到 RustCodeGraph stdio 子进程。三个指定工具均有同一调用 ID 的开始和成功完成事件。

| 工具 | 实际输入 | 成功调用 ID | 输出节选 |
| --- | --- | --- | --- |
| `rustcodegraph_query` | `search=createRueSiteApp` | `call_6b4b528d1f7549a293d798ab:1` | `createRueSiteApp`，`app/app.tsx:32` |
| `rustcodegraph_callers` | `symbol=createRueSiteApp` | `call_6bc97c548ecb4e969ae5fed1:0` | `mountRueSiteApp`，`app/app.tsx:39` |
| `rustcodegraph_node` | `name=useApp`，`file=packages/runtime/src/hooks/useApp.ts` | `call_e2f96229baeb4af0a26d3dee:0` | 第 4 行调用 `_$createApp(factory)`，从 `../compiler-runtime/app` 导入 |

## 配置边界

AutoHand 发行版按显式配置参数、`AUTOHAND_CONFIG`、`AUTOHAND_HOME` 下的默认配置文件依次选取路径。ShiJu 受管驱动将 `AUTOHAND_CONFIG` 指向私有 `AUTOHAND_HOME` 下的 `config.json`。回环探针在两个位置配置冲突的模型和端点，ACP 实际选择显式配置的模型并访问其回环端点。原生 `newSession` 在创建会话 agent 后连接收到的 `mcpServers`。独立进程的无挂载和撤除探针均没有向模型声明该 MCP 工具。

发行版默认开启 `features.codeMode`。在该模式下，模型先收到 `tool_search`、`run_tool_script` 等入口，真实 MCP 调用可作为脚本的子调用发生。私有探针将 `features.codeMode` 置为 false 后，模型侧仍可见内置 `tool_search`。再尝试仅允许三个限定名称的原生 `--allowed-tools`，挂载轮的模型请求也没有收到目标 MCP 工具。该参数作用于进程运行范围，当前受管入口只接受已验证的启动参数。复用它需要先证明按会话隔离、三个选中的工具仍可调用，以及原权限策略继续生效。

## 三轮结果

| 轮次 | 新会话 ID | HTTP / curl / 耗时 | SSE 与工具对照 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef8d004c001tEz3NLaNn8vje1` | 200 / 0 / 10.857 秒 | 一个 `chat.done`，目标工具为零。执行三次 `tool_search`，在 `MCP_NOT_AVAILABLE` 后追加说明及回顾 |
| 挂载 | `ses_1ef8d2ba50013tEsP15UFq2e2R` | 200 / 28 / 300.006 秒，续传 200 / 0 / 43.807 秒 | 使用原请求标识和末事件 ID 续传，同一会话最终收到一个 `chat.done`。指定三种工具均有成功调用，另有额外工具和失败事件 |
| 撤除 | `ses_1ef926c79001JjiMuNWrUXNJKB` | 200 / 0 / 16.951 秒 | 一个 `chat.done`，答复为 `MCP_NOT_AVAILABLE`，目标工具为零。仍执行两次 `tool_search`、三次 `run_tool_script` 和两次 `tools_registry` |

三轮无 `chat.error`。UI summary 均回读 `acp-autohand / autohand` 和 `full_access=true`。ACP options 在执行中和终态都回读到指定 `session_model` 及完整模型 ID。执行中 `connected=true` 且权限为 `client_only`，终态连接已释放。没有人工批准动作，结束时待审批列表为空。另用关闭 Code Mode 的原生回环探针调用只读 `rustcodegraph_explore`，确认真实输出进入下一次模型请求。探针未收到该调用的 `session/request_permission`，因此无法实测宿主拒绝该 MCP 调用的行为。宿主权限策略和原生会话模式沿用现有设置。

挂载轮共有 76 次 `query`、6 次 `callers` 和 49 次 `node` 成功完成，另有 5 次 `node` 失败。首次工具失败为读取不存在的 `app/router.ts`。同时完成了 37 次 `run_tool_script`、一次 `tool_search`，以及 `status`、`files`、`explore`、`callees`、`impact` 等额外 MCP 调用。还有一次脚本执行失败。第一处合同偏差出现在无挂载轮的第一条工具事件，模型调用了内置 `tool_search`。

答复的 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp` 链路、浏览器的非 SSR 守卫、缺少 `#app` 时抛错、路由及 i18n 注册、`/rust-layers` 绕过布局均有源码证据。答复把保留快照与客户端节点重复、卸载后遗留快照写成待验证推断，并给出 DOM 计数和卸载验证两项场景。答复指出 `./router` 导入缺失，但把 `app/en22try-server.tsx` 称为实际启用的服务端入口。快照脚本仍引用已删除的 `app/entry-server.tsx`，现有证据只能确认站点入口的 SSR 守卫和副本中的候选模块。没有执行 RueJS 构建或浏览器场景。

## 结论

原生 stdio 的挂载、真实调用和撤除对照已有证据。任务保留为已阻塞，原因是原生工具发现与脚本行为超出提示的限定，且当前已验证的配置入口无法按会话限定模型仅使用指定工具。CLI 局部开关和进程级工具范围的回环探针没有恢复该合同。需要原生会话级工具范围支持后，按同一请求复跑三轮。

仅更新本验收记录。未触及公共 ACP 转发或生命周期边界，未执行 Cargo 定向测试或构建。原始 SSE 和回读保存在本机私有目录。临时配置已撤除，私有网关与 MCP 子进程已退出，回环端口已关闭。

## 共享构建与原生工具范围复验

使用 RueJS 同一提交的干净归档，RustCodeGraph 1.2.10 在每份副本索引 3478 个文件。原提示、租户、模型及会话权限保持一致。工作区保留的其他任务修改未纳入本任务。核对时，AutoHand 驱动和生命周期夹具自前次记录起没有提交修改。npm 官方 `latest` 仍为固定的 0.9.11。AutoHand 仍使用原生 ACP。[官方 ACP 说明](https://docs.autohand.ai/integrations/acp-mode)

先用原构建（SHA-256 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`）及完整 stdio 工具集跑干净源码对照，再分别用该构建和 Snow 共享构建（`e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`）复跑工具范围。

在私有副本的 `.autohand/settings.local.json` 通过 `permissions.availableTools` 限定三个完整的 MCP 工具名。经 stdio 转发器把同一个 RustCodeGraph 的工具列表限制为这三项，描述首部附原工具名供 AutoHand 的相关性筛选使用，调用参数与结果原样传递。转发器拒绝其他工具名和副本外的 `projectPath`。项目权限配置三轮相同，只有挂载轮有宿主 MCP 条目。此验证找到了可用的项目级限制，补充了前次仅验证进程参数的结果。

独立回环在 AutoHand 私有主配置设置 `features.codeMode=false`、`ui.completionReportEnabled=false`，正常流中模型可见工具数依次为 0、3、0。没有 Code Mode 派发或强制回顾指令。挂载轮实际按顺序调用 query、callers、node，续轮收到对应的三个真实输出，三轮均 `end_turn`。项目权限可以限制模型所见工具，但 AutoHand 的项目配置合并不接纳这两个主配置字段。此探针使用回环模型，真实请求继续沿用 ShiJu 受管入口。

| 构建与工具范围 | 轮次 | 新会话 ID | HTTP / curl / 耗时 | 终态和结果 |
| --- | --- | --- | --- | --- |
| 原构建，完整 | 无挂载 | `ses_1effc6fc9001N41TrLPqYaR670` | 200 / 0 / 6.954 秒 | `chat.done`，目标工具为零，执行内置搜索和脚本，标记后追加说明 |
| 原构建，完整 | 挂载 | `ses_1effc8b6c001bWLjwq0uX3KXfB` | 200 / 0 / 203.121 秒 | `chat.done`，三种目标工具成功，仍有额外调用和两次 `node` 失败 |
| 原构建，完整 | 撤除 | `ses_1efffa647001GCAjpfNcRprLnv` | 200 / 0 / 10.996 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，目标工具为零 |
| 原构建，限定 | 无挂载 | `ses_1effde695001BMh4Q3MCxSlUnv` | 200 / 0 / 4.920 秒 | `chat.done`，零工具事件，答复含未执行的 DSML 标记 |
| 原构建，限定 | 挂载 | `ses_1effdfb23001QaFvBjqVjDkjhk` | 200 / 0 / 205.012 秒 | `chat.done`，只调用三种指定工具，146 次成功，1 次失败 |
| 原构建，限定 | 撤除 | `ses_1f0011d8d001cgntFAqRFAj9c4` | 200 / 0 / 5.992 秒 | `chat.done`，零工具事件，答复仍含未执行的 DSML 标记 |
| 共享构建，限定 | 无挂载 | `ses_1efff45e2001IIWFriTBBW9xyw` | 200 / 0 / 6.490 秒 | `chat.done`，零工具事件，答复是未被执行的 DSML 脚本调用标记 |
| 共享构建，限定 | 挂载 | `ses_1efff61d9001FJEs6jhmBTMM8O` | 200 / 0 / 64.671 秒 | `chat.done`，只调用三种指定工具，40 次成功，2 次失败 |
| 共享构建，限定 | 撤除 | `ses_1f0005f9c001J4okg3gnZ05M4R` | 200 / 0 / 6.033 秒 | `chat.done`，答复为 `MCP_NOT_AVAILABLE`，零工具事件 |

共享构建三轮均无 `chat.error`。UI summary 回读确认指定预设与 `full_access=true`。ACP options 确认指定模型和 `session_model`，执行中 `connected=true`、`access_status.support=client_only`，终态 `connected=false`。工具限制只收窄模型可见能力，没有修改审批模式。

共享构建挂载轮的 `call_d1adea153da54f9aaa23c36e` 查到 `createRueSiteApp`（`app/app.tsx:32`），`call_4fb07daf2ba24cc1b4fb52fe` 查到其调用者 `mountRueSiteApp`（同文件第 39 行），`call_bb77a4803bc94cfb892d5c72` 读回 `useApp` 第 4 行委托 `_$createApp`。首次失败请求读了不存在的 `app/router.tsx`，随后又读了不存在的 `app/router.ts`。在同一索引中，`query createAppRouter` 返回空，按文件行读取 `app/router/index.ts:787` 则可见工厂定义。答复覆盖主链、SSR 和缺少容器的分支、插件接入、布局绕过及两项快照验证设想。它明确未完整回读路由插件安装实现。预期节点数量必然翻倍的说法超出源码证据，实际数量取决于快照和新根块结构。

项目级限制消除了共享构建挂载轮的额外调用，但两种受管构建的限定无挂载轮都返回 DSML 标记。可控回环证明关闭主配置中的 Code Mode 和强制回顾后，三种工具仍可调用并续轮。真实验收还需要受管入口支持这两项设置，以及固定模型使用现有索引准确定位文件，或由 RustCodeGraph 修复相关符号索引。三轮合同当前仍有实际失败，故保留为已阻塞。没有修改产品代码或运行 Cargo。
三座私有网关、回环端口及工具进程均已关闭。临时工作区、权限配置和租户凭据副本已清理，私有目录保留脱敏检查和原始事件证据。

## RustCodeGraph 1.2.11 和原生直出工具

使用同一 RueJS 干净提交，复用已审验的 RustCodeGraph 1.2.11（SHA-256 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`），重建 3478 文件索引，确认 `createAppRouter` 命中 `app/router/index.ts:787`。延续项目级工具范围和宿主 `allowedTools` 三项声明。AutoHand 的原生项目权限负责隐藏内置工具，stdio 转发器只公开指定三个图谱工具并拒绝越界请求。复用 Goose 已验证的分页、真实入口文件和索引根说明，保留原参数和真实返回。固定请求、租户、模型、服务名及 `full_access=true` 不变。[发行构建对照](pi.md) · [元数据对照](goose.md)

新图谱的基线使用已通过 Qoder 验收的网关构建（SHA-256 `2e84923a46704187dfd9d56dfc4b45d6ebf9cfc93398a6bb3a82b1cc61a1f935`）及 AutoHand 0.9.11。

| 轮次 | 新会话 ID | HTTP / curl / 耗时 | 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0fdb7410017ZXsbcP0UBLGkm` | 200 / 0 / 5.796 秒 | `chat.done`，零工具事件，返回未执行的 DSML 脚本调用标记 |
| 挂载 | `ses_1f0fdcebd001HabJdNOGyXCkBK` | 200 / 28 / 300.004 秒，续传 200 / 0 / 114.089 秒 | 同一会话收到 `chat.done`，74 次 query、1 次 callers、68 次 node，全部成功 |
| 撤除 | `ses_1f104226c001JpolaINkBu0n1W` | 200 / 0 / 5.247 秒 | `chat.done`，零工具事件，返回说明及未执行的 DSML 标记 |

挂载轮的 `call_217910ef128c419781ca2ff9` 查询定位 `createRueSiteApp`，`call_623f95de73b04d239f7a64b1` 返回 `mountRueSiteApp`，`call_98049e837e8242128a8dffe3` 读回 `useApp`。分页调用 `call_29d832b1fbb248c5b9c6557b` 读取 `app/router/index.ts:760–791`，包含路由工厂。答复正确走通调用链和各分支，并把快照影响列为静态推断。两项 DOM 场景未经实际执行，布局和标签的精确重复数量取决于夹具。新版图谱和元数据消除了该组错误路径，两个对照的回复仍不符合合同。

用同一 AutoHand 二进制进行独立回环，原生 `--set features.codeMode=false --set ui.completionReportEnabled=false` 移除了不可用的脚本派发指令和强制回顾提示。三轮声明数依次为 0、3、0，挂载的三个调用收到真实续轮输出，均由原生 ACP 返回 `end_turn`。原网关显式配置这组参数时，无挂载实测会话 `ses_1f1046af8001xbb481DtrquZFQ` 在发起推理前返回 `chat.error`，原因为 `Autohand custom launch arguments are not verified`。驱动仅允许这组精确的可选参数追加到两个已验证的原生命令，维持原启动方式、模型绑定、授权和撤销边界，并继续拒绝其他配置覆盖。

定向命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib autohand_checks_direct_tool_launch_overrides`。槽位 3 的 target 和 build 目录均为仓库的 `target/rust-slot-3`。修复前退出 101，实际 1 项因启动参数门禁失败。修复后实际 1 项通过，退出 0，覆盖两个原生命令接受精确参数组，以及拒绝额外模型端点覆盖。同槽位 `cargo build -j 1 --locked -p vw-cli --bin shiju` 退出 0，耗时 7 分 30 秒，网关 SHA-256 为 `6897d9bc2574f4789f46771eaab82b88335dabb9d6d05ba85ef766fdd9c75861`。公共 ACP 会话转发未改动，未重跑生命周期夹具。

新构建第一组三轮均 HTTP 200、curl 0、一个 `chat.done` 且零 `chat.error` 或模型恢复。

| 轮次 | 新会话 ID | 耗时 | 工具及答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f10cbaff001owRNkQiVn5xMcz` | 4.651 秒 | 零工具，精确标记 |
| 挂载 | `ses_1f10cce20001P0iikpNlZWWpvl` | 278.578 秒 | 36 次 query、2 次 callers、50 次 node，均为完成事件 |
| 撤除 | `ses_1f1110ef50019rnk3khTmNjV8U` | 4.611 秒 | 零工具，精确标记 |

`call_90a6a7edafd14c52ac5dfe94` 查询定位工厂，`call_47e07da5d6204d2a9e575cea` 返回挂载调用者，`call_1f2f5c32ca9d4cfd9e32aadb` 读回 `useApp` 与后续运行时工厂。答复覆盖源码链路和两个可检验场景，但误称 callers 未挂载，与实际调用和输出矛盾。另有一次 node 原样返回根 `index.html` 未被索引（`call_0af8d627a79b4649ba673d99`），尽管文件存在且 ACP 记录工具已结束。独立直连相同索引确认同一结果。这组只通过工具限制和无挂载、撤除对照，答复准确性需复验。

最后一组复用同一构建和固定请求，仅补充工具目录元数据。标注明确注册的三种工具、单步相关性筛选与既有调用结果的关系，以及根 HTML 存在但未被图谱索引的事实。分页字段继续转发。工具入参与结果未改写。

| 轮次 | 新会话 ID | HTTP / curl / 耗时 | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f111c89b001bBu3Rf7A71o11r` | 200 / 0 / 5.602 秒 | 一个 `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f111e000001Vr3hw1DPRMo2II` | 200 / 0 / 184.384 秒 | 一个 `chat.done`，42 次 query、3 次 callers、47 次 node，均成功 |
| 撤除 | `ses_1f114b1840010KMKJdt990zgq8` | 200 / 0 / 4.681 秒 | 一个 `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

三轮无 `chat.error`、模型恢复或额外调用。挂载的 92 个调用均有同 ID 的开始和完成事件，无失败或索引路径错误。`call_070ffd1dce0e4c8480c52c28` 定位 `app/app.tsx:32`，`call_cc058ce9b65246dcb1dc4976` 返回 `mountRueSiteApp`（同文件第 39 行），`call_95a6b1a234b548ae98f852ac` 读回 `useApp` 调用 `_$createApp`，`call_7139a7e3d79e4fe7b51ba48a` 返回运行时工厂实际源码。

三轮 summary 确认 `acp-autohand / autohand` 和 `full_access=true`。ACP options 确认原定模型及 `session_model`，运行中为 `client_only`，终态连接全部释放。未改宿主审批设置或原生会话模式。

答复正确引用主链与 callers 实际结果，覆盖浏览器和 SSR 分支、容器缺失、路由和 i18n 的安装顺序、布局绕过及快照残留。两项设想分别使用已注册的设计路由检查静态首屏去重，以及检查预填模板的哨兵节点是否被清除。未运行浏览器或快照场景，具体重复数量取决于模板内容。答复正确定位路由的容器级安装路径，但对全局回退有一处表述偏差。独立核对 `packages/router/src/index.tsx:475–480` 确认仅在无容器时设置 `__activeRouter`。

严格三轮的工具、终态和主链行为通过。三座私有网关、端口和工具进程均已退出，临时权限配置、项目副本及租户凭据已清理，原始 SSE 保留在本机私有目录。
