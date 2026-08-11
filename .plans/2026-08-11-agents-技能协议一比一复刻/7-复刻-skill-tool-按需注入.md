# 任务 7: 复刻 Skill Tool 按需注入

批次：【批次 3】 依赖批次 2

状态：已完成，待回归验证

目的：强制模型在技能匹配时先调用 Skill Tool，并把完整 SKILL.md 作为元用户消息按需注入。

来源任务：用户要求像 Claude Code 一样自动使用 AGENTS 中规定的技能。

预计会话范围：聚焦 Skill Tool 描述、参数、验证、权限和结构化结果，不接入各模型循环。

## 文件

- 新建：`crates/vw-agent/src/tools/skill/prompt.rs`
- 新建：`crates/vw-agent/src/tools/skill/invocation.rs`
- 新建：`crates/vw-agent/src/tools/skill/invocation_tests.rs`
- 修改：`crates/vw-agent/src/tools/skill/mod.rs`
- 修改：`crates/vw-agent/src/tools/traits/mod.rs`
- 测试：`crates/vw-agent/src/tools/skill/tests.rs`

## 上下文

- 当前工具描述只说“匹配时调用”，成功结果作为普通工具文本返回；Claude Code 使用 BLOCKING REQUIREMENT，并返回完整技能的 meta user `newMessages`。
- V2 `ToolCallResult.extra_messages` 已存在但未被 Skill Tool 使用；参数改为 Claude 同构的 `skill` 与可选 `args`，兼容别名只保留真实调用者所需部分。

## 测试计划

- 行为：匹配提示明确要求先调用；禁用模型调用的技能被拒绝；成功调用返回完整正文、基础目录、参数替换和 user 角色额外消息。
- 失败验证测试：新增 `skill_tool_prompt_requires_blocking_invocation_before_response` 与 `skill_tool_returns_full_skill_as_meta_user_extra_message`。
- 失败验证命令：`cargo test -p vw-agent skill_tool_ -- --nocapture`
- 预期失败原因：现有工具没有阻塞措辞、args 和 extra_messages 注入。
- 通过验证命令：`cargo test -p vw-agent skill_tool_ -- --nocapture`
- 模拟策略：真实临时 SKILL.md；仅权限交互使用现有测试替身。

## 步骤

1. 编写阻塞提示、技能不存在、模型禁用、权限拒绝、参数替换和额外消息失败测试。
2. 运行失败验证并记录现有普通工具输出差异。
3. 拆分工具 prompt 与 invocation，复用统一技能发现器和安全审计。
4. 使用 `ToolCallResult.extra_messages` 返回完整技能元用户消息，工具模型结果仅保留简短确认。
5. 应用 allowed-tools、model、effort、hooks 和 fork context 更新；不支持路径明确报错。
6. 运行聚焦测试并确认未写死任何技能名称。

## 验证

- 运行：`cargo test -p vw-agent skill_tool_ -- --nocapture`
- 预期：Skill Tool 合约测试通过，完整正文只出现在额外元用户消息中。
- 所需证据：红绿输出、退出码 0、权限拒绝、参数替换、角色与正文断言。

## 完成

Skill Tool 具备 Claude 同构调用合约，返回可由运行时消费的元用户技能消息和上下文更新。

## 回归验证记录

- 已运行 `CARGO_TARGET_DIR=/tmp/vibe-window-task-7-red cargo test -p vw-agent skill_tool_ -- --nocapture`；命令在编译 `vw-agent` 时被无关工作树错误阻断，尚未执行 Skill Tool 测试。
- 阻断错误：`session/instruction.rs` 引用了缺失的 `instruction_cache`、`instruction_include`、`instruction_rules` 子模块；`gateway/api/handlers/session/stream_tests.rs` 缺少 `DelegateAgentConfig` 导入；`skills/discovery_tests.rs` 引用了私有 `helpers` 模块；`session/child_agent_tests.rs` 调用了私有 `stream_request`。这些文件均不在本任务范围。
- 已完成本任务的 `rustfmt --edition 2024`（仅变更文件）与 `git diff --check`，二者退出码均为 0。
