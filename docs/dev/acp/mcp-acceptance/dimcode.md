# DimCode 动态挂载 RustCodeGraph

2026-11-19，验证固定版 DimCode 0.5.28 的 ACP 会话。宿主明确拒绝 stdio 挂载，原生 Streamable HTTP 和 SSE 路径均完成了三种真实只读工具调用。

## 环境与请求

- Darwin arm64 私有回环网关复用已验收的 ShiJu 0.6.0 构建，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。使用现有固定 npm 发行版的 `dim acp`，原生二进制 SHA-256 为 `0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0`。在隔离 HOME 单独握手读到 ACP v1、`loadSession=true`、`http=true` 和 `sse=true`。
- RueJS 使用 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 干净副本，以 RustCodeGraph 1.2.10 索引 3478 个文件。`createRueSiteApp` 位于 `app/app.tsx:32`。
- 请求复用 `189d22dc1` 中的提示，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。租户为 `10001 / 100010005`，使用 `acp-dimcode / dimcode`、`session_model`、`alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `full_access=true`。每轮通过同一 `curl` 请求另建会话。[复用请求](../ACP验收.md#可复用的验收请求)

## stdio 三轮

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efcd6c480016DgDGwaY6oWoZw` | 200 / 0 / 6.480 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efcd85de001f81asB1dXYxMSr` | 200 / 0 / 1.357 | `chat.error`，零工具事件 |
| 撤除 | `ses_1efcd8bb40012q8b2kKpnnFDxo` | 200 / 0 / 5.896 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

挂载轮收到一个 `chat.model_recovery`（`phase=stopped`）及一个 `chat.error`，原因为 `DimCode does not support ACP stdio MCP; select an HTTP or SSE server`。`validate_mcp_contract` 在发出 `session/new` 前拒绝该传输。UI summary 保留请求绑定和 `full_access=true`，options 无连接和选模结果。对照轮运行中选模回读匹配，权限为 `client_only`，终态释放连接。

## Streamable HTTP

采用仓库已有 Copilot 验收路径，在私有目录使用 Supergateway 4.2.2，将回环 Streamable HTTP 代理到同一个实际 RustCodeGraph stdio 进程，仅公开 query、callers 和 node。桥接只使用自己的短期 Bearer，匿名请求返回 HTTP 401。桥接进程使用独立 HOME，环境只包含 HOME 与 PATH。上游模型凭据用于网关，DimCode 使用现有请求私有模型中继。首次配置将 `headers` 写成对象，HTTP 400 发生在工作区配置解析，修正为 `name/value` 数组后发出后续请求。[Copilot 的桥接验收](copilot.md#stdio-拒绝与-http-路径) · [Supergateway 文档](https://github.com/supercorp-ai/supergateway/blob/main/README.md)

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1efcff0e00019YurDLNE21WcCG` | 200 / 0 / 5.733 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1efd007b3001UnHOwfmo4wLSdj` | 首段 200 / 28 / 300.003，续传 200 / 0 / 88.909 | 原请求及游标续传后收到一个 `chat.done`，无 `chat.error` |
| 撤除 | `ses_1efd5f8fe001zedYK6rNqserkm` | 200 / 0 / 5.405 | 桥接仍在线，`chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

三轮运行中回读 `acp-dimcode / dimcode`、`full_access=true`、`connected=true`、`support=client_only` 与指定模型，终态连接释放且模型绑定仍可回读。既有 `dimcode.rs` 和 `dimcode_access.rs` 分别配置会话私有模型及同步受宿主约束的权限档位。HTTP 挂载轮的 178 个 MCP 调用均有 `Permission allowed once` 事件。

| 工具 | 首轮代表调用 ID | 回执 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_f78dd42027374f9490c471d4` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_b70203339216406b9f75ffc1` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_247a8d7a71dd42f0a058ad23` | `packages/runtime/src/hooks/useApp.ts:4` 的 `useApp` 将根工厂传给 `_$createApp` |

首轮完成回执含 query 120 次、callers 2 次、node 56 次、原生 `grep` 18 次。桥接原始结果有 5 次 `isError=true`，其中 2 次尝试读取不存在的文件，3 次把子包路径当成单独索引项目。DimCode 将这 5 次报告为 `completed`，回执正文仍可识别错误。挂载闭环已证实，首轮违背了只使用目标 MCP 和无工具失败两项要求。

首轮答复的主调用链、浏览器与 SSR 分支、`#app` 缺失处理、插件安装顺序和 `/rust-layers` 的布局分支与源码相符。快照残留与提前就绪属于源码推断。答复给出的第二个场景只描述客户端入口写入内容，需用实际 `mountRueSiteApp` 路径才可检验清空逻辑。两项场景均未运行。

从 RueJS 副本目录启动桥接后重新挂载，会话 `ses_1efd619710012ExtVPzgHswn9n` 首段 HTTP 200、curl 28、300.010 秒。随后使用原请求标识及游标续传，HTTP 200，仍有目标工具结果。两段共完成 query 115 次、callers 2 次、node 84 次，桥接记录 15 次 `isError=true`。也出现了 `glob`、`skill` 与 `grep`。确认复跑已违反合同后关闭私有网关，续传以 curl 18 结束，未收到 `chat.done` 或 `chat.error`。该复跑未再发撤除轮，撤除证据来自首组已完成的 HTTP 三轮。

## SSE 复验

以同版本 Supergateway 的 SSE 传输桥接真实 RustCodeGraph，沿用回环绑定、Bearer 鉴权和三项工具的公开范围。首次夹具误启动了两套进程并共享私有目录，清理时相互影响，撤除轮报缺少模型凭据。该轮只用于诊断，复跑使用新目录、单一网关和桥接，原提示及模型保持一致。

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态及结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff2b107001xduPn7OyKWH94b` | 200 / 0 / 5.638 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1eff2c7b6001IjptjvhMfvdCQF` | 首段 200 / 28 / 300.009，续传 200 / 0 / 153.436 | 原请求和游标续传后收到一个 `chat.error` |
| 撤除 | `ses_1eff9b53e001JG615aoNIKd3XB` | 200 / 0 / 4.217 | 桥接仍在线，`chat.done`，`MCP_NOT_AVAILABLE`，零工具事件 |

三轮 UI summary 和 ACP options 核对同一绑定、`full_access=true`、指定会话模型和 `client_only` 权限状态。终态连接均释放。挂载轮两段工具完成事件包含 query 211 次、callers 11 次、node 81 次和原生 `exec` 一次。桥接报告 5 次 `isError=true`，均为模型猜测了不存在的文件，包含 `app/router.tsx`、`app/router.ts`、`app/i18n.ts` 和 `app/index.html`。观察到原生 `exec` 使用 `true` 并获逐次放行后，终止本轮私有 DimCode 进程。宿主以 `server shut down unexpectedly` 结束挂载流，未取得可验收的完整答复。撤除轮在同一私有网关另建会话。

| 工具 | 独立 SSE 复跑代表调用 ID | 回执 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_195dffb39fc144dc9c5998e7` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_c304ff961b8c4623b32d1a16` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_9ce390a95c514c40abef0c6a` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp` |

固定版 `dim acp` 的参数解析仅处理帮助、认证和已移除的远程模式。包内允许工具的命令行参数属于 headless 路径。宿主 `acp_allowed_tools` 则通过 `claudeCode.options.allowedTools` 传递。两者均没有在当前 DimCode ACP 中建立会话级原生工具白名单。桥接端已限制公开工具，原生工具和模型选择仍由 DimCode 的会话处理。工具失败在桥接侧已可复现，改变传输无法修正模型提交的无效文件路径。

## RustCodeGraph 1.2.11 复验

复用同一已核验的 ShiJu 和 DimCode 二进制、RueJS 提交、指定模型、租户、访问级别和原提示。以已通过验收的 RustCodeGraph 1.2.11 发行物重新索引两个私有副本，各 3478 个文件。发行物 SHA-256 为 `5fab2f2dbf506ddf71d842ef1a7cd6e744c5ec90ff3f5f043ea020e1b3696312`。两组均用已有 Supergateway 4.2.2 的受鉴权回环 Streamable HTTP 路径。第一组复用 Goose 的工具目录声明，第二组采用 Qoder 将已存在的入口路径放在说明前部的声明。两组都提供 `offset`、`limit` 及索引根提示。[Goose 目录说明](goose.md#共享构建复验) · [Qoder 直接声明](qoder.md#集成回归与-qoder-原生工具声明)

协议对照确认原始目录 8 项、限定目录 3 项。对两组分别核对查询 `createAppRouter`、查找 `createRueSiteApp` 调用者、分页读取 `app/router/index.ts:787–791` 的结果，转发与直连完全相同。限定目录拒绝额外工具和副本外路径，未改变实际工具结果。原生 DimCode 的 HTTP 连接确实发起 `tools/list`，桥接回执包含分页和索引根元数据。DeepAgents、Pi 的宿主策略只投影给受审的指定 stdio 入口。DimCode 的目标驱动没有对应策略消费入口，本轮沿用原审批并在桥接处限定 MCP 工具。[DeepAgents 策略](deepagents.md#最终集成回归与受管-v4) · [Pi 验收](pi.md#最终集成回归通过)

| 目录与轮次 | 新会话 | HTTP / curl / 秒 | 终态 |
| --- | --- | --- | --- |
| Goose，无挂载 | `ses_1f1102cde001saznpnHrhW3gaY` | 200 / 0 / 5.680 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| Goose，挂载 | `ses_1f1104374001zX2364bdzRT2LH` | 200 / 28 / 300.011，原游标续传 200 / 0 / 78.368 | `chat.done`，75 次 MCP 和 45 次原生 `grep` |
| Goose，撤除 | `ses_1f1160af7001U9PR5xMrZBJJeZ` | 200 / 0 / 4.564 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| Qoder，无挂载 | `ses_1f112a568001VqtD4N7kcTzPbC` | 200 / 0 / 5.243 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| Qoder，挂载 | `ses_1f112ba98001Q639Xb778rjo8h` | 200 / 0 / 210.939 | `chat.done`，76 次 MCP 和 2 次原生 `grep` |
| Qoder，撤除 | `ses_1f115f382001C2UEhxUnhOs1bU` | 200 / 0 / 5.185 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

六轮各有一个 `chat.done`，无 `chat.error` 或模型恢复。UI summary 与 ACP options 回读 DimCode 绑定、`full_access=true`、指定 `session_model`，连接期间为 `client_only`，终态均已释放。两组每个 MCP 调用均有开始、完成和同 ID 的 `Permission allowed once`，分别为 query/callers/node 19/3/53 和 27/2/47，桥接均无 `isError=true`。撤除轮发出时，桥接仍在监听。

| 工具 | 第二组代表调用 ID | 真实回执 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_46d41ada51984962b31c48e5` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_query` | `call_eb9c2b4f4094453a93d84e37` | `createAppRouter` 位于 `app/router/index.ts:787` |
| `rustcodegraph_callers` | `call_e7285741e9c24c8fa9fde2bf` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_b6c9f134fcfb42c493698ee3` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp` |

两次挂载的首个越界点均是插件安装溯源。第一组 `call_72af884d726742268f7f9eef` 直接对路由实现执行原生 `grep`，后续另有一次 `grep` 因单文件匹配上限报错。第二组 `call_42e131729ca84afdbdc03bc9` 在路由实现中查找 `attachRouter`，`call_0766a11131cf42e6b9401559` 又在站点目录查找 `data-rue-client-mode`。两次均有实际原生结果。新图谱和入口说明消除了此前复现的 MCP 路径失败，仍有两次原生工具选择反证。

两组答复均追到工厂，覆盖 SSR、缺少 `#app`、插件顺序、布局分支及快照机制。答复提出部署后首次挂载和预填模板快照的对照，均未实测。第二组片段将 `useApp` 称作 `createRue` 的别名，后文正确追到运行时的实际实现。第二个场景列出的 `snapshotClientRoute` 参数还缺必填的 `outDir`。原生 `grep` 事件独立使两组验收不通过。

本轮只提交验收记录。前轮的四项定向测试结论继续有效，未重复运行 Cargo。两个私有网关和桥接已退出，监听端口、挂载及授权副本已清理。两个 RueJS 副本各有 4109 个 Git blob 与源提交一致，原网关健康检查 HTTP 200。

## 私有补丁试验

在固定 DimCode 0.5.28 Darwin arm64 二进制的副本上试验。输入校验原发行摘要及唯一代码锚点，补丁保持 Bun 内嵌代码位置，私有副本重新签名并经 `codesign --verify --strict` 验证。原安装和用户网关继续使用原发行物。试验的逻辑包括以下两处限制。

- 创建每次 ACP prompt 的运行元数据时，取当前 `mcpRuntime.servers` 中的服务 ID。有挂载的回合只向模型声明元数据匹配这些服务的 MCP 工具。空挂载回合沿用原行为。
- 在处理模型工具调用时，执行前根据同一运行元数据检查工具定义中的 `mcpServerId`。超出范围的调用走原有拒绝结果流程，合规调用继续走原审批与执行。该试验没有调整 stdio 门禁和宿主权限。

回环模型记录了真实 ACP 请求中挂载后的工具目录。原版声明 17 项，仅声明补丁将其缩到三项，但模型仍可点名未声明的原生 `exec` 并触发审批。加入执行前检查后，同样复现返回 `Outside MCP scope` 且没有审批回调。正常 MCP 调用选择允许一次后，模型收到真实图谱输出 `app/app.tsx:32`；取消审批后，模型收到 `Permission request cancelled`。同一 ACP 连接再建空挂载会话，模型目录不再包含这三个 MCP 工具。最终私有二进制 SHA-256 为 `aada0dcd9678e2c32d7733cf760e2737cb39b0100a68aade9330b45e25f26421`。

仅声明补丁的三轮会话依次为 `ses_1f157f402001V8p3W1qvdF36Fk`、`ses_1f1580bff001qRbalESo3Qtn4J`、`ses_1f15ec44d001dfrYQLnltqAVkT`。三轮 HTTP 200、各一个 `chat.done`，前后仅有 `MCP_NOT_AVAILABLE`。挂载轮首段 curl 28，300.002 秒，沿同一请求游标续传 curl 0，140.037 秒。157 次目标调用获原审批放行，零额外工具。答复覆盖调用链及快照推断，但哨兵场景仅预填一个 `p` 节点却预测出现两份完整外壳，无法据此通过答复验收。该版也没有通过回环的未声明工具执行隔离检查。

加入执行前检查后，原目录的三轮会话依次为 `ses_1f15c44fc001EkQjLJuET40wx8`、`ses_1f15c62600010cBYfk2zC6WYEA`、`ses_1f162731d001WYjgO4oiM0p3RW`。均为 HTTP 200 和单一 `chat.done`。挂载首段 curl 28、300.005 秒，续传 curl 0、97.336 秒。挂载轮完成 119 次目标调用、无原生工具，但模型将 `app` 及一个 i18n 文件路径当作索引根，收到两次未初始化错误。撤除轮三种 MCP 均不可用，原生 `exec true` 仍获原审批并执行，答复也有额外文字。这与空挂载保持原工具目录的设计一致，该组三轮未通过。

最后一组保留执行前检查和真实 RustCodeGraph 1.2.11，在绑定到同一索引根的三项目录中省略可选的 `projectPath`，保留文件分页，工具参数及实际结果继续原样转发。协议对照仍为三个相同结果、额外工具及越界路径均拒绝。提示、模型、租户及现有审批均沿用。

| 轮次 | 新会话 | HTTP / curl / 秒 | 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f160f873001figWQE81jkN9cT` | 200 / 0 / 6.299 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f16111c7001ePWN5xltxIeGMI` | 200 / 28 / 300.003，原游标续传 200 / 0 / 149.582 | `chat.done`，119 次目标调用，零额外工具及失败 |
| 撤除 | `ses_1f167efe1001xFoHTNdb4AHZdc` | 200 / 0 / 5.251 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |

119 次调用按同一 ID 核对开始、一次审批、完成和结果。query 48 次，例如 `call_ba5f78df6baf4bdf912c9495` 命中 `app/app.tsx:32`。callers 5 次，例如 `call_f27edc54c10444e293a1c24b` 命中同文件第 39 行。node 66 次，例如 `call_44c1216931a04f73bf0ac518` 读回该文件的启动和挂载源码。三轮无模型恢复或 `chat.error`，均核对绑定为 `acp-dimcode / dimcode`、`full_access=true`、指定会话模型、连接期间为 `client_only`，结束连接释放。撤除轮桥接仍在线。

答复追到了 `_$createApp`，覆盖 SSR、缺少 `#app`、路由与 i18n 的安装顺序、布局条件、快照残留及验证预期。第二项使用已注册的 `/design/button` 检验实际布局是否重复。第一项用预填陈旧节点检验快照，但夹具入口只描述渲染 `main`，需要明确让它经过待改动的挂载逻辑，才能隔离 `replaceChildren()` 的效果。两项均为源码推断，未执行场景。最后一组验证了私有补丁的工具与审批闭环，生产安装、补丁校验接线和严格答复验收仍有待完成。

三组私有服务及监听端口已清理，授权副本和临时挂载已移除，各 RueJS 副本的 4109 个 Git blob 与原提交相符。原安装摘要与原网关状态一致。保留私有补丁脚本、回环断言及原始证据，本轮提交的仓库变更为验收记录。

## 受管安装接入

Darwin arm64 安装器从固定 npm 归档取出 DimCode 0.5.28 的原生程序，依次核对归档和补丁输入、补丁编译产物的位置及唯一锚点、重新签名并校验最终二进制。受管入口位于配置目录的 `acp-runtime/dimcode-0.5.28-model-source-v1/dim`。安装使用既有临时目录和原子发布。挂载仅支持受管 `session_model` 的 HTTP/SSE，宿主在启动前核验摘要，在 ACP 建会话、加载与恢复前再次校验。stdio、关闭工具或拒绝全部权限时明确拒绝。配置了 MCP 工具级允许或拒绝列表时也报错，避免原生 DimCode 忽略宿主策略。空挂载仍可沿用原版入口。

独立运行仓库安装脚本，固定归档下载、二进制补丁、签名校验和 `dim --version` 均通过。受管产物 SHA-256 为 `89a2f42669235390c13036f3fd63f6c454381bb8578b2f06f5fe1d659e2b170b`。用该产物重跑回环模型，挂载目录准确为三项；原审批的允许一次返回 `app/app.tsx:32`，取消产生 `Permission request cancelled`；未声明原生 `exec` 在审批前被拒绝；撤除后的模型目录无目标 MCP。桥接监听和短期密钥已清理。指定阿里模型的三轮仍以先前私有补丁证据为准。

## 受管网关复验

复用包含 `c40f0622f` 安装摘要与启动门禁的共享槽位 6 网关产物。私有副本 SHA-256 为 `109066ea0b79ce6edfc4399624715f0dafa80d397adabe7a1ae46158407e7cba`。验收请求使用当前文档格式，仅以 `agent=acp-dimcode` 选择后端，原提示摘要、模型、租户与访问级别相同。原始证据保存在本机私有目录。RustCodeGraph 1.2.11 仍使用三工具目录和固定索引根，直连对照确认三项实际结果相同，额外工具及越界路径被拒绝。

私有网关的 `POST /v1/desktop/acp-install/dimcode` 冷安装 HTTP 200、16.270 秒，重复安装 HTTP 200、0.063 秒。安装的 DimCode 0.5.28 摘要与受管清单一致，签名验证通过，进程可执行路径确认为私有配置下的受管入口。独立改动发行二进制一个字节后，补丁脚本在生成输出前拒绝了输入。

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1f18df59e001NNqGzV94iN1vj5` | 200 / 0 / 7.594 | `chat.done`，仅 `MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1f18e149b001NuEV0HzLyedGLQ` | 首段 200 / 28 / 300.004，原游标续传 200 / 0 / 221.638 | `chat.done`，161 次目标调用全部成功并回传 |
| 撤除 | `ses_1f1960dad001TcVafwIuZls6D1` | 200 / 0 / 9.568 | `chat.done`，桥接仍在线，仅 `MCP_NOT_AVAILABLE`，零工具 |

三轮均无 `chat.error` 或模型恢复事件。UI summary、ACP options 核对绑定、`full_access=true`、指定会话模型和连接期间的 `client_only`，终态均释放连接。挂载的 query 80 次、callers 7 次、node 74 次，各有同 ID 开始、一次放行、完成和实际结果。当前标准允许官方工具按原权限共存，同时要求挂载工具真实执行和回传。本次挂载仅调用目标 MCP。

| 工具 | 调用 ID | 实际回传 |
| --- | --- | --- |
| `rustcodegraph_query` | `call_e8ca9018c5e24d14b9bb181e` | `createRueSiteApp` 位于 `app/app.tsx:32` |
| `rustcodegraph_callers` | `call_160bdbaad9fb4a2dbb33f1a8` | `mountRueSiteApp` 位于 `app/app.tsx:39` |
| `rustcodegraph_node` | `call_a250c0c5cba44fc5b0e909aa` | `packages/runtime/src/hooks/useApp.ts:4` 调用 `_$createApp` |

答复引用完整调用链、SSR 和浏览器分支、缺少容器、路由与 i18n 安装和布局条件，并指出 `/rust-layers` 当前未注册。以源码推断快照残留和重复挂载，使用已注册且配置客户端模式的 `/settings` 设计重建快照对照，并提出静态预览与导航时检查旧节点。两项浏览器及快照场景未执行。

复用共享槽位 9 本轮编译的 `acp-client` libtest 私有副本，SHA-256 为 `9cca32b073f5fafdc0c3ddd0f1dad2240483b283bd81ab096011e655f8a5b2ce`。以 `dimcode --test-threads=1` 执行，6 项通过、零失败，包含未校验入口不得发出建会话请求及五项权限测试。未另起重复编译，沿用前轮四项 onboarding 结果。

私有网关、桥接、监听端口、短期凭据与临时挂载均已清理。RueJS 副本 4109 个 Git blob 与原提交一致，原网关健康检查 HTTP 200。

## 结论与验证

受管 DimCode 0.5.28 通过 Streamable HTTP 的正式三轮验收。保留 stdio 拒绝、现有审批行为及测试断言。此前原版的 HTTP、SSE 及私有补丁结果作为对照保留。

定向测试复跑使用槽位 3，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-3`。首次中止退出 130、零测试。恢复后原 stdio 过滤命令退出 0，1 项通过。原拒写夹具缺少权限选项，导致 `acp_onboarding_dimcode_permission_denial` 在建会话时提前失败，退出 101，1 项失败。夹具补齐只读权限确认后，审批拒写、stdio 拒绝和会话生命周期均通过。注册断言原先固定要求 `npx`，现按注册表契约同时验证已安装的 `dim acp` 与固定版本回退入口。最终 `python3 scripts/cargo_slot.py cargo test -j 1 -p vw-acp --test onboarding_dimcode acp_onboarding_dimcode_` 退出 0，4 项通过。生产权限、传输门禁与原拒写断言均未修改。

私有网关、桥接和监听端口已清理。已删除私有网关上游授权副本、一次性桥接密钥及临时 MCP 配置。RueJS 副本 4109 个 Git blob 与源提交一致，原工作区和用户全局配置未写入。原网关健康检查仍为 HTTP 200。原始日志保存在本机 0700 目录。
