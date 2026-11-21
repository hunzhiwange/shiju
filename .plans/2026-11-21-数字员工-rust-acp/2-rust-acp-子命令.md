# 任务 2 · Rust ACP 子命令与云端会话

先阅读并遵守 [公共执行规则](plan.md)。

批次【批次 2】 · 依赖任务 1  
状态为 `未开始`。

目的为通过真实 stdio 提供产品会话建立、发送、恢复、列表和取消。会话范围包括 ACP Agent 与子命令，产品 HTTP 由任务 1 提供。

## 文件

- 在 `crates/libs/acp-server/src/` 新建 `employee.rs`、`employee_state.rs`、`employee_tests.rs`，修改 `lib.rs` 与所需清单。
- 修改 `crates/libs/cli-command-schema/src/cli.rs`、`crates/libs/cli-bootstrap/src/lib.rs`，测试放入相邻 `acp_tests.rs`。
- 复用 `acp-server/src/server/agent.rs` 的 `AgentSideConnection`、异步 stdio 与 update sink 模式，参考既有 Go bridge 的响应丢失和取消语义。

## 步骤

1. 使用锁定的 `agent-client-protocol` SDK 实作受限 ACP Agent。优先复用 Agent 连接和历史通知接线。按现有 `SessionBackend` 能力检查列表和确认式取消；若通用适配在这些点上语义不同，保留专用员工 Agent，并复用传输层，避免改变其他后端。
2. `session/new` 分配可恢复的本地 ACP ID，首条提示才提交云端创建。以服务端会话 ID、用户消息 ID 和回复归属关联轮次。保存账号范围、目标、ACP 到产品会话的映射、未决发送的幂等键和必要的轮次标识。持久化提交前意图，处理响应丢失、进程重启、重复提交和并发写入，已确认的轮次沿用原发送身份。状态使用该项目已有的原子文件或存储写入模式。历史正文由产品读取。
3. `session/load` 校验当前身份和目标仍可访问，按顺序回放产品历史。列表按当前员工、账号及工作目录元数据解释游标，重开无需创建新云端会话。拒绝不支持的内容块或非空 MCP 请求。SDK 优先使用现有版本提供的能力门控，真实支持后才声明。
4. `session/prompt` 只发送符合产品长度上限的纯文本并有界等待。按本轮结果发一个最终正文通知再结束。取消跨越提交竞态时记录意图，取得本轮 ID 后发取消。以产品确认的 `CANCELLED` 或先到达的完成状态为准。EOF、进程关闭、超时和显式取消分别处理。用安全错误区分已拒绝与尚未确认，保留可恢复的未决记录。
5. CLI 在现有 ACP 早期分发处解析新命令，通过进程私有环境读取当前凭据。stdout 承载协议帧，诊断送 stderr。账号状态隔离以及路径权限失败须明确报错。

## 测试计划与验收

- 新建一组 `employee_acp_` 用例，覆盖私有和共享各两轮、重开回放、跨账号不可恢复、取消后下一轮、丢失首次 POST 响应后复用键、取消与完成竞态和 EOF。复用内存 I/O 或真实 SDK 连接，模拟产品 HTTP 边界。
- 失败和通过命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p acp-server --lib employee_acp_`。用可编译的初始入口证明缺少会话结果或续聊恢复，再运行通过版本。
- `python3 scripts/cargo_slot.py cargo test -j 1 -p cli-bootstrap --lib employee_acp_` 覆盖命令解析、凭据缺失及 stdout 单独承载帧。现有 ACP 分发测试可用同一目标中原来的 `acp_command_routes_to_agent_side_stdio` 回归。

