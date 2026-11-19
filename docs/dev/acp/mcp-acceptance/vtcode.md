# VT Code 动态挂载 RustCodeGraph

2026-11-19，验收已阻塞。VT Code 0.183.0 在 ACP 建会话时忽略 `mcpServers`。宿主已连上 RustCodeGraph，指定模型在挂载轮仍报告 `MCP_NOT_AVAILABLE`。[VT Code 0.183.0 建会话实现](https://github.com/vinhnx/VTCode/blob/0.183.0/crates/codegen/vtcode-acp/src/zed/agent/session_state/mod.rs#L388-L412)

## 真实模型三轮

- Darwin arm64 私有回环网关复用已验收的 ShiJu 0.6.0 构建，二进制 SHA-256 为 `0257dd18ce1224e9d39956fae32db69ed9df3534a3158c90daaceb53798da951`。从已验证安装复制受管 VT Code 0.183.0，私有实例的安装端点成功返回。原生版本为 `0.183.0+4fd8e47a885c`，二进制 SHA-256 为 `88cbc99b8aac0c3a9b8596976c610026aebef14d89f9943735c38016a44919a4`，与仓库清单一致。[既有 VT Code 实机验收](../ACP验收.md#vt-code-补齐接入与最终实机验收2026-11-18)
- RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出干净副本，索引 3478 个文件。挂载轮在副本启用 `ruejs_graph_probe`，使用 RustCodeGraph 1.2.10 原生 stdio 和 `--no-watch`，宿主日志确认 MCP 服务完成初始化。第三轮先移除该配置，再建立新会话。
- 请求保留租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `session_model`，只替换预设和后端及私有目录和网关地址。提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。[复用请求](../ACP验收.md#可复用的验收请求)

| 轮次 | 新会话 | HTTP / curl 退出码 / 耗时 | SSE 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efc92ef2001bHipL6R5fRJdma` | 200 / 0 / 4.680 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efc941ac001oISQsKDcvSqK4k` | 200 / 0 / 4.115 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 撤除 | `ses_1efc95318001FcUcwiMsHiYDU3` | 200 / 0 / 5.152 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

三轮各收到一个 `chat.done`，均无 `chat.error`。UI summary 确认 `acp-vtcode / vtcode` 和 `full_access=true`。ACP options 运行中为 `connected=true`、权限支持为 `client_only`，选模回读确认指定模型和 `session_model`，终态回读连接均已释放。三次独立的自动标题请求使用另一模型，均收到 HTTP 403。

在 VT Code 的三个真实会话中，`rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 均无调用或回传。两轮对照符合预期，挂载轮返回了只适用于对照轮的标记，缺少调用链、分支行为及快照分析，无法验收答复准确性。

同一副本的独立 stdio MCP 调用三项均成功。它们用于排查服务端，不计入 VT Code 会话验收。

| 工具 | 独立探针输出 |
| --- | --- |
| `rustcodegraph_query` | `createRueSiteApp` 在 `app/app.tsx:32` |
| `rustcodegraph_callers` | `mountRueSiteApp` 在 `app/app.tsx:39` |
| `rustcodegraph_node` | `useApp` 在 `packages/runtime/src/hooks/useApp.ts:4`，定义调用 `_$createApp` |

## 首个失败边界与权限

- 仓库的 ACP 路径从工作区选择服务，经 `llm-runtime` 交给客户端，再填入 `session/new.params.mcpServers`。真实网关轮未抓取原始 ACP 帧。另用同一受管二进制、私有 HOME 和禁网沙箱发出独立 ACP 请求，捕获 `initialize` 返回 ACP v1、`loadSession=true`、`http=true`、`sse=false`，且空列表、stdio 和 HTTP 列表的新会话请求均成功。固定发行版的 `new_session` 对非空列表只发出忽略日志，随后建立会话。上游主分支仍有同一分支逻辑，HTTP 能力标记不能证明会话挂载已经实现。[固定版处理函数](https://github.com/vinhnx/VTCode/blob/0.183.0/crates/codegen/vtcode-acp/src/zed/agent/session_state/mod.rs#L388-L412) · [上游主分支](https://raw.githubusercontent.com/vinhnx/VTCode/main/crates/codegen/vtcode-acp/src/zed/agent/session_state/mod.rs)
- `vtcode.rs` 为每个受管启动准备私有 HOME、TOML 和请求绑定的回环模型中继。宿主拒绝 VT Code 的文件与终端回调，原生进程受 macOS 沙箱约束。沙箱禁止 fork，只允许执行已校验的 VT Code，网络仅允许模型中继的回环端口。客户侧自动审批不会赋予进程启动 MCP 子进程的权限。模型 Chat 中继的 VT Code 工具白名单也仅包含 `read_file`、`list_files`、`code_search` 和 `apply_patch`。这些限制位于 ACP 建会话忽略列表之后，单独改传输或白名单无法修复首个断点。
- 上游 TOML 支持启动期 MCP provider。直接写入 stdio provider 仍受子进程限制，独立 HTTP 端口也不在当前沙箱的网络范围内。仓库未找到可直接复用、同时保留会话授权与撤除的 VT Code MCP 转发入口。本次保留权限边界和现有驱动。复跑需要经审查的后端支持，能消费会话服务列表并在现有权限下把同一 RustCodeGraph 工具的调用结果送回模型。[VT Code MCP 配置](https://github.com/vinhnx/vtcode/blob/main/docs/guides/mcp-integration.md)

私有网关已退出，监听端口已关闭，私有授权文件和副本中的临时挂载配置已清理。原 RueJS 与用户全局配置未写入。原始证据保留于本机 0700 目录。本轮仅提交验收记录。现有 `onboarding_vtcode` 生命周期夹具覆盖建会话、继续和取消，没有改变这些边界，未运行 Cargo。

## 共享构建与三工具转发复验

复用共享槽位 3 的 ShiJu 0.6.0 构建副本，SHA-256 为 `1a033a19a5c5e2c115e90abbd336f898c0df3ac101a9824ffe8b36f11b0bcc47`。受管 VT Code 仍为 0.183.0。固定 RueJS 提交、提示、服务名、租户、指定模型、`session_model` 和 `full_access=true`。挂载轮复用 Goose 的私有 stdio 转发器，仅声明三种真实图谱工具，补齐分页和实际入口元数据，注明 `projectPath` 可省略且仅根目录存在索引。转发保持参数和上游结果。私有网关确认挂载时宿主侧 MCP 初始化成功。[Goose 三工具转发基线](goose.md#通过的三轮)

| 轮次 | 新会话 | HTTP / curl 退出码 / 耗时 | SSE 结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0579fc4001wolHPOEW2VLnNM` | 200 / 0 / 4.488 秒 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f057b26e001J5Ku0m9YgFH6a8` | 200 / 0 / 4.038 秒 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 撤除 | `ses_1f057c3b9001qeyaKgLfTe70Nx` | 200 / 0 / 3.561 秒 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

每轮新建会话并收到一个 `chat.done`，均无 `chat.error` 或模型恢复事件。UI summary 回读后端绑定和全访问请求，ACP options 在线时读到 `client_only` 与指定模型，终态三次均释放连接。挂载仍未回传三种工具的结果，答复准确性无法验收。另有自动标题模型请求返回 HTTP 403。

独立诊断沿用 OMP 的同端口办法。在沙箱允许的唯一回环端口提供模型探针和带独立占位令牌的 MCP HTTP 转发。用同一 VT Code 二进制比较三次原生 ACP 初始化及提示，配置分别为空列表、建会话携带 HTTP 服务，以及在建会话携带 HTTP 的同时启用私有 TOML 的 `mcp.providers`。三次 ACP 提示均返回 `end_turn`，模型探针均只收到相同的五种原生工具声明，HTTP 服务没有收到 VT Code 发来的 MCP 请求。模型探针位于生产中继工具过滤之前，使用占位密钥及诊断答复。随后用宿主客户端请求同一 HTTP 路由，成功列出三种限定工具，并收到真实 `query`、`callers` 和 `node` 结果，分别命中 `app/app.tsx:32`、`app/app.tsx:39`、`packages/runtime/src/hooks/useApp.ts:4`。这些控制调用不计入真实会话验收。[OMP 同端口诊断](omp.md#同端口传输和权限诊断)

VT Code 的 ACP 注册表由 `ZedAgent::new` 初始化。启动期 MCP provider 的初始化在交互运行循环中，固定版 ACP 路径未接入。Goose 和 Hermes 的目录转发依赖原生 ACP 接入。OMP 的办法依赖原生 ACP 消费 HTTP 服务。两种前提在 VT Code 固定入口均未满足。Pi、OpenClaw 的受管适配各自依赖已接线的引擎入口，VT Code 没有等效入口。复跑所需条件为可按仓库契约安装的受审 VT Code 原生 ACP 发行，具备会话级 MCP 注册、宿主审批和同一调用 ID 的结果回传。若由本仓库维护该能力，则需要另行确定原生补丁的构建、交付和摘要审查归属。独立转发器和同端口配置的证据已取得，继续调整工具描述无法解决注册边界。[VT Code ACP 注册表](https://github.com/vinhnx/VTCode/blob/0.183.0/crates/codegen/vtcode-acp/src/zed/agent/mod.rs#L88-L118)

本轮只更新验收记录。未修改 Rust 或运行 Cargo。私有网关、原生诊断和图谱进程已退出，临时挂载与私有授权文件已清理。

## 原生 ACP 源码补丁

在 VT Code 0.183.0（`4fd8e47a885c`）的独立源码副本实现会话级 MCP。补丁复用原生 MCP 客户端和 ACP 工具审批，将 HTTP 工具目录保存在新建会话的内存状态中。调用用原参数并沿用模型工具调用 ID 回传输出。原型只接受有宿主令牌的回环 HTTP，保留原有进程及网络沙箱。[源码补丁与探针](../../../../patches/acp-model-source/vtcode/README.md)

原版 0.183.0 的聚焦探针通过无挂载和撤除两项对照。挂载正例因模型请求缺少三种 MCP 声明而失败。补丁在原版源码的应用检查、在修改副本的反向检查及修改文件的格式检查均通过。

安装上游指定的 Rust 1.100.0，并完成固定锁文件的 macOS arm64 依赖下载。共享工作区连续运行其他 Rust 构建，本轮预留的串行窗口未取得，原型尚未编译或执行绿灯探针。任务仍需验证补丁制品上的批准、拒绝、同一调用 ID 回传、重放拒绝与撤除，然后接入会话绑定的受管中继并使用原模型复验。当前受管发行继续使用仓库现有固定入口。

### 原生补丁构建与聚焦探针

使用 Rust 1.100.0 和固定锁文件从 VT Code 独立源码副本构建 `vtcode` CLI。槽位 3 的 `cargo build --offline --locked -j 1 -p vtcode --bin vtcode` 退出 0，耗时 16 分 2 秒。两个构建缓存均使用槽位 3。产物自报 VT Code 0.183.0，SHA-256 为 `8b8f6c5204e1125b20d63c7c9637b027821b22d206517ebd00db5f513890cdfb`。

原生聚焦探针四项通过。空挂载和撤除轮各一次模型请求、零工具。批准轮四次模型请求，真实 RustCodeGraph 的 `query`、`callers`、`node` 各执行一次，结果与参数按原调用 ID 回读。拒绝轮两次模型请求，只有审批及失败事件，图谱执行为零。模型探针覆盖无效令牌和已消费授权重放，两者均被拒绝。ACP 开始帧允许省略默认 `pending` 状态，探针按协议补齐该默认值后校验事件序列。

### 隔离原生 ACP 的真实模型初跑

原版 VT Code 在同一私有转发、同一真实模型和原提示的无挂载对照返回精确标记。补丁版挂载轮收到 49 个模型 HTTP 200，实际图谱执行 116 次，其中 query 49、callers 10、node 56 次成功。首次工具错误把未独立索引的 `packages/rue` 传作 `projectPath`。真实结果随原调用 ID 进入后续模型请求。五分钟时终止诊断，挂载轮没有终答。另两轮零工具并正常结束，但模型用正文模拟 MCP 发现调用，没有返回精确标记。这组证据验证工具回传，未通过严格三轮合同。

第二组按模型可见元数据将 `projectPath` 明确限定为可省略的工作区根，配置挂载时仅暴露三种图谱工具，对照时沿用 VT Code 中继原生工具白名单。两次对照均以 `end_turn` 返回精确的 `MCP_NOT_AVAILABLE`，零工具。挂载轮完成 query 22、callers 2、node 35 次，图谱失败为零。当第 30 个模型请求建立 TLS 时底层连接提前断开，原生 ACP 返回网络错误。两个对照和全部工具调用均保留，挂载轮仍需终答。

### 隔离原生 ACP 取得真实模型终答

沿用第二组工具目录、原提示与模型。无挂载和撤除使用第二组的有效对照，挂载轮复跑。一处 TLS 建连错误在私有转发恢复，送达模型的 60 个请求均为 HTTP 200。三次原生独立进程均收到 `end_turn`，终态后由探针回收。两个对照只返回 `MCP_NOT_AVAILABLE`，审批与工具调用均为零。

挂载轮共 137 次逐次批准和完成事件，图谱调用全部成功，也没有额外工具调用。全部完成事件的 ID 均原样出现在模型续轮的工具结果中。

| 工具 | 代表性调用 ID | 输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_ee1b0edbc6c24175bbda6357` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_9d96b4dd0ea646e8900a48f9` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_ee69dccd0a174c1e8989fd1b` | 返回 `app/app.tsx` 的启动和布局分支 |

答复复原调用链、SSR 守卫、缺少容器的异常、路由和 i18n 安装及布局条件，并用源码推断未清空旧快照时会追加挂载。两个回归场景检查首页头部重复和预填模板的陈旧节点残留。两个场景均未执行。答复对第二个场景的精确计数需要修正，仅含 `data-stale` 的旧节点不会使新组件标记从一次变成两次。布局判断使用规范化的路由路径，查询字符串不直接参与该比较。

原生进程沿用私有 HOME、禁止子进程、单回环端口和源码只读。图谱进程只可写私有索引目录以便 SQLite 创建临时文件。RueJS 4108 个源码文件与符号链接和固定归档一致，索引条目单独检查，监听与子进程已关闭。此证据来自独立 ACP，受管网关仍需挂载目录接线、补丁产物的受审安装和原 curl 三轮回归。后续改动按用户要求保留在工作区，尚未提交。

### 官方工具共存复验

按新标准，模型目录同时公布原中继允许的 `read_file`、`list_files`、`code_search`、`apply_patch` 和已挂载的三种图谱工具。官方工具沿用原审批及沙箱。隔离原生 ACP 复用同一补丁产物、指定阿里模型、原提示和只读源码副本。两个对照沿用前次等效证据。

挂载轮 29 个模型请求均为 HTTP 200，收到 `end_turn`。query 33 次、callers 7 次、node 39 次，全部真实执行成功。79 条完成事件的调用 ID 均在后续模型请求中匹配到工具结果。模型本轮选择了图谱工具，答复覆盖调用链、SSR、缺失 `#app`、路由/i18n、布局与快照推断。单独的聚焦批准轮在同一会话先调用原生 `list_files`，获准后返回 `app.tsx`，随后三个挂载调用分别执行并回传。拒绝和撤除对照仍为零图谱调用。复用现有编译产物，探针及转发进程均已回收。受管网关仍需接入，改动未提交。
