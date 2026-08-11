# 任务 1: 实现 AGENTS 分层发现与优先级

批次：【批次 1】 无

状态：已完成，待回归验证

目的：按 Claude Code 顺序发现托管、用户、项目和本地 AGENTS 文件，并保证越靠近当前目录优先级越高。

来源任务：用户要求一比一复刻 Claude Code 指令协议，仅加载 AGENTS.md。

预计会话范围：仅修改指令发现领域和相邻测试；不处理包含、技能或模型消息注入。

## 文件

- 新建：`crates/vw-agent/src/session/instruction_discovery.rs`
- 新建：`crates/vw-agent/src/session/instruction_discovery_tests.rs`
- 修改：`crates/vw-agent/src/session/instruction.rs`
- 测试：`crates/vw-agent/src/session/instruction_tests.rs`

## 上下文

- 当前 `FILES` 会在 AGENTS、CLAUDE、CONTEXT 中择一，`project_agents` 只读取所选根目录顶层文件；目标改为 AGENTS 文件族和根到 cwd 的稳定顺序。
- 对应目录为托管配置、用户配置、每层 `AGENTS.md`、`.vibewindow/AGENTS.md`、`.vibewindow/rules/*.md`、`AGENTS.local.md`。

## 测试计划

- 行为：多层目录同时存在 AGENTS 文件时，输出顺序和覆盖优先级与 Claude Code 一致，且不再读取 CLAUDE.md/CONTEXT.md。
- 失败验证测试：新增 `agents_discovery_orders_managed_user_project_and_local_root_to_cwd` 与 `agents_discovery_ignores_legacy_instruction_names`。
- 失败验证命令：`cargo test -p vw-agent agents_discovery_ -- --nocapture`
- 预期失败原因：现有实现只选择一种文件名，且项目入口仅读取根目录 AGENTS.md。
- 通过验证命令：`cargo test -p vw-agent agents_discovery_ -- --nocapture`
- 模拟策略：使用真实临时目录和环境隔离锁，不模拟文件系统。

## 步骤

1. 编写覆盖加载层级、根到 cwd 顺序、去重和旧文件名拒绝的失败测试。
2. 运行失败验证命令并确认失败来自当前发现规则。
3. 新增结构化来源类型和确定性目录遍历，替换 `FILES` 择一及顶层专用读取。
4. 保留禁用项目配置、工作树边界和规范化路径的安全行为。
5. 运行聚焦测试并审查只改变指令发现语义。

## 验证

- 运行：`cargo test -p vw-agent agents_discovery_ -- --nocapture`
- 预期：新增用例全部通过，输出顺序固定，CLAUDE.md/CONTEXT.md 不出现在解析结果。
- 所需证据：实现前预期失败、实现后退出码 0、测试数量与已审查差异。

## 完成

完成后应由 `instruction_discovery.rs` 唯一决定 AGENTS 来源顺序；`instruction.rs` 不再按旧文件名回退，聚焦测试通过。

## 验证记录

- 已运行 `cargo test -p vw-agent agents_discovery_ -- --nocapture`；因并发的 `cargo check/clippy -p vw-agent --all-targets --all-features` 持有 package/artifact 锁，未能取得退出码。
- 已运行 `git diff --check`，本任务改动无空白错误。
