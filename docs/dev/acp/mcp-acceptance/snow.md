# Snow CLI 动态挂载 RustCodeGraph

2026-11-19，目标后端 `acp-snow / snow`，使用原生 stdio，租户 `10001 / 100010005`，`session_model` 绑定 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，请求 `full_access=true`。提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

RueJS 副本来自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 及当时工作区。RustCodeGraph 1.2.10 索引了 3687 个文件。确认 `createRueSiteApp` 位于 `app/app.tsx:32`，`mountRueSiteApp` 位于 `app/app.tsx:39`，`useApp` 位于 `packages/runtime/src/hooks/useApp.ts:4`，`_$createApp` 位于 `packages/runtime/src/compiler-runtime/app.ts:57`。

## 修复前指定模型三轮

Darwin arm64，ShiJu 0.6.0 的二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。私有网关使用 Snow 0.9.14 受管 v1 入口，脚本摘要 `dea7a6b9b28490a78c7c0f95d294b5c5203c7aa8f4a9f7cedc97720bbdd9d25e`。

| 轮次及新会话 | HTTP / curl / 秒 | SSE 终态 | 工具完成事件 |
| --- | --- | --- | --- |
| 无挂载 `ses_1efc1abef001I4jhk1a0GTKRCa` | 200 / 0 / 9.834 | `chat.done` | `tool_search` ×2，`session-command-list`，`session-command-run` |
| 挂载 `ses_1efc1d2af001vWRDPr2ZAHBA2a` | 200 / 0 / 9.118 | `chat.done` | `ace-search` ×2，`session-command-run` |
| 撤除 `ses_1efc1f7cb001E4Zw9FOQorF4YU` | 200 / 0 / 7.544 | `chat.done` | `tool_search` ×3 |

三轮均无 `chat.error`，UI summary 回读后端及 `full_access=true`，ACP options 在线和终态均确认指定模型及来源。在线权限支持报告 `client_only`。挂载轮和两轮对照均含 `MCP_NOT_AVAILABLE` 及额外说明，三种目标工具均无开始、完成或输出。

## 首个失败边界及修复

原生握手确认 ACP v1、`loadSession=true`、HTTP/SSE 能力均为 false。固定版接受带 stdio MCP 定义的建会话请求并保存在 `session.mcpServers`。提示阶段的工具收集只读取 Snow 自己的工具缓存，从未消费该字段。模型侧工具表因此缺少挂载工具。

`SNOW_CONFIG_DIR/config.json` 中的 `snowcfg` 配置模型中继。独立的 `SNOW_CONFIG_DIR/settings.json` 支持 Snow 自己的 `mcpServers`，随后合并工作区的 `.snow/settings.json` 并进入工具缓存。受管 v2 入口直接使用每个 ACP 会话携带的定义，按提示回合连接、发现、执行和关闭 stdio 服务。调用先请求 ACP 单次审批，拒绝会写入失败事件和模型工具结果。原生工具的既有审批路径继续运行。受管 v2 使用独立安装目录，固定发行版本仍为 0.9.14。

## 原生回环回归

使用 `patches/acp-model-source/snow/probe.py` 对已索引的 RueJS 副本和真实 RustCodeGraph 执行独立探针，只有模型端由回环响应替代。旧入口在挂载轮缺少目标工具，退出 1。新入口退出 0，覆盖六个场景。

- 无挂载和撤除后的新会话中，模型工具表均无目标工具，目标事件与审批均为零。
- 新建挂载会话与重载挂载会话均记录 `query`、`callers`、`node` 的开始、审批及成功完成。输出依次命中 `createRueSiteApp`、调用者 `mountRueSiteApp`、浏览器挂载源码全文，模型续轮收到三项真实结果。
- 同一会话重载时撤去挂载，后续模型工具表不再出现目标工具。拒绝轮记录三次单次审批拒绝及失败回传，成功事件和查询输出均为零。
- Snow 的 npm 归档完整性与仓库固定值一致。对原件运行补丁后，新入口的 SHA-256 为 `bb652a8210080da71c20e281217d1e331e840dc2e374c67c64dfbcb75cf26495`，与仓库契约一致，`node --check` 通过。
- 独立执行仓库 `managed_runtime/install.py` 安装最终版 Snow 0.9.14，校验通过。用该安装入口重跑同一探针，六个场景均通过。

## 指定模型原生补充探针

直接启动已安装的受管 v2 入口，通过私有回环 Chat Completions 中继调用同一阿里模型，沿用完整提示。上游凭据仅由中继持有。此探针记录原生 ACP 报文，没有网关 SSE 和 UI summary。

- 无挂载的 ACP 会话 `42a682f6-b4ef-4720-a5b8-39aa98ca6f0c` 以 `end_turn` 结束。模型尝试调用 `terminal-execute`，探针通过 ACP 审批拒绝，目标工具为零。
- 挂载的 ACP 会话 `917383d4-9ba2-483a-8420-357e192c4907` 获批并成功完成 `query` 281 次、`callers` 3 次、`node` 19 次，还调用了 9 次其他 RustCodeGraph 工具。首组输出依次为 `createRueSiteApp`（`app/app.tsx:32`）、`mountRueSiteApp`（`app/app.tsx:39`）和含 `useApp(RootApp).use(router).use(i18n)` 的工厂源码。三次 `node` 调用因为模型给出的文件路径不存在而失败。模型反复查询 `createRouter`、`RouterView` 和 `useRoute`，探针主动结束，未收到挂载轮终态或足以校核的最终答复，撤除轮未执行。探针和图谱子进程均已退出。

## 受管 v2 网关三轮

从当前共享工作区用 `python3 scripts/cargo_slot.py bash <验收脚本> <私有目录>` 领取槽位 3，在同一槽位运行 `cargo build -j 1 --locked -p vw-cli --bin shiju` 并使用产物。两个 Cargo 缓存路径均为仓库下的 `target/rust-slot-3`，构建退出 0，耗时 16 分 43 秒。未改动公共宿主转发，故未运行其聚焦单测。网关健康检查通过，私有配置中的 Snow 系统安装请求成功，实际 v2 入口版本及摘要与上文一致。ShiJu 0.6.0 二进制 SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。

| 轮次及新会话 | HTTP / curl / 秒 | SSE 终态 | 工具完成事件 |
| --- | --- | --- | --- |
| 无挂载 `ses_1efea07fd001S3WME69C0SnEzb` | 200 / 0 / 8.029 | `chat.done` | 目标 0，原生 4 |
| 挂载 `ses_1efea28c90017DDXzsWb3cwi2i` | 200 / 0 / 176.390 | `chat.done` | 目标成功 133，目标失败 16，其他 11 |
| 撤除 `ses_1efecda260019ikSgD1xGSBTwN` | 200 / 0 / 4.607 | `chat.done` | 目标 0，原生 1 |

- 三轮各有一个 `chat.done`，无 `chat.error`。全部在线及终态的 UI summary 显示绑定 `acp-snow / snow`、`full_access=true`，ACP options 复读确认指定模型与 `session_model`。在线权限支持为 `client_only`，终态均断开。原生拒绝探针验证了未授权时的失效关闭。
- 挂载轮有 `query` 成功 78 次、`callers` 成功 1 次、`node` 成功 54 次。首组成功调用的 ID 分别为 `call_9a8b9c75667045b8985bb70f`、`call_f94be863eb05402bb34763ea`、`call_24afd1e586c049ae8bd60838`。对应输出为 `createRueSiteApp app/app.tsx:32`、`mountRueSiteApp app/app.tsx:39`，以及包含路由与 i18n 注册的工厂源码。SSE 按相同调用 ID 给出开始与成功完成。
- 挂载轮另有目标查询失败 16 次及其他工具 11 次，其中 10 次为其他图谱工具，1 次为 `askuser-ask_question`。含不存在的 `app/router.tsx`、`packages/router/src/index.ts` 等模型参数。两轮对照均返回带说明的 `MCP_NOT_AVAILABLE` 并调用了原生工具，目标工具均为零。
- 最终答复的入口调用链、浏览器和 SSR 分支、缺少容器、插件安装、`/rust-layers` 布局绕过与保留快照会形成双份 DOM 的推断均有源码对应。答复给出了 DOM 唯一性和文档页签两个场景。DOM 唯一性场景写反了失败条件，应在移除清空逻辑后预期重复。答复也声称 i18n 插件失败后仍可使用 Provider，实际插件安装抛错会在根组件工厂执行前中断挂载。两项场景均为源码推断，未执行浏览器验证。
- 验收状态为已阻塞。动态工具已进入真实模型回合并返回输出，严格合同仍因额外工具、失败事件、对照回复格式和答复偏差而未通过。私有网关、Snow 与图谱进程已退出，回环端口关闭，槽位锁释放，挂载条目及私有认证文件清理完毕。原始证据留在本机私有目录。

## 共享构建与三工具目录复验

使用当前共享槽位 3 的 ShiJu 0.6.0 副本，SHA-256 为 `1a033a19a5c5e2c115e90abbd336f898c0df3ac101a9824ffe8b36f11b0bcc47`。安装端点确认受管 Snow 0.9.14 v2 入口的摘要仍为 `bb652a8210080da71c20e281217d1e331e840dc2e374c67c64dfbcb75cf26495`。沿用原请求、模型、租户与审批设置。RueJS 使用固定提交的干净归档，RustCodeGraph 1.2.10 索引 3478 个文件。

复用 [Goose](goose.md#通过的三轮) 与 [Hermes](hermes.md#共享构建复验) 的私有 stdio 转发方式，仅公开三种目标工具，补充 `node.offset`、`node.limit`、实际入口文件名和 `projectPath` 可省略的元数据。协议探针核对原工具目录 8 项、限定目录 3 项。工厂查询、调用者查询、从 `app/router/index.ts:787` 读取五行的返回与原 stdio 服务完全一致。

| 轮次及新会话 | HTTP / curl / 秒 | SSE 终态 | 调用结果 |
| --- | --- | --- | --- |
| 无挂载 `ses_1f0526244001gNHoEGJ9PxlGos` | 200 / 0 / 5.860 | `chat.done` | 零图谱工具，原生列表及运行各一次 |
| 挂载 `ses_1f0527a6c0010VTK2Z11B6q17x` | 200 / 0 / 93.366 | `chat.done` | 39 次 query、1 次 callers、39 次 node，全成功 |
| 撤除 `ses_1f053e753001fRby3E0LrIfYmt` | 200 / 0 / 4.427 | `chat.done` | 零图谱工具，原生工具搜索一次 |
| 无挂载重试 `ses_1f0545c170013RM3wx7fJI08Jv` | 200 / 0 / 4.635 | `chat.done` | 零图谱工具，原生会话命令一次 |
| 无挂载重试 `ses_1f0546eb1001Uy9ue6kclXieDi` | 200 / 0 / 4.285 | `chat.done` | 零图谱工具，原生工具搜索一次 |
| 撤除重试 `ses_1f0547ff80019nK0pdJxqvMPLW` | 200 / 0 / 4.059 | `chat.done` | 零图谱工具，原生文档列表一次 |
| 撤除重试 `ses_1f0549134001NjBvdpARkgqm91` | 200 / 0 / 6.146 | `chat.done` | 零图谱工具，原生工具搜索三次 |

- 七轮各有一条 `chat.done`，无 `chat.error`。终态均确认 `acp-snow / snow`、`full_access=true` 和指定模型 `session_model`。首组三轮在线权限为 `client_only`。复用既有原生单次审批和拒绝证据。
- 挂载轮只调用三种指定工具。79 个调用的开始、完成和非空结果按 ID 配对。`call_1214ee227c6c4b61a3c1f8b5` 命中 `createRueSiteApp`（`app/app.tsx:32`），`call_8485c4894b2942acb3e4e233` 返回 `mountRueSiteApp`（第 39 行），`call_e6770e43df654acca5cd021a` 与 `call_f5320fc0ba674d7088237fd1` 分别读回 `useApp.ts:4` 与运行时工厂 `app.ts:57`。
- 答复还原入口、浏览器与 SSR 分支、缺少容器的报错、插件安装及布局绕过条件。快照重复与残留由源码推断。两项验证分别覆盖静态产物和预填容器的客户端挂载，具体 DOM 计数未经运行验证，第一项的示意选择器需在执行测试时替换为实际节点。
- 两类对照各有三次独立新会话，均无图谱调用。指定模型均在 `MCP_NOT_AVAILABLE` 之外附带说明，并调用 Snow 原生工具。原生内置提示中已有工具搜索引导，该行为在无图谱声明时即可复现。会话级 MCP 实现、聚焦回归、挂载结果和撤除隔离均已验证，严格对照待回归。本轮仅记录证据，无运行时代码变化或 Cargo 重跑。两座私有网关已退出，临时挂载及私有认证文件已清理。

## 最终集成回归

在共享工作区 `5ad05b0f0` 复用任务 8 的槽位 6 构建。两项任务的 CLI 副本摘要相同，均为 `7f8fb6ab97e9e6b00e6694ce48376e2d50a3ce4b2bff3fff93004b98db824916`。从公共测试夹具提交 `2bd74758a` 到此版本，宿主默认值、模型运行时及 Snow 补丁无变化。复用任务 8 在同槽位执行的 `cargo test -j 1 -p session-engine --lib acp_mcp_mount_carries_resolved_tool_policy` 结果，退出 0，实际 1 项通过。该测试检查 Pi 与 Cortex 的策略投影。Snow 转发与拒绝继续采用本记录的 79 次配对事件和六个原生回归场景。未重复构建、公共测试或已经通过的挂载轮。

| 独立对照 | HTTP / curl / 秒 | SSE | 原生工具完成事件 |
| --- | --- | --- | --- |
| 无挂载 `ses_1f07399e9001XSc4DHVu5rrD4h` | 200 / 0 / 10.558 | `chat.done` | 工具搜索 ×2、会话命令列表、会话命令运行 |
| 无挂载 `ses_1f073c4af001hMuIvIQwHSgBT0` | 200 / 0 / 5.635 | `chat.done` | 工具搜索 ×2 |
| 撤除 `ses_1f073db90001NCVO8e2BhnXUW3` | 200 / 0 / 4.287 | `chat.done` | 工具搜索 |
| 撤除 `ses_1f073ecbf0015U1yu0ebujaw68` | 200 / 0 / 4.111 | `chat.done` | 文档列表 |

四轮均无图谱调用、`chat.error` 或模型恢复事件，均有附加说明。UI summary 与 ACP options 核对了原绑定、`full_access=true`、原模型及来源，在线权限为 `client_only`，终态均释放连接。

独立原生诊断使用 Snow v2 的私有副本，通过该版本已有的 `toolSearchDisabled` 选项使内置提示描述实际预加载的工具表。保留全部工具和审批，原样用户提示通过私有 Chat Completions 中继送往同一阿里模型。抓取的模型请求确认已切换到预加载说明。三个无挂载原生会话均以 `end_turn` 结束，仍分别尝试 2、1、2 次原生会话命令，诊断客户端在 ACP 审批处拒绝命令，三个答复均未达到精确标记。该诊断定位了原生模型行为，未进入发布入口或生产代码。

最终状态为已阻塞。需要 Snow 与指定模型在原生工具仍可用时稳定满足精确标记与零调用约束，或由任务负责人明确调整对照验收合同。私有网关已关闭，挂载条目和私有认证文件已清理。

## Snow v3 会话工具目录补丁

Snow 0.9.14 的原生 Chat Completions 路径会把工具定义一并提交给模型，默认系统提示却描述按需加载，并要求先搜索工具。仅切换预加载说明仍会触发原生会话命令。v3 在原生系统提示前加入本回合真实挂载清单，明确用户限制工具时应直接查阅本回合的工具定义，服务缺失时遵循用户指定的回复格式。原生系统提示、安全要求、工具目录、审批、用户消息和模型继续沿用。改动仅接入 Snow ACP 的 Chat Completions 分支。v3 安装在独立目录，最终入口摘要为 `2a38b3678070cf5d47892ad0c8c43429090b73e5a4000cb3d92edaa5478c4a77`。最后的清单只包含发现了已校验工具的服务。

- 同一原生回环探针在 v2 因缺少真实清单断言退出 1，在 v3 覆盖六个场景并退出 0。确认无挂载、挂载、重载与撤除后的模型清单，原用户消息保持不变，原生工具仍在目录，原单次审批和拒绝断言通过。供应商原包重打补丁所得内容与 v3 入口逐字节一致，`node --check` 通过。
- 私有原型诊断用同一指定模型、原用户消息和真实三工具转发。三个单独对照都只回复 `MCP_NOT_AVAILABLE` 且零工具。另一次原生三轮中，无挂载与撤除也各自零工具且精确回复，挂载会话 `d0e0fbf6-f653-44a2-81de-745318c17925` 以 `end_turn` 结束，37 次 query、1 次 callers、39 次 node 全成功。
- 初版 v3 入口的原生三轮也以 `end_turn` 结束。无挂载 `5743cc43-6240-4e13-a5f2-eee8feaac084` 和撤除 `512fa4dc-68de-4a40-8074-28efe3c671ee` 均只回复 `MCP_NOT_AVAILABLE` 且零工具。挂载 `87318766-2c0c-4544-aca9-ce5dc8695900` 仅调用三种目标工具，132 次成功，另有一次 query 因模型把未索引的 `packages/i18n` 用作 `projectPath` 而失败。原生诊断与受管网关 SSE 验收分别记录。

- 追加空服务名称隔离断言，初版 v3 在新增场景退出 1，加固入口七个场景全部通过。加固入口使用只公布当前索引根的三工具转发，原样转发工具调用及结果。指定模型的两个原生对照 `77b37d15-2fdc-42da-8d1b-5de8e74ad2fc` 和 `00f8d93c-0544-4394-a953-19f12a8f9bc2` 均为精确标记、零工具。挂载 `e5ebdb46-4794-4de0-8b0b-ec47b65da65e` 的 query 58 次、callers 2 次、node 56 次均成功，零额外工具，三轮均收到 `end_turn`。

### 初版 v3 网关复验

复用包含初版 v3 的槽位 3 构建，CLI 摘要为 `25988bf02e238fe5450ca7c941015eaed1c067f5c64123c85aeacf1e2454734a`。私有安装校验到初版 v3 入口 `d744c069564ecb1ce37eafc8a0cfe21429df0706839906bab459a4bca5f4737c`。共享网关在 `2b35013ae` 移除了请求体中的 `acp_agent` 字段，旧请求三次均 HTTP 422。按新接口仅省略该字段，仍由 `agent=acp-snow` 选择原后端，用户消息、模型、来源及访问级别保持原值。

| 轮次 | 新会话 | HTTP / curl / 秒 | 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f1759d510012B0qJgWNNTweRy` | 200 / 0 / 3.041 | `chat.done`，仅精确标记，零工具 |
| 挂载 | `ses_1f175aa2f0019fRT2iMxt2licR` | 200 / 0 / 165.187 | `chat.done`，query 60、callers 2、node 48，全部成功 |
| 撤除 | `ses_1f1782fcc001zCDcmf1E1RkSkf` | 200 / 0 / 3.027 | `chat.done`，仅精确标记，零工具 |

三轮均无 `chat.error` 或恢复事件。UI summary 和 ACP options 确认 `acp-snow / snow`、`full_access=true`、原模型及 `session_model`。在线权限为 `client_only`，结束时连接释放。挂载仅调用指定的三个工具，配对的调用 `call_7eec03a4a0c8449ba2aaed43`、`call_f73c970ac13b47a98c643798`、`call_3ce6055069204409a1003682` 分别返回工厂位置、挂载调用者及应用源码。

答复还原了入口、SSR、容器缺失、插件安装、布局绕过及重复 DOM 的推断，给出浏览器首屏唯一性和构建期快照非空两个检查场景。其关于尾斜杠和查询字符串的附带断言忽略路由归一化。加固后另用仓库安装脚本全新安装 Snow v3，复用 npm 下载缓存，退出 0，最终摘要与已通过原生三轮的加固入口逐字节一致。

### 加固版 v3 网关验收

复用槽位 6 的共享构建，CLI 摘要为 `109066ea0b79ce6edfc4399624715f0dafa80d397adabe7a1ae46158407e7cba`。私有网关通过安装端点安装 Snow 0.9.14 v3，入口摘要为 `2a38b3678070cf5d47892ad0c8c43429090b73e5a4000cb3d92edaa5478c4a77`。使用新接口的预设绑定、原提示、原模型与原授权。私有转发服务绑定单一 RueJS 索引根，公开三种真实图谱工具及分页参数，原样转发请求参数和输出。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f18ded08001IQj9I4odMdsfBL` | 200 / 0 / 3.465 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f18dfb460019Jg6kle3syHsiB` | 200 / 0 / 127.924 | `chat.done`，query 58、callers 1、node 49，全部成功 |
| 撤除 | `ses_1f18fef78001Yebm2Jyt1F0ro9` | 200 / 0 / 2.616 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

- 三轮无 `chat.error` 或模型恢复事件。挂载轮 108 个调用逐一配对开始与成功完成，结果均非空。`call_40fc28660f97474b849fbad2` 定位 `createRueSiteApp`（`app/app.tsx:32`），`call_0ab2b9a942454bcf948a1faf` 返回 `mountRueSiteApp`（第 39 行），`call_364bd55fcca24ab29e15c301` 回传应用入口源码。
- 三轮终态确认 `acp-snow / snow`、`full_access=true`、指定的阿里模型和 `session_model`，连接均释放。无挂载及挂载轮采到在线 `client_only`，撤除轮采样时已断开。七个聚焦场景另外验证了原生工具目录保留、单次审批及拒绝。
- 答复给出调用链、SSR、容器缺失、路由及 i18n 安装、布局条件，区分源码与移除清空后的推断。两项验证涵盖 `/design/button` 等实际路由的首屏重复节点，以及构建快照与首帧的差异。DOM 计数与交互推断未在浏览器独立运行。
- 按用户明确的新标准，官方工具可按原权限共存，挂载工具须实际执行并回传。此项已通过。复用共享构建与原公共测试，无重复 Cargo 验证。私有网关端口已关闭，挂载条目和私有认证文件已清理。
