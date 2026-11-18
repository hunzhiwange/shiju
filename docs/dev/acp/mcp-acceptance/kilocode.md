# Kilocode 动态挂载 RustCodeGraph

2026-11-19。Kilocode 原生 stdio 已完成服务发现、目标工具调用及结果续轮。完整验收合同在模型调用额外工具时首次偏离。

## 环境

- Darwin arm64 受管 `@kilocode/cli` 7.8.18，启动参数为 `acp --pure`。实际加载的 `reviewed/kilocode` SHA-256 为 `19cc66a17aead6565b60dda07f7f7a47a1b73f72e0d91a9456fd94a483cfa7ff`，与仓库固定契约一致。独立握手报告 ACP v1，支持加载、恢复以及 HTTP 和 SSE MCP。本次选原生 stdio。
- 私有回环网关使用已有的 `shiju 0.6.0`，SHA-256 为 `f7dc15b13f941ae96a6457d39660388308e79305f41f9f3575f682beddfd5790`。仅在私有实例启用预设，使用租户 `10001 / 100010005`、模型 `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` 及 `session_model`。既有供应商凭据仅供私有网关使用。Kilocode 驱动为模型路由设置私有 XDG 配置与按会话隔离的状态目录。MCP stdio 定义的环境数组为空。
- 请求来自提交 `189d22dc1`，提示 SHA-256 为 `207198f85dcd07b542ad54b77fb4630b4c4d64ca3f9c0ead912454cbd75db4cc`。RueJS 从 `main` 的 `14fe287b13549f5183d42fbc45b28e1498ffae1c` 导出私有副本。RustCodeGraph 1.2.10 索引 3478 个文件，命中 `app/app.tsx:32` 的 `createRueSiteApp`。 [复用请求](../ACP验收.md#可复用的验收请求)

## 真实网关走查

| 轮次 | 新会话 | HTTP / curl / 秒 | SSE 终态和答复 | 工具状态事件 |
| --- | --- | --- | --- | --- |
| 无挂载 | `ses_1efb4f9a1001c8KUAZd0gEzMDn` | 200 / 0 / 10.823129 | `chat.done`，`MCP_NOT_AVAILABLE` | 0 |
| 挂载 | `ses_1efb5257f001nGADgGTiRRSMY4` | 200 / 0 / 147.427393 | `chat.done`，完整调查答复 | 385 条，77 个调用完成 |
| 撤除 | `ses_1efb76629001TT7BCBk2yJAsWm` | 200 / 0 / 9.589136 | `chat.done`，`MCP_NOT_AVAILABLE` | 0 |

每轮恰有一个 `chat.done`，无 `chat.error`，对话步骤以 `stop` 结束。运行中和终态的 UI summary 均回读 `acp-kilocode / kilocode` 和 `full_access=true`。ACP options 在运行中为 `connected=true`、权限 `client_only`，终态连接释放。两阶段均回读指定模型、提供商和 `session_model`。未观察到审批交互，原生免审批能力未验证。网关日志另记两条后台会话标题生成的 HTTP 403，涉及 `kimi-k2.7-code`，三轮指定模型的对话均收到成功终态。

挂载轮的每个调用 ID 都有开始和完成事件，无失败状态。三种目标工具的实际输出包括：

| 工具 | 完成次数 | 示例调用 ID 和结果 |
| --- | ---: | --- |
| `rustcodegraph_query` | 33 | `call_34340cb709084f95b3572f91` 命中 `createRueSiteApp`，`app/app.tsx:32` |
| `rustcodegraph_callers` | 5 | `call_aedcd997607a4823931e4397` 返回调用者 `mountRueSiteApp`，`app/app.tsx:39` |
| `rustcodegraph_node` | 29 | `call_5156e2e54fef4f588f01dccc` 返回 `createRueSiteApp` 源码。另两次回读 `useApp`（`packages/runtime/src/hooks/useApp.ts:4`）与 `_$createApp`（`packages/runtime/src/compiler-runtime/app.ts:57`） |

最终答复还原了完整调用链，说明浏览器与 SSR 的挂载决策、`#app` 缺失时抛错、路由与 i18n 在挂载时安装、`/rust-layers` 绕过布局。关键结论与回读源码吻合。答复还指出调用图漏报模块作用域调用，以及 `createAppRouter` 等符号的检索缺口。它推断去掉 `replaceChildren()` 会保留快照节点并造成重复渲染，给出首屏快照去重与深链接布局接管两项带预期的检查场景。两项场景均为建议验证，未运行浏览器。

首个不符合项是模型在首次 `query` 后发出 `rustcodegraph_status`（`call_036c4ac90e754d729b7d034e`），该调用完成并返回索引状态。模型后续还调用 `files` 5 次和 `explore` 4 次。所有调用均来自指定服务。三种目标工具有真实输出，额外调用仍使完整合同未通过。

## 独立协议验证及收尾

同一受审二进制分别在三个私有 HOME 下处理 `session/new`，传入空列表、单条 stdio 服务和空列表。回环模型端点依次收到 0、8、0 个目标服务工具声明，挂载轮三种目标工具的结构化定义齐全。控制的 `query` 调用执行了真正的 RustCodeGraph，`app/app.tsx:32` 的命中内容出现在下一条模型请求。三轮 `session/prompt` 均返回 `end_turn`。该探针用于验证协议续轮边界，真实网关三轮另见上表。

保持现有实现和权限，未改 Rust 或受管补丁。转发和会话生命周期未出现故障，未运行编号任务中的定向 Cargo 测试。临时挂载配置和私有网关的供应商凭据已撤除，探针子进程已退出，私有端口已关闭。档案中的 4109 个文件和链接经回读与基准一致，副本仅有索引产生的新增文件。原始日志位于本机 0700 私有验收目录。
