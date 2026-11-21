# 任务 3 · 内置 ACP 后端与纯文本发送策略

先阅读并遵守 [公共执行规则](plan.md)。

批次【批次 1】 · 无依赖  
状态为 `未开始`。

目的为让已授权的员工会话通过现有 ACP 宿主选中内置后端，并发送当前用户的纯文本。任务以启动契约和可控子进程验证宿主，网关的账号与员工绑定由任务 4 完成。

## 文件

- 修改 `crates/libs/acp-agent-registry/src/agent_registry.rs`、`crates/libs/agent-config/src/catalog.rs`；检查 `schema/config_validate.rs` 的内置后端约束。
- 修改 `crates/libs/llm-runtime/src/acp/request.rs` 和按需修改 `replay.rs`、`session.rs`，测试放入相邻 `request_tests.rs`、`replay_tests.rs`。
- 复用 `crates/libs/acp-client/src/client/process.rs` 和现有会话恢复策略。依据现有启动实现决定最小改动及其相邻用例。

## 步骤

1. 注册单一内置后端 `employee-acp` 和 profile `acp-employee-acp`。受控启动指向本机网关实际运行的 ShiJu CLI 绝对路径。用任务 4 提供的服务端员工绑定及请求期凭据实例化启动参数。拒绝通过自定义命令、用户可编辑环境或普通 Agent 继承这些凭据。
2. 员工模型由产品负责，使用宿主已有无需本地 provider 选模的启动路径。将工具、技能和 MCP 请求限制在此后端的纯文本范围。保持其他内置 ACP 的模型和权限行为。
3. 在构造 ACP prompt 前取当前轮用户文本，从源消息区分系统、工作区协议、历史和用户输入，拒绝不支持的输入。既有会话按 ACP ID 恢复，恢复失败明确返回，避免回放整段本地历史作为一轮新提示。关闭后重开和「停止」继续走原宿主链路。

## 测试计划与验收

- 新增 `employee_acp_builtin_` 目录与准入用例，预期旧目录缺少可选后端。命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p agent-config --lib employee_acp_builtin_`。
- 新增 `employee_acp_host_` 用例，现有 `build_request_prompt` 路径会附加本地工作区协议文本，预期回环 peer 捕获到不应发出的内容或启动进入模型凭据分支。命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib employee_acp_host_`。
- 两条命令均用于失败与通过验证。真实 ACP 消息断言当前正文、无额外本地上下文、恢复失败未建新会话及受控启动。补一例现有 Agent 的正常协议上下文，证明员工规则的适用边界。

