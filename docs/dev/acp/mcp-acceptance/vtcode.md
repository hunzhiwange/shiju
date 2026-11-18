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
