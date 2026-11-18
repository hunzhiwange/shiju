# Hermes Agent 动态挂载 MCP 验收

2026-11-19。受管安装补齐原生 MCP SDK。共享构建以限定的真实 RustCodeGraph 工具目录通过三轮验收。初验曾出现额外工具调用和不可直接执行的验证场景。

## 环境与首个失败边界

- 在独立回环网关复用 shiju 0.6.0，二进制 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。仅在隔离数据副本启用 `acp-hermes`。最初三轮创建了宿主会话，HTTP 200、curl 0，均以 `chat.error` 结束，报 `ACP session_model requires Provider credentials`。将既有阿里授权文件中对应提供商条目加入隔离网关后复跑，原模型可用。
- 使用 [可复用的验收请求](../ACP验收.md#可复用的验收请求) 在 `189d22dc1` 的版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。保持租户 `10001 / 100010005`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model`、`full_access=true` 和服务名 `ruejs_graph_probe`。RueJS 使用 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的干净副本。RustCodeGraph 1.2.10 索引 3478 个文件。
- Hermes 自报 0.19.0，Darwin arm64、Python 3.12.16。受管 PyPI wheel SHA-256 为 `bd0bac012aee38a60894781f4597dc29ee7bedb3448540249921f10d3bef327f`。独立 ACP 探针返回协议 v1，声明 load、resume、fork 和 list，没有声明 HTTP 或 SSE MCP。实际选择原生 stdio，加 `--no-watch`。探针的 `session/new` 发出非空 `mcpServers` 并收到新原生会话 ID。真实网关轮由宿主解析工作区挂载，未开启原始 ACP 线包日志。
- 修复授权后，原受管安装的三轮都有 `chat.done`、零 `chat.error`、零工具事件。挂载会话 `ses_1efad001f001H9Kcv4qGwsm7ld` 在宿主侧已初始化 RustCodeGraph，但模型仍报告 MCP 不可用。独立探针进入 Hermes 的注册与工具列表刷新路径，只得到 15 个内置工具。其 Python 环境没有 `mcp`，`tools.mcp_tool._MCP_AVAILABLE` 为 false，原生 `register_mcp_servers` 在连接前提前返回。这解释了挂载定义已被接收却没有进入模型工具列表。
- 将 Hermes 发行安装从 `[acp,cli]` 改为 `[acp,cli,mcp]`。发行元数据已固定 SDK 为 1.26.0。使用独立的受管 v2 安装路径识别旧发行，并校验 SDK 入口摘要。实际执行仓库安装脚本冷安装 v2，退出 0，十个契约文件的摘要相符。用冷安装入口重复独立 ACP 探针，原生注册 8 个 RustCodeGraph 工具，挂载会话刷新后有 23 个工具。Hermes 按所选会话工具集暴露工具，原生连接注册表按进程保存。本轮实测采用独立 ACP 进程和私有 `HERMES_HOME`，撤除后通过新会话复验。
- 模型驱动继续使用每次请求生成的私有 `HERMES_HOME` 和模型回环中继。仅网关使用阿里授权，探针用本地占位配置。挂载通过原生 `session/new`，因此无需把工作区 MCP 写进用户或私有 Hermes 配置文件。未修改恢复、全访问校验或审批路径。

## SDK 补齐后的真实三轮

真实模型复验使用旧网关已校验的 Hermes 入口，在其私有发行副本安装与冷安装相同的 SDK 1.26.0。该复验与冷安装探针分别记录。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 与可见答复 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efaf3d0d001isCzrQU2OFD4Rf` | 200 / 0 / 7.128 | `chat.done`，去除推理标签后仅 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efaf5919001UvhxXAQQuJfYCn` | 200 / 0 / 164.238 | `chat.done`，48 次工具调用均配对记录开始和完成 |
| 撤除 | `ses_1efb1dadd001qFiwRbB8YJI1gu` | 200 / 0 / 6.579 | `chat.done`，去除推理标签后仅 `MCP_NOT_AVAILABLE`，零工具事件 |

三轮各一条 `chat.done`、零 `chat.error`。对照轮的原始 `chat.delta` 仍含推理标签及内容。挂载轮 48 次调用全部来自 `ruejs_graph_probe`。query 5 次、callers 4 次、node 30 次，另调用 files 4 次、explore 4 次和 status 1 次。未观察到其他服务、内置工具或 shell 调用。使用同一证据断言验证目标三类工具有配对及输出、对照无调用，原安装失败，补齐 SDK 后通过。严格合同仍拒绝挂载轮的额外图谱工具。

| 目标工具 | 代表性 SSE 调用 ID | 实际输出 |
| --- | --- | --- |
| `rustcodegraph_query` | `tc-c92f91b4dfe9` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `tc-36532217eb50` | 调用者 `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `tc-374343bf62ae` | 返回入口实现，含 `#app` 判空、清空、挂载和浏览器守卫 |
| `rustcodegraph_node` | `tc-90c04d645f19` | `packages/runtime/src/hooks/useApp.ts:4` 转调 `_$createApp` |
| `rustcodegraph_node` | `tc-bb21d5f977a1` | `packages/runtime/src/compiler-runtime/app.ts:57` 的应用工厂及插件安装 |

答复的主链、浏览器与 SSR 入口、容器缺失抛错、插件在根组件前安装及 `/rust-layers` 布局条件均可对回源码。移除 `replaceChildren()` 后，完整客户端入口会把新根块追加到现有快照，属于源码推断。答复给了静态文档页 DOM 唯一性和客户端布局绕过两项验证设想，两项均未运行。第二项涉及的 `/rust-layers` 在当前路由表无注册，且需在初始 `#app` 中预放旧快照并加载完整客户端入口，答复未满足可直接执行的条件。答复对所有文档入口都会重复挂载的预测也超出了证据范围。

## 权限和收尾

三轮 `ui/summary` 均为 HTTP 200，回读 `acp-hermes / hermes` 及 `full_access=true`。`acp-options` 回读原模型和来源 `session_model`。挂载轮进行中为 `connected=true`、`access_status.support=partial`，权限队列为空。原生探针初始模式为 `default`。宿主分别处理 Hermes 编辑审批和终端权限，本次实际只调用了只读图谱工具，未实测编辑或终端审批。三轮结束后各自释放 ACP 连接。网关另有标题生成模型的 HTTP 403，三轮使用的会话模型均返回正文和终态。

仓库的公共转发与会话驱动未改，未运行 llm-runtime 定向测试或 Cargo 构建。验证覆盖 installer 冷安装、契约文件校验、独立 ACP 探针、真实模型对照和 `git diff --check`。私有网关及其 RustCodeGraph 子进程已退出，端口关闭，凭据及租户数据库副本已清理。RueJS 副本源码及符号链接与归档一致，临时挂载配置已撤除。现有网关继续运行，其 Hermes 预设仍停用。原始事件和诊断日志保存在本机私有目录。

## 共享构建复验

复用共享槽位 6 的 shiju 0.6.0 构建副本，SHA-256 为 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`。私有网关报告 Hermes 已安装，真实会话启动经校验的 0.19.0 受管 v2 入口。三组均保持原始提示、同一租户和模型、全访问授权，并使用同一 RueJS 提交的独立副本。

复用同批次的临时 stdio 转发方式，只公布 query、callers、node，保持真实请求和图谱结果。协议检查确认符号查询、调用者、从第 787 行开始的分页源码，以及拒绝额外工具和副本外路径。三组分别采用 RustCodeGraph 1.2.10 与分页元数据、1.2.11 与真实入口文件元数据、1.2.10 与同样的入口元数据。1.2.11 能索引 `createAppRouter`，但本组对真实根 `index.html` 的 node 请求返回未索引。

| 工具目录 | 轮次 | 新会话 | HTTP / curl / 秒 | 终态与结果 |
| --- | --- | --- | --- | --- |
| 1.2.10 分页 | 无挂载 | `ses_1f036c42b001jHeOu6ATkgF44D` | 200 / 0 / 6.332 | `chat.done`，精确可见标记，零工具 |
| 1.2.10 分页 | 挂载 | `ses_1f036dd0c001xnF4Lt0pkY50e5` | 200 / 0 / 239.754 | `chat.done`，72 次成功、2 次读取失败 |
| 1.2.10 分页 | 撤除 | `ses_1f03a85c2001pgnwrSU233Zvcm` | 200 / 0 / 6.177 | `chat.done`，标记附说明，零工具 |
| 1.2.11 入口 | 无挂载 | `ses_1f03a8a7d001JSEwKw80Y8KFZ0` | 200 / 0 / 5.370 | `chat.done`，标记附说明，零工具 |
| 1.2.11 入口 | 挂载 | `ses_1f03a9fb3001gN2NbFbwL00Ics` | 200 / 0 / 138.702 | `chat.done`，55 次成功、2 次读取失败 |
| 1.2.11 入口 | 撤除 | `ses_1f03cbdae001Urt1EUlyF7NzbW` | 200 / 0 / 5.553 | `chat.done`，标记附说明，零工具 |
| 1.2.10 入口 | 无挂载 | `ses_1f03bfc1b001G5KVsWfVH7CiW6` | 200 / 0 / 5.212 | `chat.done`，精确可见标记，零工具 |
| 1.2.10 入口 | 挂载 | `ses_1f03c10ad001ay8Y19YCgPuH3h` | 200 / 0 / 161.431 | `chat.done`，59 次成功，零读取失败 |
| 1.2.10 入口 | 撤除 | `ses_1f03e876d001fSyp7Xd70V40D0` | 200 / 0 / 4.828 | `chat.done`，精确可见标记，零工具 |

九轮均无 `chat.error` 或模型恢复事件。三次挂载只调用指定的三类工具。第一组猜测了不存在的 `app/router.ts` 和 `app/i18n.ts`。第二组的两次读取失败涉及根 `index.html` 及不存在的 `packages/router/src/useRoute.ts`。工具包把这些读取结果标为已完成，输出本身仍保留了读取失败。

第三组通过三轮验收。两个对照的最终可见文本仅为 `MCP_NOT_AVAILABLE`。原始 SSE 仍带现有的 `<think>` 区块，三个会话均完成推理。挂载轮的 query、callers、node 分别成功 19、3、37 次。query `tc-463311068316` 返回 `createRueSiteApp`（`app/app.tsx:32`），callers `tc-b6a2a56981b8` 返回 `mountRueSiteApp`（第 39 行），node `tc-f15884980ba3` 返回实际挂载源码，`tc-7f12f14b40d2` 返回 `useApp` 对运行时工厂的转调。59 个调用均能配对到开始和完成事件，未发现空输出、节点读取错误或额外工具。

答复用行号复原主链、SSR 入口、缺少容器时的抛错、路由和 i18n 安装，以及当前路由表未注册 `/rust-layers` 这一约束。两项场景以实际存在的 `/design/button` 为例，分别检验快照首屏是否只出现一份布局和客户端导航后是否残留旧快照。重复 DOM 是源码推断，两项场景均未执行。答复对旧链接点击的附加预测需要收窄。旧 DOM 的链接仍有原生 `href`，可能触发整页跳转，验证应区分客户端路由接管与浏览器普通导航。

九轮终态回读绑定 `acp-hermes / hermes`、`full_access=true`、原模型和 `session_model`，ACP 连接均已释放。挂载运行中读到 `partial`，第三组期间权限队列为空。私有网关、转发器及 RustCodeGraph 进程已退出。三份副本各 4109 个文件及符号链接与归档一致，临时挂载、私有凭据和 SQLite 副本已清理，原网关健康。此次无运行时代码变更，复用共享构建，未重跑 Cargo。
