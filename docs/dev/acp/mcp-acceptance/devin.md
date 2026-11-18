# Devin 动态挂载 RustCodeGraph

2026-11-19，任务 16 已阻塞。三轮均在 ShiJu 的 `session_model` 驱动门禁终止。隔离的 Devin 原生 ACP 能连接会话指定的 stdio 服务，指定模型选模和推理仍受阻。

## 环境和请求

- Darwin arm64，复用本机已有 ShiJu 0.6.0 开发产物，SHA-256 为 `e61098c0a9a4f5a6c6f658b191741bc880b4591031ef9d7dae3d9257af66882f`，未重新构建。私有回环网关复用固定的官方 Devin 受管运行时，安装复用接口 HTTP 200，安装状态为 true。实际入口为 `devin acp --process-model in-process`，版本 `3000.13.4 (d9a56503ce88)`，二进制 SHA-256 为 `62e08b13df72b4626a69669d536c460f45e77f0f58ffc82762436dac5dc8f272`，与受管清单一致。
- 从 `189d22dc1` 提取[验收请求](../ACP验收.md#可复用的验收请求)，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。使用 `acp-devin / devin`，沿用租户 `10001 / 100010005`、`session_model`、`full_access=true` 和 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017`。
- 首次启动的配置副本未解析到指定模型，三次 HTTP 请求均以“未找到模型”终止。随后从现有已授权来源仅同步该模型供应商配置及凭据到私有实例，配置、凭据写入和配置回读均为 HTTP 200，确认指定模型存在和预设启用后重跑下表。凭据只交给私有网关。
- RueJS 副本取自 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c`，索引 3478 个文件。仅挂载轮在副本写入 `.vwacprc.json`，使用 RustCodeGraph 1.2.10 的原生 stdio 和 `--no-watch`，服务名为 `ruejs_graph_probe`。

## 网关三轮

| 轮次 | 新会话 | HTTP / curl 退出码 / 秒 | SSE 终态 | 工具事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1ef9d541b0015CF6PcjZAqBKzK` | 200 / 0 / 4.449867 | `chat.error` | 0 |
| 挂载 | `ses_1ef9d65b0001NM8qbJ07IXmKrv` | 200 / 0 / 4.070101 | `chat.error` | 0 |
| 撤除 | `ses_1ef9d75c40017hS5jF72J7WFXy` | 200 / 0 / 4.185779 | `chat.error` | 0 |

三轮首个 ACP 失败均为 `ACP session_model driver is not verified`，发生在 `initialize` 和 `session/new` 之前。挂载轮网关日志显示宿主侧启动了 RustCodeGraph。三个会话的 UI summary 均回读到 `acp-devin / devin` 和 `full_access=true`。ACP options 均回读为 `connected=false`、空选项、无已选模型，`session_model` 状态为 `unsupported_cli`。其中 `cli_local` 是断开时的返回字段。

三个会话均无 `chat.done` 或模型答复，无挂载和撤除轮均未返回 `MCP_NOT_AVAILABLE`。挂载轮没有收到三种工具的调用、结果和权限事件，无法验收调用链、SSR、容器缺失、路由和 i18n、布局绕过、静态快照影响或两个验证场景。

## 原生 ACP 和独立 MCP 探针

- 使用同一已校验的 Devin 二进制，在无凭据、禁止网络的私有进程中分别建立 `peat-dragon`、`moored-spice` 和 `octagonal-rudbeckia`。三次 `initialize` 均协商协议 v1，声明 HTTP 和 SSE 支持。三次 `session/new` 均成功，只有挂载轮的 `mcpServers` 包含一个 stdio 定义。
- 挂载轮直接观测到 Devin 拉起 RustCodeGraph，并发送 `server/discover`、`initialize` 和 `notifications/initialized`，日志确认服务连接成功。等待八秒未观察到 `tools/list` 或 `tools/call`。无挂载和撤除轮无服务启动。
- 三轮的 `session/set_config_option` 均以 `-32602` 拒绝完整的指定模型。随后仅用独立短提示定位认证边界，`session/prompt` 均以 `-32000` 要求登录，ACP 无工具调用或权限请求。没有使用默认模型重做验收。固定发行包的 `share/devin/docs/reference/partner-builds.mdx` 将 OpenAI 兼容自定义端点限定在获批的合作伙伴构建。该构建的 ACP 中继及工具边界仍待验证。[官方模型说明](https://docs.devin.ai/cli/models)
- 独立 MCP 客户端完成三个真实调用。`rustcodegraph_query` 命中 `createRueSiteApp`（`app/app.tsx:32`），`rustcodegraph_callers` 命中 `mountRueSiteApp`（`app/app.tsx:39`），`rustcodegraph_node` 返回 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）转调 `_$createApp` 的源码。这些结果没有进入 Devin 会话。

未修改生产代码或权限策略，未运行 Cargo 和生命周期 fixture。自有网关与原生探针退出码均为 0，回环端口关闭，私有租户配置及工作区挂载配置已清理。原始探针及 SSE 保存在本机 0700 私有目录。复跑需先取得获授权、可严格绑定原模型且保留 ACP 权限边界的 Devin 推理入口。
