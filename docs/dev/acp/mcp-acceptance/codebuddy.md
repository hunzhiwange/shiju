# CodeBuddy 动态挂载 RustCodeGraph

2026-11-19 使用 [可复用的验收请求](../ACP验收.md#可复用的验收请求) 的 `189d22dc1` 版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。各轮使用租户 `10001 / 100010005`、`acp-codebuddy / codebuddy`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

私有回环网关复用现有 shiju 0.6.0 构建，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。使用开发数据的私有 SQLite 备份及同一授权，仅在私有网关启用内置 CodeBuddy 预设并指向受管安装的副本。Darwin arm64 上的入口自报 `2.179.3`，三个发行 JS 文件及入口与仓库的固定摘要一致。诊断 `initialize` 返回 ACP v1、`loadSession=true` 和 MCP HTTP、SSE 支持。真实请求使用原 RustCodeGraph 1.2.10 的 stdio 与 `--no-watch`。

RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出到私有目录，索引 3478 个文件，命中 `createRueSiteApp`（`app/app.tsx:32`）。挂载轮仅在副本中启用 `ruejs_graph_probe`，第三轮前撤除。挂载期间观察到 CodeBuddy 进程启动原 RustCodeGraph stdio 子进程。下文以 SSE 同一调用 ID 下的工具参数与输出核对挂载结果。

独立 ACP 探针也用同一受管入口向 `session/new` 发送含一个 stdio 服务的 `params.mcpServers`，得到新原生会话 ID。探针只验证握手及建会话。正式轮次的推理均使用指定阿里模型。宿主 `new_session_info` 将当前 MCP 选择传入原生 `session/new`。CodeBuddy 模型驱动仍使用隔离的配置目录和会话级回环模型中继。

## 三轮结果

首个不符合合同的边界是无挂载轮的模型工具选择。没有目标 MCP 调用，但 CodeBuddy 执行了内置 `ToolSearch` 三次和 `WaitForMcpServers` 一次。可见答复包含 `MCP_NOT_AVAILABLE` 及额外解释与建议，未满足只返回标记的要求。

| 轮次 | 新会话 | HTTP / curl | 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef92710c001OhVNsqNxQF7sr7` | 200 / 0，25.744 秒 | `chat.done` |
| 挂载 | `ses_1ef92d633001DITOWTbnovqqC6` | 200 / 28，300.001 秒；续传 200 / 0，194.740 秒 | 续传收到 `chat.done` |
| 撤除 | `ses_1ef9a653f001bsRSPNVdlXU5Rr` | 200 / 0，24.751 秒 | `chat.done` |

挂载轮按原流请求 ID 和 `Last-Event-ID: 2543` 从 2544 继续，没有重复提交请求。三轮均无 `chat.error`。网关日志另有会话标题生成错误，主推理仍返回终态及答复。撤除轮没有目标工具调用，再次执行三个内置搜索及一次就绪检查，可见答复包含标记和额外解释。

| 工具 | 代表性 SSE 调用 ID | 实际输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_88def52b395d4013bd059546` | 找到 `createRueSiteApp`（`app/app.tsx:32`） |
| `rustcodegraph_callers` | `call_5531a2da9a9f481db66dc27e` | 返回调用者 `mountRueSiteApp`（`app/app.tsx:39`） |
| `rustcodegraph_node` | `call_aaa2b84931864ffe941511ce` | 返回 `app/app.tsx`，含浏览器挂载条件、容器判空、清空及挂载 |
| `rustcodegraph_node` | `call_6a0b97dea7d94bd3b5507b22` | 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）及对 `_$createApp` 的转调 |

这些调用均有开始和完成事件。目标工具通过 `DeferExecuteTool` 调用，SSE 的结构化参数带完整 `mcp__ruejs_graph_probe__rustcodegraph_*` 名称。完成事件包含 query 41 次、callers 10 次和 node 76 次。node 的两次完成事件在输出中报告文件不存在，涉及 `app/router.ts` 和 `app/router.tsx`。同一服务另收到 status 1 次、files 10 次、explore 7 次，CodeBuddy 内置 `ToolSearch` 另完成 2 次。未观察到 shell、写入或其他 MCP 服务调用。

挂载答复的主链、浏览器判定、`#app` 缺失抛错、挂载阶段安装路由与 i18n、`/rust-layers` 条件绕过布局均可回对源码。路由表没有注册该路径，答复也指出顶层调用与部分符号存在图谱缺边或漏索引。移除清空逻辑的主要推断成立，运行时会把根块追加到已有快照后。答复提出产物冷启动和预填容器的二次快照验证，均未实际执行。第二个场景的 `article.prose` 选择器与 `GuideDocDetail.tsx` 实现不符，应使用源码中的 `#doc-body` 定位。产物重复 DOM 的推断适用于加载完整客户端入口的页面。

三轮的 UI summary 均回读绑定 `acp-codebuddy / codebuddy`、`full_access=true`。ACP options 在运行中及结束后均保留指定模型和 `session_model`，连接在每轮结束后释放。运行中报告 `access_status.support=partial`。挂载期间读取权限队列为空，三轮没有人工审批回复。独立 ACP 探针读到默认模式为 `default`，公开 `bypassPermissions` 和 `fullAccess` 两个不同选项。宿主策略按 `bypassPermissions` 处理该访问级别，本次未调用可能触发危险命令检查的工具。

## 原生配置复验

仅在私有租户配置中设置 `CODEBUDDY_DEFER_TOOL_LOADING=0` 与 `CODEBUDDY_WAIT_FOR_MCP_SERVERS_ENABLED=0`，用相同请求重跑。SSE 的工具标题直接显示原生 MCP 工具名。此配置不改模型、挂载范围或权限。

| 轮次 | 新会话 | HTTP / curl | 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef9acc55001WpBl8nHRqo2uCq` | 200 / 0，11.843 秒 | `chat.done`，可见答复仅 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1ef9b4cef001Dz2yBMqKQlENdj` | 四段均 200，curl 依次为 28、28、28、0 | 同一请求续传收到 `chat.done` |
| 撤除 | `ses_1efa91240001TbZoCuQ6uZzuU0` | 200 / 0，12.545 秒 | `chat.done`，可见答复仅 `MCP_NOT_AVAILABLE`，零工具事件 |

复验均无 `chat.error`。挂载轮从原请求游标续传三次，最后一次耗时 0.003 秒。三种目标工具均成功，query 171 次、callers 10 次、node 53 次。同一 MCP 另运行 files 6 次、explore 41 次、status 和 callees 各 1 次。仍能核对主调用链和清空快照的推断。答复中的导航和布局计数预测仍需在实际加载完整客户端入口的静态页上验证。复验也回读到相同绑定、模型及运行中的 `partial` 权限状态，挂载时读取权限队列为空。

## 结论与收尾

原生动态挂载和撤除隔离已有真实证据。严格合同受额外工具调用阻塞，复验配置也未解决挂载轮的工具范围偏离，故未写入代码默认值。未改驱动或受管补丁，无转发故障需要修复，未运行 Cargo。

副本撤除了临时 MCP 配置，与归档的文件和符号链接比较无差异（索引除外）。私有网关和属于本任务的 MCP 子进程均已退出，端口已关闭，凭据副本已清理。现有用户网关继续运行，读回 CodeBuddy 预设仍停用。脱敏记录之外的原始 SSE 与诊断日志仅保存在本机 0700 目录。

## 共享构建的三工具复验

使用当前共享槽位已有 shiju 0.6.0 构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。重建私有租户和原 RueJS 提交的索引，沿用原提示、选模、CodeBuddy 2.179.3、两项原生环境设置及权限。CodeBuddy 驱动、受管补丁和权限策略与上一轮一致。

借用同批次已验证的 stdio 转发方式。宿主仍通过 `session/new` 传入 `ruejs_graph_probe`，转发进程只公布 query、callers、node，拒绝其他调用名及副本外路径，其他参数和返回原样通过 RustCodeGraph 1.2.10。本机协议检查确认直连工具 8 项、转发工具 3 项，拒绝 files。两条链返回相同的符号命中、分页源码与缺失文件错误。没有挂载时不启动此转发器。

第二组在同一归档提交的另一副本中，补充了旧 MCP schema 未公开的 `node.offset` 和 `node.limit`，并说明文件参数必须为真实路径且包含扩展名。协议检查验证请求从第 787 行起读三行，命中路由工厂。此调整只涉及工具目录元数据，实际请求和结果仍原样转发。

| 工具目录 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态和工具 |
| --- | --- | --- | --- | --- |
| 三工具 | 无挂载 | `ses_1f0039af6001mibQP8QMO85WTy` | 200 / 0 / 10.101 | `chat.done`，精确可见标记，零工具事件 |
| 三工具 | 挂载 | `ses_1f003c3c3001KneWM1rnSZIzDC` | 200 / 28 / 300.009，续传 200 / 0 / 187.224 | `chat.done`，139 成功、4 失败 |
| 三工具 | 撤除 | `ses_1f00b350d001wJosgqOZ7NvFDg` | 200 / 0 / 10.703 | `chat.done`，精确可见标记，零工具事件 |
| 补全元数据 | 无挂载 | `ses_1f007ec56001hXRorTXa9BaGlM` | 200 / 0 / 11.008 | `chat.done`，精确可见标记，零工具事件 |
| 补全元数据 | 挂载 | `ses_1f0081896001gkpMQ9veSHR09e` | 200 / 0 / 244.518 | `chat.done`，114 成功、4 失败 |
| 补全元数据 | 撤除 | `ses_1f00bd43e001kEFb2K5kCMBmXW` | 200 / 0 / 8.787 | `chat.done`，精确可见标记，零工具事件 |

六轮无 `chat.error`。UI summary 和 ACP options 回读验证了原绑定、`full_access=true`、指定模型和 `session_model`。运行期间权限报告 `partial`，终态采样确认连接释放。挂载轮只有三种指定工具，未出现其他工具调用。

第一组 query `call_9ae3a0db72c345328007f2d8` 返回 `createRueSiteApp`（`app/app.tsx:32`），callers `call_1d466b25b3ea44dea1ddc75a` 返回 `mountRueSiteApp`（第 39 行），node `call_b5ad009e15fb49bc9d165265` 读回 `useApp.ts:4` 转调 `_$createApp`。首次失败 `call_1007797778864868bf02dea8` 把无扩展名的模块路径当成文件，随后又读取了三个不存在的路由文件。

第二组路由工厂的真实源码已进入答复（`app/router/index.ts:787`）。首次失败 `call_f9c41ce6e32c4f0d8e269158` 仍猜测 `app/router.tsx`，另外三次分别读了不存在的 `app/router.ts`、`app/i18n.ts` 和 `app/static-render-entry.tsx`。两组最终答复覆盖主链、SSR、容器缺失、路由和 i18n、布局绕过及快照。第一组提出的 `/rust-layers` 验证不可直接执行，该路径未注册。第二组提出实际存在的示例页及预填容器场景，重复节点属于源码推断，两场景未实际运行。

严格验收仍为已阻塞。两组的真实读取错误直接违反挂载轮合同，区别于无关回归失败。恢复条件是指定模型在原提示与真实索引下完成零失败的三轮，或上游 RustCodeGraph 改善当前漏索引定义的发现能力后复验。接受已恢复的探索读取失败需要验收负责人调整合同。此次未修改运行时代码，未运行 Cargo。两份副本与归档比对无差异（索引除外），私有挂载、网关、转发器、图谱进程及凭据副本均已清理。

## RustCodeGraph 1.2.11 发行版复验

使用槽位 3 已受审的 shiju 0.6.0（SHA-256 `2e84923a46704187dfd9d56dfc4b45d6ebf9cfc93398a6bb3a82b1cc61a1f935`）、CodeBuddy 2.179.3、之前证明可用的两项原生环境设置，以及相同请求、租户、模型与权限。以 RustCodeGraph 1.2.11 发行二进制（SHA-256 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`）对原 RueJS 提交重新索引 3478 个文件，`createAppRouter` 可直接命中 `app/router/index.ts:787`。[发行版及同批次验收](pi.md)

项目配置沿用 `allowedTools`。当前宿主对 CodeBuddy 转发标准 ACP 服务描述，三工具边界由同批已审验的私有 stdio 目录转发器执行。目录补充真实入口文件、`node.offset`、`node.limit` 及默认索引根，模型可以省略 `projectPath`。协议检查确认原生目录 8 项、限定目录 3 项，三种工具的真实返回与直接 stdio 完全一致。额外工具和越界路径被拒绝，副本内不存在的文件仍返回原始图谱错误。没有更改模型工具参数或返回。

首组 1.2.11 挂载完成 109 次指定工具调用，其中两次返回真实错误。`call_3bb37bbf81e5433c98fc4a4f` 把 `packages/router` 当成独立索引根，`call_3d1080805b6c468380a300fc` 请求了未被索引的 `index.html`。撤除轮没有工具事件，标记后带解释。三轮均收到 `chat.done`。为验证这两个具体边界，第二份干净副本在同一目录声明中把可选 `projectPath` 枚举为实际索引根，且将 `node.file` 声明为源码扩展名模式。未增加参数修正或结果替换，独立 stdio 探针再次验证三种成功结果及缺失路径错误与直连相同。

| 最终轮次 | 新会话 | HTTP / curl / 秒 | 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f10172ad001btrvIl8kiqPfdm` | 200 / 0 / 10.008 | 一次 `chat.done`，可见答复仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f1019ad0001xJ58m3So6xz3nz` | 200 / 0 / 178.524 | 一次 `chat.done`，80 次指定工具调用全部成功 |
| 撤除 | `ses_1f104549e001Q6VkCpXBzVZV95` | 200 / 0 / 11.382 | 一次 `chat.done`，可见答复仅 `MCP_NOT_AVAILABLE`，零工具 |

挂载轮 32 次 query、3 次 callers、45 次 node。80 个调用 ID 均配对开始、成功完成和真实输出，零额外工具、工具失败、模型恢复及 `chat.error`。代表性调用 `call_26dfd24f95c94964a45b86cf` 返回 `createRueSiteApp`（`app/app.tsx:32`），`call_969a230c44d24ce2ac833c66` 返回 `mountRueSiteApp`（第 39 行），`call_85a5e894395e49079dc6b795` 读回 `useApp.ts:4` 对 `_$createApp` 的委托。另由 `call_facb2cfb639a4c34afded262` 命中应用工厂（`compiler-runtime/app.ts:57`），`call_5861909adb0949ea841ed109` 命中路由工厂。

答复主链、SSR 与浏览器分流、容器判空、插件安装顺序及布局绕过条件与源码对应。它分析了移除清空造成的快照残留，给出首屏去重和卸载容器两项可检验预期。实际路由表未注册 `/rust-layers`，答复提到的 SSR 与客户端布局差异只在该条件可达时成立。场景 A 应以旧节点身份或已知布局节点计数为依据，根子元素数量取决于产物结构。场景 B 应使用 `mountRueSiteApp().app.dispose()`，站点函数返回的是 `{ app, router }`。本次未运行浏览器场景。

最终三轮均回读 `acp-codebuddy / codebuddy` 和 `full_access=true`。运行中 ACP options 确认指定租户模型、`session_model` 和 `partial` 访问级别，终态连接全部释放。动态挂载三轮验收通过。未修改产品代码或运行 Cargo。两份副本的文件和符号链接与归档一致（索引除外），临时挂载、私有网关、端口、工具子进程和凭据副本均已清理。原始事件及探针结果保留在本机 0700 目录。
