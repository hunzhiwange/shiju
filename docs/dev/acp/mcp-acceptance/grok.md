# Grok Build 动态挂载 RustCodeGraph

2026-11-19 完成指定模型三轮实测。Grok Build 原生 ACP 可以动态发现、调用 RustCodeGraph 并读回结果。任务 20 保留为已阻塞。首个合同偏差出现在无挂载轮，模型调用了一次 Grok 内置的 `search_tool`。挂载轮出现额外工具与失败调用，撤除轮也没有只回复约定标记。

## 环境与请求

- Darwin arm64，Grok Build `1.0.77 (50ca579fa996)`，本机固定版入口 SHA-256 为 `197c66a63e5ac46e07450b0ca21d141ce665050a412180f185dadfae1c7a5b2a`。私有回环网关使用既有 ShiJu 0.6.0 开发构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。仅在私有租户副本启用内置 Grok 预设并指定固定版入口。
- 使用 [统一验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。三轮均使用租户 `10001 / 100010005`、`acp-grok / grok`、`session_model`、`full_access=true` 及 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出到私有目录，RustCodeGraph 1.2.10 索引 3478 个文件，命中 `createRueSiteApp`。使用原生 stdio 与 `--no-watch`，仅在挂载轮通过副本的 `.vwacprc.json` 提供 `ruejs_graph_probe`。挂载过程中观察到指向该副本的 RustCodeGraph 子进程。

## 三轮结果

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 | 可见答复和工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efa92912001rD6GhnuAGSjgFh` | 200 / 0 / 5.501 | `chat.done` | `MCP_NOT_AVAILABLE`。内置搜索一次，返回空目录，目标工具调用为零 |
| 挂载 | `ses_1efa93fda001qZIYiHKWyXimYN` | 200 / 0 / 143.100 | `chat.done` | 给出调用链、分支和快照分析。三种目标工具均成功，另有其他工具及两次失败调用 |
| 撤除 | `ses_1efab706e00107yqNGJumOw5WN` | 200 / 0 / 6.428 | `chat.done` | 包含 `MCP_NOT_AVAILABLE`、进度和解释。内置搜索一次，返回空目录，目标工具调用为零 |

三轮均有一个 `chat.done`、零个 `chat.error`，无需续传。运行中和终态的 UI summary 均确认 `acp-grok / grok` 及 `full_access=true`。ACP options 均保留指定模型、提供商和来源，连接从运行中的 `true` 变为终态的 `false`。运行中访问级别为 `client_only`。

| 目标工具 | 代表性调用 ID | 实际完成输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_6a51ffe6c4c14e5ab1c95716` | `createRueSiteApp` 在 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_577d874fcce14bfa82cd4660` | 调用者 `mountRueSiteApp` 在 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_3f315e83bdfe4bd7a7dd0d3d` | `packages/runtime/src/hooks/useApp.ts:4` 转调 `_$createApp` |
| `rustcodegraph_node` | `call_3dd244c625ee4673ae37bfef` | 运行时工厂在 `packages/runtime/src/compiler-runtime/app.ts:57`，挂载时依次安装已注册插件 |

同一调用 ID 可关联开始和完成事件。完成事件的 `UseTool` 输入包含限定名，输出中的 `server_name` 和 `tool_name` 标识实际执行者。挂载轮成功完成 query 34 次、callers 7 次、node 16 次，另完成内置搜索 3 次以及同服务的 files 6 次、callees 4 次、explore 3 次和 status 1 次。一次 files 调用将参数放在 `tool_input` 之外，Grok 拒绝解析。另一次调用了不存在的 `ruejs_graph_probe__rustcodegraph_use_tool`。未观察到 shell、写入或其他 MCP 服务调用。

答复正确追踪 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`，说明 SSR 跳过浏览器挂载、缺少 `#app` 时抛错、路由和 i18n 在根工厂渲染前安装，以及根组件按 `/rust-layers` 路径决定是否绕过布局。答复指出调用图未收录模块顶层调用。移除清空后新树追加在快照之后是源码推断，答复提出首页冷启动及从首页导航至无布局路由两个场景，均未实际执行。首页可用既有 `.site-header` 检查重复，但快照专属标记需要测试夹具提供。第二个场景还需注册 `/rust-layers` 或以夹具提供该路由，当前路由表未注册此路径。

## 原生挂载与审批边界

- 宿主建会话入口把选中服务传入 ACP `session/new.params.mcpServers`。独立原生探针以含真实 stdio 定义的建会话请求复核，ACP v1 握手成功，声明 HTTP、SSE 及会话加载支持。新会话选择 `shiju-session`，回环 Chat Completions 请求中出现 `ruejs_graph_probe` 已连接及 8 个工具的提示，模型工具列表暴露 `search_tool` 与 `use_tool`。原生工具发现依赖这两个入口，未找到固定版 ACP 受管入口可用的直接展开选项。传输切换无法解决本次观察到的额外调用和答复偏差。 [Grok Build MCP 文档](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/07-mcp-servers.md)
- Grok 模型驱动为各会话生成私有 `GROK_HOME` 和绑定模型的 TOML。网关也使用隔离 HOME。两次控制轮的原生搜索均返回空工具目录，未继承同名常驻服务。挂载轮的工具由原生 Grok 通过 stdio 调用，指定模型根据工具结果继续推理。
- 独立回环模型夹具各发出一次真实 query 请求，Grok 均发送 `session/request_permission`。选 `reject_once` 时以 `cancelled` 结束，原因为 `PermissionRejected`，未向夹具回传工具结果。选 `allow_once` 时，续轮收到真实结果 `createRueSiteApp` 位于 `app/app.tsx:32`，最终 `end_turn`。三轮正式请求未人工回复审批。权限探针保留原生审批流程，并使用独立配置和占位中继令牌。

本次保留现有原生适配，未改驱动或权限策略。失败集中在当前 Grok 工具发现路径及指定模型执行约束，既有生命周期用例不覆盖该偏差，本轮未执行 Cargo。重新验收需要受管入口可按会话直接声明目标工具并保留原生审批，再用原提示与模型复跑三轮。私有网关退出 0、端口关闭，私有租户配置及临时 MCP 文件已清理。RueJS 副本的 4108 个归档普通文件与原提交一致，索引外无新增文件。原始证据留在本机 0700 私有目录。

## 共享构建与上游新版复验

复用共享槽位 3 已有的 ShiJu 0.6.0，二进制副本 SHA-256 为 `f46699dd46d8520d8a7cfc3324132ccd5a2e05857e89fde160052fdcf75add3c`。两组私有实例实际均启动固定版 1.0.77。限定目录组的旧脚本把命令放在 `acp.agents.grok`，有效配置未采用该覆盖，实际启动日志显示固定版入口。本处据日志更正此前的 1.0.89 标注。复用相同的 RueJS 提交、提示、租户、模型和权限。限定目录组使用本批次 Cursor 验收已使用的只读 stdio 转发器，只公布 query、callers、node，补足分页及 TypeScript 路径说明。它限制三个工具及副本内路径，原样转发调用参数和 RustCodeGraph 的响应。

从官方 npm 元数据读取稳定版 1.0.89 与预发行版 1.0.90，校验 Darwin ARM64 归档的 SHA-512 完整性，解压原生入口到私有目录。分别自报 `1.0.89 (28be5bb1bf65)` 与 `1.0.90 (4f3ee008287d)`，入口 SHA-256 为 `f149e6ad81339491f8d6e0c5fd712f2813042c0dfbb595c28d5767e4567c17a1` 和 `f2e9b293e2f67d46bc8f1cd4d74ac0d0bbb1609e864f8e4c569523125f57b38b`。新版入口仅用于下述独立原生探针。 [官方 npm 发行](https://www.npmjs.com/package/@xai-official/grok?activeTab=versions)

| 版本与目录 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态和结果 |
| --- | --- | --- | --- | --- |
| 1.0.77 完整 | 无挂载 | `ses_1f02da9ed001pg508bLd0THaGF` | 200 / 0 / 9.643 | `chat.done`，内置搜索 3 次，答复含标记及解释 |
| 1.0.77 完整 | 挂载 | `ses_1f02dd068001dx5jZsdNQok2Dr` | 200 / 0 / 219.481 | `chat.done`，三个目标工具均成功，另有额外调用及 3 次失败 |
| 1.0.77 完整 | 撤除 | `ses_1f0312a93001sHNsKN0jDAGqb3` | 200 / 0 / 8.198 | `chat.done`，标记准确，内置搜索 1 次 |
| 1.0.77 三工具 | 无挂载 | `ses_1f02f822f001vRUHi5Jy15NnWy` | 200 / 0 / 6.497 | `chat.done`，标记准确，内置搜索 1 次 |
| 1.0.77 三工具 | 挂载 | `ses_1f02f9bc3001NlboTOVSNuLNCU` | 200 / 0 / 231.738 | `chat.done`，三个目标工具均成功，内置搜索 1 次及 5 次工具失败 |
| 1.0.77 三工具 | 撤除 | `ses_1f0332549001mzhlwPN40Z7Hdv` | 200 / 0 / 6.474 | `chat.done`，内置搜索 2 次，答复含标记及解释 |

六轮均无 `chat.error`，无需续传。控制轮的搜索结果均为空，目标工具调用均为零。六轮 UI summary 与 ACP options 回读均确认原预设、`full_access=true`、指定模型和 `session_model`。运行中连接为 `true`，访问级别为 `client_only`，结束时连接释放。

- 1.0.77 挂载成功完成 query 56 次、callers 12 次、node 36 次。代表性完成事件包含 `call_b493a95f4ac549389ad21e27`（`app/app.tsx:32`）、`call_c7f31d9441bb488798b59180`（`app/app.tsx:39`）、`call_bb65c54cb8854e549a2ddb64`（`useApp.ts:4` 转调工厂）。首次工具失败读取了不存在的 `app/router.ts`，两次后续失败是模型把 status 和 files 的参数放在 `tool_input` 外。答复给出主链及快照重复推断，导航验证仍假定了当前未注册的 `/rust-layers`。
- 1.0.77 限定目录挂载成功完成 query 47 次、callers 8 次、node 31 次。对应事件 `call_204ad622ce7143589d2b93e1`、`call_5564914dc54b4d96b93cba14`、`call_c546bc1087504b0585f37350` 指向相同源码。模型未调用目录外的 MCP 工具，但尝试读取不存在的 `app/router.tsx`、`app/i18n.ts`、`app/index.html`，另有一次错误的 `input` 字段和一次嵌套的 `tool_input`。答复的主链、SSR、缺失容器、插件安装、布局守卫及追加导致的快照重复均与源码相符。它提出预填 `#app` 和静态首页导航两个场景，夹具及浏览器均未运行。
- 1.0.89 和 1.0.90 的独立原生探针均通过 ACP 挂载真实 stdio。两版在 `--agent-profile` 配置三个限定名后，模型仍只通过原生搜索和分派访问 MCP。1.0.89 的 `session/new._meta.agentProfile` 对象探针也未直接公布这三项定义。两版分别选 `reject_once` 均得到 `PermissionRejected` 与 `cancelled`，模型夹具未收到查询结果。分别选 `allow_once` 均在续轮读到 `app/app.tsx:32`，以 `end_turn` 结束。

严格验收继续阻塞。当前 Grok 发行的原生工具目录和分派方式保留了审批及会话边界。更换共享宿主构建并缩小目录后，合同内的失败仍可复现。新版原生探针也仍通过内置工具分派。复跑条件是上游提供保留原生审批的会话级直接工具声明，并让指定模型按原请求完成零失败的调用及精确控制答复。若接受原生发现调用或已恢复的工具错误，需由验收负责人明确调整合同。没有修改 Grok 驱动、注册项或权限，也未运行 Cargo。两座私有网关退出 0、端口关闭，私有租户配置及挂载文件已清理。两份 RueJS 副本的 4109 个归档文件及符号链接均无差异，索引外无新增。原始记录保存在本机 0700 私有目录。

## 私有模型中继补丁

补丁原型分别在两套私有网关运行。两套均从有效配置和启动日志确认实际命中临时启动器，并由启动器调用 Grok 1.0.77 原生入口。使用相同网关构建、RueJS 提交和正式请求。宿主原有中继仍绑定指定阿里模型，原型只代理私有回环的 Chat Completions。控制轮向模型声明零个工具。挂载轮向模型声明真实只读服务公布的三个工具，将模型返回的调用在进入 Grok 前转换为原生 `use_tool`，保留调用 ID，续轮投影回直接工具名。Grok 继续处理权限、执行和 ACP 事件。模型文本原样返回。第一组沿用三工具说明，第二组的 `node` 说明还列出副本中实际存在的应用入口文件，避免模型把 TypeScript 目录导入误当成文件。

原型仅接受本次只读 RustCodeGraph 配置和独立新会话。为读取该服务的 `tools/list`，它会额外启动一次受限服务。正式接入需要复用宿主已连接的目录，并验证恢复、撤销及工具策略。四项投影测试覆盖空目录、碎片化 SSE 参数拼装、拒绝未选中工具和保留辅助请求，均通过。原生独立探针中，批准后模型续轮收到 `app/app.tsx:32` 的实际查询结果，拒绝后以 `PermissionRejected` 结束且未收到结果，两项均通过。未运行 Cargo。

| 补丁 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- | --- |
| 直接声明 | 无挂载 | `ses_1f15aa40e001m77cxFiRdBtrFS` | 200 / 0 / 5.814 | `chat.done`，精确标记，零工具 |
| 直接声明 | 挂载 | `ses_1f15abc1d001flflpKW1trNThA` | 200 / 0 / 240.161 | `chat.done`，只用三种目标工具，七次读取不存在路径 |
| 直接声明 | 撤除 | `ses_1f15e6797001otV0nbaXA0rsi4` | 200 / 0 / 5.447 | `chat.done`，精确标记，零工具 |
| 直接声明与真实路径 | 无挂载 | `ses_1f15bc5c300121PXT9oI6TvNWw` | 200 / 0 / 6.156 | `chat.done`，精确标记，零工具 |
| 直接声明与真实路径 | 挂载 | `ses_1f15bde26001O8bER1BkCkFsj4` | 200 / 0 / 241.968 | `chat.done`，89 次工具成功，零失败及额外工具 |
| 直接声明与真实路径 | 撤除 | `ses_1f15f90df001jWWpt9Bfe33QVi` | 200 / 0 / 4.517 | `chat.done`，精确标记，零工具 |

两组均读回指定预设、`full_access=true`、原模型来源与模型 ID。运行中权限级别为 `client_only`，终态释放连接。六轮均无 `chat.error`，未续传。网关另有既有辅助标题模型的 HTTP 403，调查主模型及工具轮次均正常返回。

带真实路径说明的挂载轮完成 query 35 次、callers 3 次、node 51 次，每个完成事件均与同 ID 开始事件对应，输出均来自 `ruejs_graph_probe`。代表证据为 `call_514fae4ee32a4717bad600eb`（`app/app.tsx:32`）、`call_a5085ff449764e1daeba56e1`（`app/app.tsx:39`）、`call_ea188751fe9546b7b6b9fb33`（`useApp.ts:4` 转调工厂）及 `call_e66f36aa2d614db785c640d8`（`app.ts:57`）。答复说明了调用链、两种启动分支、缺少容器的抛错、插件安装和布局条件，并确认当前路由表未注册 `/rust-layers`。对快照重复的影响明确标为推断，给出带客户端入口的静态首页测试和预置陈旧节点的挂载测试，均未执行浏览器测试。

补丁原型通过主验收流，表明 Grok 原生审批与直接工具声明可以共存。正式入口仍需完成会话选中目录、验证安装、重连和撤销的接线，因此保留任务卡。原型、四项聚焦测试和原始证据保存在本机 0700 私有目录。私有网关均退出 0，端口关闭，配置副本已清理。三份诊断及原型 RueJS 副本均与 4109 项归档文件和符号链接一致，索引之外无新增。
