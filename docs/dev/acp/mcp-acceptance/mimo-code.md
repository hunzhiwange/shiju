# MiMo Code 动态挂载 RustCodeGraph

2026-11-19，验收阻塞于受管 MiMo Code 启动 stdio 服务。宿主成功连接 RustCodeGraph，原生 ACP 执行 `posix_spawn` 时收到 `EPERM`。

## 环境

- Darwin arm64，受管 MiMo Code 0.1.17。复用固定源码及 stdio 补丁构建的安装产物，二进制 SHA-256 为 `c086c1bd5c4ada962880f7636de5b07fc442db19edccf3e383887f6f3c47f7ae`。私有网关报告已安装。独立握手返回 ACP v1，代理自报 `OpenCode` 0.1.17，声明 HTTP、SSE 和会话 load、fork、list、resume。
- 私有回环网关复用 shiju 0.6.0，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。仅在私有实例启用预设，使用租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 与 `session_model`。
- [复用请求](../ACP验收.md#可复用的验收请求) 取自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 私有副本来自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。RustCodeGraph 1.2.10 索引 3478 个文件并命中 `app/app.tsx:32` 的 `createRueSiteApp`。挂载使用 `ruejs_graph_probe` 原生 stdio，关闭文件监听。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态和答复 | 原生工具调用 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efb70598001a1yENBpMDVj4Nq` | 200 / 0 / 8.201515 | `chat.done`，标记及额外说明 | 2 次 `mcp_tool_search` |
| 挂载 | `ses_1efb72755001zrtE5s96hA0iDN` | 200 / 0 / 9.061355 | `chat.done`，标记及额外说明 | 2 次 `mcp_tool_search` |
| 撤除 | `ses_1efb74bed001zBsSMF3rCk45Id` | 200 / 0 / 6.937092 | `chat.done`，标记及前置说明 | 1 次 `mcp_tool_search` |

各轮一条 `chat.done`、零 `chat.error`。三轮均输出 `MCP_NOT_AVAILABLE`，去掉推理标签后仍有额外文字。按调用 ID 去重，SSE 分别记录 2、2、1 次原生工具调用，合计 16、16、8 条更新，完成标题均为 `No deferred MCP tools available`。三种目标工具 `rustcodegraph_query`、`rustcodegraph_callers`、`rustcodegraph_node` 在各轮均零调用、零结果。挂载轮未回答调用链、行号和静态快照验证，两个对照也未达成仅输出标记且零工具事件的要求。

三轮运行中及终态的 UI summary 回读均为 `acp-mimo-code / mimo-code`、`full_access=true`，ACP options 均回读指定提供商、模型及 `session_model`。运行中 `connected=true`、`access_status.support=client_only`，终态连接释放。此次只有目录检索，未触发写入或终端审批。标题生成另有不同模型的 HTTP 403。

## 失败边界与收尾

挂载轮宿主成功完成 stdio MCP 初始化。独立探针复用同一原生入口、`--pure`、私有 `MIMOCODE_CONFIG` 和受管沙箱的进程及网络约束，将同一服务定义传入 `session/new`。MiMo 日志记录该服务为 `local`，尝试启动 RustCodeGraph 时失败，随后仍返回会话 ID。用私有回环模型核对，挂载和空目录对照收到的主提示工具定义相同，均为 `read`、`edit`、`write`、`mcp_tool_search` 和 `mcp_tool_use`，两轮均以 `end_turn` 结束。回环探针只用于定位工具发现边界。

现有 `MIMOCODE_CONFIG` 继续使用私有模型配置，项目配置和外部兼容配置入口继续隔离。受管沙箱只允许执行 MiMo 入口，出站限定本次模型中继端口。MiMo 声明远程 MCP 能力，但已装 RustCodeGraph 只提供 stdio，仓库的 stdio 回环桥属于测试夹具，现有中继只支持模型协议。固定版补丁仅将 MiMo 内部 SDK 请求留在进程内。复跑需要可保留会话、租户、审批、撤销和进程生命周期的宿主兼容入口。

生产代码、安装契约和权限策略未修改。生命周期夹具未运行，原生探针已定位到真实进程启动失败。本次未运行 Cargo。私有网关和原生探针均退出，端口关闭，挂载已撤除，私有配置、凭据和数据库副本已清理。原 RueJS 工作区未写入，现有用户网关的 MiMo 预设仍停用。原始 SSE 和诊断保存在本机私有目录。

## 共享工作区复验

复用较新的共享构建 shiju 0.6.0，SHA-256 为 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`。隔离网关使用同一个受审 MiMo Code 安装产物、RueJS 提交、指定租户和模型。三轮请求体与前轮相同，仅更新私有 cwd 与回环端口。

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态与实际调用 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f0418d0500100HJnD3wlVLfTf` | 200 / 0 / 7.627464 | `chat.done`，2 次原生工具搜索 |
| 挂载 | `ses_1f041abdb001cG6VZoia4sjVpX` | 200 / 0 / 6.550112 | `chat.done`，1 次原生工具搜索 |
| 撤除 | `ses_1f041c6380016DiY8OyV9OsY32` | 200 / 0 / 8.075596 | `chat.done`，2 次原生工具搜索 |

三轮均无 `chat.error`，均返回 `MCP_NOT_AVAILABLE` 及额外文字。没有目标工具调用或结果。UI summary 和 ACP options 回读原后端、`full_access=true` 及指定模型来源，终态连接已释放。

### 同端口传输和权限诊断

用同一原生程序、`--pure` 与原有进程和出站沙箱约束，在模型端口提供私有 Streamable HTTP 诊断路由。路由通过单独的诊断令牌将请求转给宿主侧真实 RustCodeGraph，只发布三种目标工具。其他端口对照没有 MCP 请求抵达，空挂载对照没有工具声明或调用。

- 保留生产工具白名单与权限时，MiMo 完成 `initialize`、`notifications/initialized`、`tools/list`，日志确认连接及 3 个目录项。回环模型未收到这些工具定义。仅为这三个工具配置询问权限时，仍未发布定义。发行源码在发布工具前同时检查运行权限和代理精确工具白名单。
- 在诊断私有配置中仅把目录中这三个完整工具名加入白名单，逐个配置为询问，并使模型直接收到工具定义。三个 `session/request_permission` 选择 `allow_once` 后，各收到一次真实 MCP 调用。相同的第一个请求返回取消时，服务收到零次 `tools/call`。诊断前后均无待审批写入请求。

| 工具 | 真实结果 | 回环模型续轮调用 ID |
| --- | --- | --- |
| `rustcodegraph_query` | `createRueSiteApp` 位于 `app/app.tsx:32` | `mimo_probe_0` |
| `rustcodegraph_callers` | `mountRueSiteApp` 位于 `app/app.tsx:39` | `mimo_probe_1` |
| `rustcodegraph_node` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp(factory)` | `mimo_probe_2` |

复核共享工作区新增路径。Cortex 的客户端工具扩展及 DeepAgents、OpenClaw 的受管工具入口依赖各自原生协议。当前通用模型中继没有足以提供会话级 MCP 的路由。公开 npm 最新版本仍为 0.1.17，现有已审入口已是该版本。因此纯 MiMo 配置或升级无法完成端到端验收。剩余依赖为经审核的宿主侧会话级 MCP 入口，以及依据会话目录投影 MiMo 工具白名单和审批规则。需要协调公共中继接入与责任归属，再使用真实网关复跑撤除、取消、策略和原模型三轮。

诊断使用真实 RustCodeGraph 及回环模型，真实指定模型三轮单独记录。此次未改生产代码或安装契约，未运行 Cargo。诊断监听、子进程与隔离网关均已退出，私有配置、凭据和数据库副本已清理。

## 同端口转发原型与真实模型

原型将私有模型地址与 Streamable HTTP MCP 路由置于同一监听端口。MiMo 仍运行在原有 Seatbelt 进程、文件和单端口约束内，宿主侧 RustCodeGraph 使用原生 stdio。模型路由只转发固定的阿里模型，拒绝重定向，模型和 MCP 使用不同的本地令牌。阿里凭据只由私有中继读取。MCP 路由仅注册和转发三种图谱工具，由原生 `session/request_permission` 逐次审批。原型直接驱动受审 MiMo ACP，未接入 ShiJu 的生产请求路径。

原请求和模型保持不变。RustCodeGraph 1.2.10 的原型先保留原生文件工具，再改用只声明三种图谱工具的私有配置。两组无挂载和撤除均返回 `end_turn` 和准确的 `MCP_NOT_AVAILABLE`，没有工具调用。两组挂载均返回 `end_turn`，三种真实结果均在后续模型请求中出现。第一组收到 65 次获批 MCP 调用，其中 61 次成功，4 次工具错误，并出现其他原生工具尝试。第二组收到 89 次获批 MCP 调用，其中 87 次成功，2 次工具错误，且模型仍尝试使用未声明的内部工具。第一组和第二组的挂载进程在原生回复终态后均超过私有退出等待时间，由探针发送 SIGTERM 回收。

第二组答复追踪了 `mountRueSiteApp → createRueSiteApp → useApp → _$createApp`，并引用浏览器守卫、SSR 入口、`#app` 保护、插件安装及布局条件。答复按源码推断移除 `replaceChildren()` 会让旧快照与客户端 DOM 并存，也提出了两项验证设想。答复将 `/rust-layers` 注册位置作为假设，当前路由表未找到该路径。验证设想未实际运行。

继续用摘要为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312` 的 RustCodeGraph 1.2.11 重建 3478 文件索引，复用 OpenCode 的窄目录说明，保持原提示、模型与工具执行语义。三轮均原生 `end_turn` 且进程退出 0。无挂载只出现 `MCP_NOT_AVAILABLE` 和额外正文，撤除只有标记，两轮均无工具调用。挂载收到 54 次获批图谱调用，53 次成功。唯一的图谱错误将源码文件当作 `projectPath`。模型续轮分别收到 query、callers、node 的真实输出。原生记录还出现未声明的文件工具尝试，均失败。答复覆盖调用链、SSR、容器缺失、路由和 i18n、布局条件及两项快照验证设想，且将当前路由表中的布局入口标为尚未确认。

核对三份 RueJS 副本各 4109 项归档条目，排除索引后均无差异，探针及其监听全部退出。三组已证明同端口方案能将三种工具交给指定真实模型，也保留了撤除隔离。本轮未达严格模型合同，且尚缺生产会话绑定、撤销与隔离网关回归。未修改受管二进制、沙箱、生产 Rust 代码或租户配置，未运行 Cargo。
