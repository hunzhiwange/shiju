# Codex CLI 动态挂载 MCP 验收

2026-11-19。三类目标工具均能通过动态挂载实际调用并返回结果。严格验收仍阻塞，详见下文。2026-11-18 的 2.5.0 单工具实测仍作为基线。[原始验收和复用请求](../ACP验收.md#可复用的验收请求)

## 环境和路径

- 使用现有 `shiju 0.6.0` 开发网关的已启用 `acp-codex` 预设。网关二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。租户 `10001 / 100010005`，模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`，来源 `session_model`。同一 JSON 的提示内容与 `189d22dc1` 对应小节匹配，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。仅替换目录和本地 HTTP 地址。
- 受管 `@agentclientprotocol/codex-acp` 2.5.0 和原生 Codex 0.165.0（Darwin arm64）。适配器入口 SHA-256 为 `730bff93eddd77ad1ea1c3ff941ee5fa4fe2b9cb940449e6ee82712295846136`，与受管契约一致。独立实际 `initialize` 返回 ACP v1，支持 load、resume、HTTP MCP，SSE MCP 和 ACP MCP 标志为 false。实际使用 `session/new` 的 stdio MCP，由项目 `.vwacprc.json` 传入本地 RustCodeGraph 程序。挂载轮原生工具轨迹识别出同一个服务名；未开启原始建会话 RPC 负载日志。
- Codex 向会话回环中继使用 Responses，按本地提供商目录和中继路由代码，阿里端使用 OpenAI 兼容 Chat Completions。未抓取本轮上游请求体。Codex 原生会话轨迹记录 `mcp__ruejs_graph_probe` 下的 `function_call`，并记录同一调用 ID 的 `function_call_output`。多个连续调用及最终答复佐证工具返回后继续推理。
- 独立私有网关先通过 `/v1/health`。其 `/v1/config` 缺少目标模型有效条目，三轮 HTTP 200 的 SSE 均在选模阶段返回 `chat.error`，理由为未找到指定模型。也未确认该隔离实例的受管启动资格。私有网关退出 0，私有配置与租户数据副本随即清理。后续按计划改用现有网关，未变更其全局配置。
- 首次真实复验复制 RueJS `main` 工作树中现存的跟踪文件，索引 3475 文件。工作树基于 `14fe287b13549f5183d42fbc45b28e1498ffae1c`，包含入口移动等未提交变更，跟踪文件副本缺少 `app/router/index.ts` 和 `app/entry-server.tsx`。为排除副本差异，按同一提交重新导出完整源码快照和索引，索引 3478 文件，两个入口均存在。

## 工作树副本结果

| 轮次 | 新会话 | HTTP / curl | SSE 终态 | 工具完成 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef7e548a001uJsSldmsHlW8E7` | 200 / 0 | `chat.done` ×1 | 0 |
| 挂载 | `ses_1ef7ea39e001XxcP5bNtmJQG6t` | 200 / 0 | `chat.done` ×1 | 73 成功、6 失败 |
| 撤除 | `ses_1ef829e930016e4YjAHoooPDu2` | 200 / 0 | `chat.done` ×1 | 0 |

三轮均无 `chat.error`。两轮原生落盘的最后一条模型答复均精确为 `MCP_NOT_AVAILABLE`。严格合同的第一个差异在无挂载轮的 SSE，`chat.delta` 含 Codex 模型元数据回退警告和带标签的推理文本。受管入口的 `createWarningEvent` 在当前兼容路径把警告发成普通 agent text。挂载轮 79 个原生函数调用各有同 ID 的结果，全部来自挂载服务，shell 和其他服务调用为零。工具成功数为 query 33、callers 2、node 28、files 9、explore 1。额外图谱工具超出了严格验收要求，另有 6 个 node 调用失败，首个失败是读取缺失的 `app/router.ts`。答复给出目标调用链、浏览器与 SSR 分支、缺少 `#app`、插件安装、布局条件、快照残留推断与两项预期可验证场景。答复明确标出路由入口与 SSR 入口在该副本中不可读。

## 提交快照结果

| 轮次 | 新会话 | HTTP / curl | SSE 终态 | 工具完成 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef827c060017877cpfWb86oz6` | 200 / 0 | `chat.done` ×1 | 0 |
| 挂载 | `ses_1ef82eec3001Yx2ltLv5gL7RCM` | 200 / 28，续传 200 / 0 | 续传收到 `chat.done` ×1 | 108 成功 |
| 撤除 | `ses_1ef87e1bb0010OCoJ8LpOLEwm4` | 200 / 0 | `chat.done` ×1 | 0 |

挂载轮初次连接 300 秒超时，按原请求标识和 `Last-Event-ID: 219` 续传，同会话从事件 220 继续，23.36 秒后收到终态。三轮均无 `chat.error`。两个对照轮原生答复仍精确为 `MCP_NOT_AVAILABLE`，SSE 仍带警告和推理文本。

挂载轮 108 个 Responses 函数调用与结果 ID 一一对应，均为 `mcp__ruejs_graph_probe`。SSE 每个调用均配对记录开始与成功完成。模型调用 query 31 次、callers 11 次、node 57 次，另调用同一服务的 files 7 次及 explore 2 次。shell 和其他服务调用为零。

| 工具 | 代表性 SSE 调用 ID | 实际返回 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_b90099ce804e49689e87bf9d` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_6e7bb920c4934aad8bca4edf` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_5da26d37c9094ad5bee770a3` | `mountRueSiteApp` 源码含查找容器、抛错、清空及挂载 |
| `rustcodegraph_node` | `call_900cb5546a3541638a3ae3e5` | `packages/runtime/src/hooks/useApp.ts:4` 的定义转调 `_$createApp` |

答复的主链与源码吻合。浏览器在 `app/app.tsx:54` 自动挂载，找不到 `#app` 时在第 42 行抛错。SSR 入口 `app/entry-server.tsx:31` 渲染静态文档，路由与 i18n 由 `app/app.tsx:34` 注册后在运行时挂载中安装。`RootApp` 对 `/rust-layers` 绕过布局，当前路由表未定义该路径。答复区分了图谱漏索引和源码定义，例如 `createAppRouter` 由符号查询返回空，按文件读取仍能验证 `app/router/index.ts:787`。答复将快照残留和重复 DOM 作为推断，给出预填容器与已部署静态页两个可检验场景。本轮未运行这些场景。

## 绑定、权限和收尾

- 六个真实会话的 UI summary 均回读 `acp-codex / codex` 和 `full_access=true`。完成后的 ACP options 均回读原提供商、全限定模型和 `session_model`，连接均已释放。挂载期间回读 `connected=true`、`access_status.support=native`。原生轨迹记录 `approval_policy=never` 和 `sandbox_policy=danger-full-access`，对应请求的原生权限模式。两轮对照均没有工具调用，挂载轮仅调用该只读服务。
- 两个私有副本均移除临时 `.vwacprc.json`，普通源码文件摘要在每组三轮前后完全相等，Codex 用户配置摘要相等。撤除对照会话无工具事件。两组挂载各有一个仅针对私有副本的 RustCodeGraph 子进程在会话后仍存活，按命令和路径确认归属后终止并确认退出。现有开发网关未重启。
- 提交快照已排除原工作树副本的工具失败。严格验收仍缺少纯标记 SSE 和仅三种工具的事件约束。保留任务文件。此次未修改运行时代码，未进行 Cargo 构建或单元测试。既有 Responses 中继保护用例预留给该边界发生变更或出现协议失败时复跑。原始记录保存在本机权限为 0700 的独立目录。

## 限定工具后的三轮复验

共享工作区后续提交未修改 Codex 受管入口、Responses 驱动或宿主选取 MCP 的路径。继续用相同网关二进制、适配器、RueJS 提交、提示哈希、租户和模型。宿主识别 `allowedTools`，但 ACP 建会话只转发服务声明，适配器的 stdio 转换也只有 `command`、`args` 和 `env`。同名 Codex 项目配置会优先于会话挂载。为了保留动态挂载和撤除对照，挂载轮使用私有 stdio 转发器，原样传递三种允许工具的参数与实际 RustCodeGraph 返回。它裁剪 `tools/list` 并拒绝名单之外的 `tools/call`。撤除时删除整个项目挂载条目。直接 MCP 检查确认原入口公开 8 项、可调用 files，过滤入口公开 3 项、拒绝 files，两种入口都查询到 `createRueSiteApp`。

| 轮次 | 新会话 | HTTP / curl | 秒 | SSE 终态 | 工具完成 |
| --- | --- | --- | --- | --- | --- |
| 无挂载 | `ses_1eff0afbb001FFGlLIYiMpM5tN` | 200 / 0 | 20.47 | `chat.done` ×1 | 0 |
| 挂载 | `ses_1eff10302001yTdYz2Uf21meXB` | 200 / 0 | 160.61 | `chat.done` ×1 | 52 成功、2 失败 |
| 撤除 | `ses_1eff37891001cWN5UgA76g55bU` | 200 / 0 | 21.14 | `chat.done` ×1 | 0 |

- 三轮无 `chat.error`。对照轮的原生最终答复精确为 `MCP_NOT_AVAILABLE`，网关 SSE 仍含已知元数据警告和带标签的推理文本。独立比对了原生答复、诊断和终态，未将警告当作推理或中继失败。
- 54 个函数调用均匹配同 ID 的原生结果。工具名仅有 query、callers、node，成功次数分别为 28、2、22。代表性输出包括 query `call_0acb671762134893b1c0666e` 命中 `app/app.tsx:32`，callers `call_d9853cdbefd64ea98d16b40b` 返回 `app/app.tsx:39`，node `call_2964f7d901d24e4c8dc0bf30` 返回 `useApp` 调用 `_$createApp` 的定义。两次失败来自模型请求不存在的 `app/router.tsx`（`call_a40637282a98474a917662e4`）及 `app/i18n.ts`（`call_8254a3ba0c0d44c8aabe86a2`）。实际定义均在各自目录的 `index.ts`。
- 最终答复复原了调用链及各分支，并把静态 DOM 重复列为预测。第二个场景使用的 `counter-button` 测试标记在该快照的应用源码中不存在，落地该断言需绑定真实交互控件或在测试夹具添加该标记。
- 三轮结束后选模均保持原绑定，连接已释放。挂载中原生权限回读为 `native`，原生轨迹仍为 `approval_policy=never`、`sandbox_policy=danger-full-access`。临时挂载已撤除，转发器与 RustCodeGraph 进程均退出，副本源码和 Codex 用户配置摘要相等。无运行时改动，未跑 Cargo。
- 状态仍为已阻塞。直连副本曾实现零工具失败，但使用了额外工具。过滤副本仅调用三种工具，但出现了可复现的真实工具失败。当前严格合同需要同一轮同时满足两项；恢复条件是指定模型在该只读目录完成零失败挂载轮，或验收方明确允许这些已恢复的探索读取失败。RustCodeGraph 若能索引当前遗漏的入口符号，也可在版本更新后原样复跑。
