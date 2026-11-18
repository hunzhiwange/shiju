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
