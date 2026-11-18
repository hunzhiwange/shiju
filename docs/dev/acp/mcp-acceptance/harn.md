# Harn 动态挂载 RustCodeGraph

2026-11-19，**已阻塞**。Harn 0.10.201 在 `session/new` 拒绝非空的 `mcpServers`。原样挂载轮未进入选模及推理。无挂载和撤除两轮均完成，并返回约定的缺失标记。[复用请求](../ACP验收.md#可复用的验收请求)

## 环境

- Darwin arm64 使用已安装的受管 Harn。原生入口自报 `harn 0.10.201`，SHA-256 为 `23c951212c045c56deea491f9ce9505b8298472daa862334b33413ea31dcdcca`。原生入口、launcher 和 pipeline 的摘要均匹配仓库受管清单。网关使用已有 shiju 构建的私有副本，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`，回环健康检查通过。
- 私有网关启用 `acp-harn`，使用原租户 `10001 / 100010005`、原模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`、`session_model` 和 `full_access=true`。提示取自 `189d22dc1`，SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 使用 `main` 提交 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 的导出副本，索引 3478 个文件。挂载轮配置 `ruejs_graph_probe`，使用原 RustCodeGraph stdio 入口和 `--no-watch`。

## 真实模型三轮

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efa9f6f2001ouaWW7Ov6bpQ57` | 200 / 0 / 10.311 秒 | `chat.done` | `MCP_NOT_AVAILABLE`，工具事件 0 |
| 挂载 | `ses_1efaa1ff8001IWnlxlvaq68QHb` | 200 / 0 / 5.282 秒 | `chat.error` | 建会话拒绝 `this agent does not support session mcpServers intake`，工具事件 0 |
| 撤除 | `ses_1efaa35d0001Fnvz0GfF4r8qn3` | 200 / 0 / 9.620 秒 | `chat.done` | `MCP_NOT_AVAILABLE`，工具事件 0 |

三轮终态 UI summary 均确认绑定 `acp-harn / harn` 和 `full_access=true`。两个对照轮运行中 ACP options 的 `connected=true`、权限状态 `client_only`、`selected_model` 均与预期一致。连接在终态关闭，仍可回读原模型绑定。挂载轮始终未连接，`selected_model` 为空。因此缺少挂载模型答复、真实模型工具事件和准确性评分。

## 首个失败边界与替代路径

- 独立向同一受管入口发送 ACP v1 `initialize`。返回 Harn 0.10.201、`loadSession=true`，声明 `resume`，HTTP/SSE 均为 false。原生 `session/new` 接受空列表，带真实 stdio 定义时返回 JSON-RPC `-32602`，错误与网关一致。该拒绝发生在创建原生会话之前。固定版本和复查时的上游主线文档均明确保留拒绝，未找到可采用的原生升级入口。[固定版本说明](https://github.com/burin-labs/harn/blob/v0.10.201/docs/src/harn-serve.md#acp) [上游主线](https://github.com/burin-labs/harn/blob/main/docs/src/mcp-and-acp.md#optional-protocol-boundaries)
- 受管驱动使用私有 provider overlay 和经摘要校验的自然语言 pipeline，拒绝项目 `harn.toml` 和 `.env` 覆写。现有 launcher 对中继凭据只授予 `in_process`，并把 `mcpServers` 原样交给原生引擎。项目级 MCP 配置会改变挂载作用域。另以隔离的原生 ACP 探针测试 Harn 自有的 `project.mcp_config` 宿主回调。私有探针的真 stdio 进程完成握手，测试中继变量从子进程环境中缺席。首个发现调用 `harness.tools.mcp_list_tools` 以 `tool_policy_rejected` 失败，具体为 `exceeds the active effect ceiling: tool:mcp:read`。保留现有 ACP 权限策略。实际模型轮在此之前已经被拒绝，未发生真实中继凭据传递给 MCP 的情况。[Harn 宿主 MCP 加载实现](https://github.com/burin-labs/harn/blob/v0.10.201/crates/harn-serve/src/adapters/acp/execute.rs)
- 直接通过独立 MCP 客户端启动同一 RustCodeGraph stdio 服务，三次真实调用均成功。`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 返回 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp` 转发给 `_$createApp` 的定义（`packages/runtime/src/hooks/useApp.ts:4`）。这些是工具独立对照结果。
- 使用摘要已核验的原生二进制运行 `patches/acp-model-source/harn/host_bridge_test.py` 中的 `HostBoundary.test_native_write_requires_host_authorization`，1 项通过。宿主拒绝写入后无目标文件。生产驱动、受管补丁和公共转发未改，未运行 Cargo 或桌面构建。

## 清理与复跑条件

私有网关退出 0，端口关闭，无本次代理或图谱子进程残留。撤除后核对 4108 个源码文件摘要均一致，额外文件仅在私有图谱索引和 Harn 运行缓存目录。已清理私有源码、网关副本和授权副本，原 RueJS 与用户网关配置未改。原始 SSE、诊断与回读在本机 0700 证据目录。

取得支持会话级授权、工具发现及隔离撤除的受审入口后，用原模型和同一请求补跑三轮。

## 共享工作区复验

复用共享工作区较新的 shiju 0.6.0（SHA-256 `6954a2bb76ca26aa39c3b0dd909786c68b85f336648526ef39651b1e3a31f718`），重新在隔离网关跑固定请求。受管 Harn 仍为 0.10.201。

| 轮次 | 新会话 | HTTP / curl / 耗时 | SSE 终态 | 答复与工具 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1f035773f001EE83bN0lYT1vlh` | 200 / 0 / 9.687 秒 | `chat.done` | `MCP_NOT_AVAILABLE`，工具事件 0 |
| 挂载 | `ses_1f0359e23001kseAD12zcvbIhf` | 200 / 0 / 3.920 秒 | `chat.error` | 原生 `session/new` 拒绝，工具事件 0 |
| 撤除 | `ses_1f035af24001GjkjZWjmyrQjuB` | 200 / 0 / 7.599 秒 | `chat.done` | `MCP_NOT_AVAILABLE`，工具事件 0 |

三个 UI summary 均回读原预设、后端及 `full_access=true`。两轮对照执行中确认 `connected=true`、`client_only` 和指定模型，终态保留选模。挂载轮未连接或完成选模，仍无答复可评分。网关退出 0，端口关闭。两个私有源码副本各核对 4108 个文件摘要后清理，授权副本已清理，保留私有诊断证据。

- 从官方发行下载并按资产摘要验证 Darwin arm64 Harn 0.10.202。压缩包 SHA-256 为 `06730908c8e1778d85c1d1c74bde9f550a8ecc58046aef03ce4306fc39e659bf`，原生文件为 `74f3e9dbac52065979f12dcd0a3e3b262f83c1a3a680fa6ee88b90f3eb717ac7`。受管版本与新版本原生 ACP 都接受空列表，并对 stdio、HTTP、SSE 定义逐一返回相同的 `-32602`。新版本的无 pipeline 入口结果相同。发行物仅用于私有探针。[官方发行](https://github.com/burin-labs/harn/releases/tag/v0.10.202) [版本协议说明](https://github.com/burin-labs/harn/blob/v0.10.202/docs/src/harn-serve.md#acp)
- 用两版原生引擎分别运行 `AgentSpec.mcp_servers` 聚焦探针。复用原自然语言 pipeline、默认 ACP `ask` 模式、隔离环境和仅进程内可见的模拟中继凭据。两版均由 `tool:mcp:mutate` 拒绝 `harness.tools.mcp_bootstrap`，模型端收到 0 个请求，未执行图谱工具。分别断言真实权限拒绝，每版 1 项通过。`project.mcp_config` 宿主回调同样复现了 `tool:mcp:read` 拒绝，并再次观察到 stdio 子进程无模拟中继变量。原受管 Harn 的宿主写拒绝测试再跑 1 项通过。
- Harn CLI 当前提供的 `code` 模式同时改变文件写入、进程和外部服务权限，无法作为保留 `ask` 的替代。工作区配置或独立宿主执行也无法沿用现有 Harn ACP 的 MCP 效果检查。当前需要上游入口接收本会话的服务定义，或支持仅授权该会话中指定 MCP 服务且保留审批策略的宿主桥。取得该入口后按固定合同重跑。此次生产实现及 Cargo 范围均无变更，任务继续标记为已阻塞。
