# Corust Agent 受管工具共存

2026-11-19 在 Darwin arm64 用固定版 `0.6.0` 验收。将缓存归档离线交给仓库受管安装脚本，归档及四个安装文件均通过摘要校验。受管目录后缀为 `model-source-v4`，入口为 `corust-agent-acp`，原生文件摘要仍为 `a2a644a851ee03c583ed9f113cc3acbf54d99ff9f8aaaa782535dbd576d97a2e`。

macOS 拒绝嵌套 Seatbelt 配置，返回 `sandbox_apply: Operation not permitted`。复验改为在现有原生和 RustCodeGraph 子进程配置内阻断 Mach 服务查找与注册、Apple Events、POSIX IPC 和钥匙串路径读取。原生进程仅能访问绑定的回环中继，RustCodeGraph stdio 子进程禁网。模型夹具监听 `127.0.0.1`，MCP 使用已有 `ruejs_graph_probe` 服务和私有 RueJS 索引副本。所有运行均经受管入口，未启动供应商登录或修改钥匙串及系统权限。

在相同隔离下，原桥的挂载目录只有三个图谱工具，缺少 `read_file`、`write_file`、`glob`、`grep`。修复前探针退出 1。修正后的桥保留四项受审原生声明，并附加当前挂载目录。原生调用交回 Corust，图谱调用继续走单次批准及独立只读沙箱。混合批次先完成图谱侧处理，再从 Corust 续轮收集原生结果，按模型原始调用顺序和 ID 组装同一次续轮。

同一挂载会话完成三组调用。四次回环模型请求均返回 HTTP 200，ACP 提示以 `end_turn` 结束，受管入口及探针均退出 0。每个 ACP 开始事件均有同 ACP ID 的完成事件。下表的模型调用 ID 在模型请求中的 assistant 声明和 tool 结果之间完全一致。

| 批次 | 工具 | 模型调用 ID | 执行方 | ACP 状态 | 模型续轮结果 |
| --- | --- | --- | --- | --- | --- |
| 原生单调用 | `read_file` | `fixture-0-0` | Corust 原生文件工具 | `completed` | `app/app.tsx` 中的 `const app = useApp(RootApp).use(router).use(i18n)` |
| MCP 单调用 | `rustcodegraph_query` | `fixture-1-0` | RustCodeGraph stdio，经 `allow_once` | `completed` | `createRueSiteApp` 命中 `app/app.tsx:32` |
| 同批混合 | `read_file` | `fixture-2-0` | Corust 原生文件工具 | `completed` | 同一实际文件内容 |
| 同批混合 | `rustcodegraph_callers` | `fixture-2-1` | RustCodeGraph stdio，经 `allow_once` | `completed` | 调用方 `mountRueSiteApp` |

另通过三项图谱工具的单类顺序调用、拒绝审批、审批中取消后补发批准、只读模式聚焦探针。私有源文件的 SHA-256 前后一致，探针退出后图谱快照目录已清理。共存复验命令为 `python3 patches/acp-model-source/corust-agent/probe.py "$ACP_LAUNCHER" "$GRAPH_EXE" "$ACP_WORKSPACE" --coexist`。名单、撤除、恢复、多平台及供应商原包对照均未纳入本轮验收。未运行 Cargo 或桌面构建。
