# Claude Code 动态挂载 RustCodeGraph

## 请求与环境

2026-11-19，在 Darwin arm64 使用 [可复用的验收请求](../ACP验收.md#可复用的验收请求) 的 `189d22dc1` 版本。提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。使用租户 `10001 / 100010005`、`acp-claude / claude`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。

使用已安装的 `@agentclientprotocol/claude-agent-acp` 0.26.0 与 Claude Agent SDK 0.2.96（Claude Code 2.1.96）。独立握手返回 ACP v1，声明支持 HTTP 与 SSE MCP，以及加载、恢复和关闭会话。实际验收使用 `session/new.params.mcpServers` 传入 `ruejs_graph_probe`，采用原 RustCodeGraph 1.2.10 的 stdio 和 `--no-watch`。Claude 受管状态目录独立于用户 Claude 配置，私有网关通过宿主保存阿里凭据，适配器使用请求级回环中继。

RueJS 位于 `main`，基准提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c`。最初副本保留工作区实况，索引 3687 个文件。工作区已有删改，包括路由文件移动。后续从同一提交导出干净副本，索引 3478 个文件。两份均命中 `createRueSiteApp`（`app/app.tsx:32`）。

## 修复前

使用现有回环网关，构建 SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`。该网关已启用内置 Claude 预设。三轮 UI summary 回读均确认绑定、`full_access=true` 及调用数 0，ACP options 回读确认所选模型和来源。

| 轮次 | 新会话 | HTTP / curl | 终态 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef7ca430001mOG410l3QO5xTz` | 200 / 0，13.857 秒 | `chat.error` |
| 挂载 | `ses_1ef7cdbfb001P9QG47g5ZkoL51` | 200 / 0，14.318 秒 | `chat.error` |
| 撤除 | `ses_1ef7d1583001Py08jF4YOVDzWN` | 200 / 0，13.328 秒 | `chat.error` |

三轮均无 `chat.done`、模型答复和工具事件。首次失败边界为 `session/prompt` 发起的第一个 Messages 推理请求。中继返回 HTTP 422，报 `Unsupported Messages capability or missing thinking/tool replay context`。挂载轮的原 stdio RustCodeGraph 已启动。使用同一旧构建在私有网关配置指定租户与模型，经租户鉴权 API 写入凭据后，干净副本的无挂载轮也复现同一 422。

## 定位与修复

在私有回环捕获到 SDK 发送 `thinking.type=adaptive` 和 `context_management`。Messages 到 Chat 的严格输入校验拒绝后一个字段。Claude 受管启动现在只在 Messages 转换路由设置 `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`。同一适配器回放确认 `context_management` 消失，`adaptive` 思考和 MCP 工具声明仍在。原生 Messages 路由沿用自身协议。模型绑定、上游凭据边界、审批和中继校验沿用原路径。

回环模型按顺序发出三个指定工具，真实 RustCodeGraph 退回 `query → createRueSiteApp`（`app/app.tsx:32`）、`callers → mountRueSiteApp`（`app/app.tsx:39`）、`node → useApp`（`packages/runtime/src/hooks/useApp.ts:4`，代理到 `_$createApp`）。适配器逐次发送宿主审批请求。探针逐次选择相应的 `allow_once`，收到三个成功结果并在续轮 Messages 请求中核对同一工具 ID 的 `tool_result`。独立无挂载进程发出的模型请求包含零个目标工具，返回固定标记。两种回环会话均以 `end_turn` 结束。单独将三个审批都选择为 `reject_once` 时，对应的三个续轮结果均为 `is_error=true`。

定向命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_model_source_claude`。使用槽位 3，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为仓库的 `target/rust-slot-3`。修复前 10 通过、1 失败，退出 101，失败断言为 Alibaba 转换路由缺少兼容设置。修复后 11 通过，退出 0。公共 ACP 传参实现无行为变更，验收使用原生适配器与实际 MCP。

## 修复后真实模型验收

定向构建 `cargo build -j 1 --locked -p vw-cli --bin shiju` 在同一槽位成功，退出 0。私有网关使用该构建，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。三轮均使用干净 RueJS 副本和原提示。各会话运行中 `connected=true`，权限为 `access_status.support=native`、`requested=true`、版本 0.26.0，仍受后端和组织策略约束。UI summary 确认 `full_access=true` 与 `acp-claude / claude` 绑定。ACP options 在运行中及结束后均确认完整指定模型与 `session_model`，结束后 `connected=false`。

| 轮次 | 新会话 | HTTP / curl | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1ef97809b001XaX7ACjMeURK8R` | 200 / 0，4.825 秒 | `chat.done`，去除 SSE 的 `<think>` 段后答复恰为 `MCP_NOT_AVAILABLE`，零工具事件 |
| 挂载 | `ses_1ef9794ee0013AyWgGjyWLS2Z0` | 200 / 0，133.225 秒 | `chat.done`，三种指定工具成功，但触发额外工具 |
| 撤除 | `ses_1ef999ead001R3q0MnzOjTSlG2` | 200 / 0，4.965 秒 | `chat.done`，去除 SSE 的 `<think>` 段后答复恰为 `MCP_NOT_AVAILABLE`，零工具事件 |

三轮均无 `chat.error`、模型恢复或中继错误。挂载轮有 304 条工具状态事件，76 个调用均有开始、完成与输出，全部来自 `ruejs_graph_probe`。`query` 完成 41 次，首次返回 `createRueSiteApp`（`app/app.tsx:32`）；`callers` 完成 7 次，其中一次返回 `mountRueSiteApp`（`app/app.tsx:39`）；`node` 完成 14 次，回读站点入口、`useApp`（`packages/runtime/src/hooks/useApp.ts:4`）和运行时工厂 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`）。首次违反合同发生在模型调用 `rustcodegraph_status` 时（`call_1ababb49563f43ba9eb39814`）。额外工具共 14 次，包括 `status`、`explore`、`callees`、`files` 和 `impact`。未观察到 shell 或写入工具。

答复正确追踪浏览器入口到运行时工厂、SSR 分支、缺少 `#app` 时的异常、路由及 i18n 的安装时机，以及 `/rust-layers` 绕过布局。答复根据运行时追加节点的源码推断静态快照残留，并提出首屏唯一性与卸载后重新挂载两项预期，未实际执行浏览器验证。答复将 `node` 返回前 120 行推断为索引不完整，工具输出只证明展示范围有限。答复还把传给 `useApp` 的 `RootApp` 当成 `createRueSiteApp` 的直接调用，源码表明根工厂在应用挂载时执行。

Claude 动态挂载、调用、结果续轮及撤除隔离已有真实证据。完整合同因模型多调用工具而未通过，保留任务。私有网关退出，两个私有端口关闭，目标 RustCodeGraph 子进程退出，复用槽位释放。私有 Claude 状态和网关凭据副本已清理，SSE 与会话回执留在本机私有目录。

## 当前共享构建复跑

使用与 Snow 验收相同的共享构建，SHA-256 为 `e755e458b4fce6ed045ef10a7c077a0d7dfb98d69637442d9c4449d0d50923ac`。复制已核验产物并启动私有网关。继续使用 `189d22dc1` 的原请求、指定模型和 Claude 0.26.0 的原生 stdio MCP。此次没有额外修改产品代码或运行 Cargo，已有 11 项 Claude 测试仍是实现的定向证据。

| 原生完整工具轮次 | 新会话 | HTTP / curl | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff05d98001u2Y3vtbV0mLHo6` | 200 / 0，4.207 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1eff06ece001RWMd0ZJ3UXjXUP` | 200 / 0，151.408 秒 | `chat.done`，95 次调用，15 次额外调用，2 次 `node` 失败 |
| 撤除 | `ses_1eff2becd001h0bpf0IRpu703j` | 200 / 0，0.318 秒 | `chat.error`，宿主拒绝与会话工作区不符的 ACP cwd，未进行模型或工具调用 |

另在独立的同提交 RueJS 副本中，通过项目级 `.claude/settings.json` 的 `permissions.deny` 隐藏额外工具和内置工具。回环先确认模型请求只声明 `query`、`callers` 和 `node`。私有配置在三轮保持一致，挂载轮仍通过宿主临时配置启动原 RustCodeGraph stdio 二进制。原提示、模型和 `full_access=true` 保持一致，运行中读回 `connected=true`、`access_status.support=native` 与指定选模。

| 工具限制轮次 | 新会话 | HTTP / curl | 终态与结果 |
| --- | --- | --- | --- |
| 无挂载 | `ses_1eff179fe00127rYd0PESDjp5G` | 200 / 0，5.842 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |
| 挂载 | `ses_1eff19131001ktwceW7nShPv9L` | 200 / 0，117.337 秒 | `chat.done`，只调用三种指定工具，4 次错误路径读取 |
| 撤除 | `ses_1eff35be4001o0pJCZdWhe3T6Y` | 200 / 0，4.589 秒 | `chat.done`，`MCP_NOT_AVAILABLE`，零工具 |

限制工具后的挂载轮有 304 条工具事件，49 次 `query`、4 次 `callers`、20 次 `node` 完成，另有 4 次 `node` 失败。首个失败参数为 `{"file":"app/router.tsx"}`，返回文件不存在。其余失败参数为 `app/i18n.tsx`、`app/router.ts` 和 `app/router/index.tsx`。三种指定工具仍回读了 `createRueSiteApp`、`mountRueSiteApp`、`useApp` 和 `_$createApp`。答复覆盖所需链路和分支，并提出首屏去重与导航后无快照残留两项回归预期。答复明确未能通过 MCP 查证路由工厂默认历史值。宿主独立核对显示，`app/router/index.ts:787` 的工厂存在，RustCodeGraph 1.2.10 在同一索引上查询 `createAppRouter` 仍返回空。

限制工具解决了额外调用，指定模型仍在真实挂载轮发出错误路径，因此完整合同继续阻塞。通过条件是在原合同下模型以现有工具准确读取源文件，或先获得能覆盖上述路由符号的 RustCodeGraph 索引，再复跑三轮。私有权限配置、租户凭据和网关均已清理。
