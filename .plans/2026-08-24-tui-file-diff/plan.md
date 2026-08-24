# TUI 文件差异接线计划

目标：让活动 V3 TUI 在文件工具完成后展示结构化 diff，并提供可用的 `/diff` 会话差异命令。

范围：扩展产品中立 V3 展示事件；复用现有补丁历史样式；从 `chat.tool` 的 `ToolResultDto.content` 提取 `structured_patch`；通过 Gateway 会话 diff 接口实现 `/diff`。覆盖 `file_write`、`file_edit`、`apply_patch`，保留普通工具卡片和并行工具生命周期。

范围外：不改变工具写入、安全审批、Gateway 协议或桌面端渲染；不迁移旧 Codex app-server 协议；不做全仓格式化。

假设：Gateway 已完整透传工具结果 `result`；写文件工具已生成结构化补丁；会话 diff 接口返回 `vw_shared::snapshot::FileDiff`。当前工作树存在文件搜索相关未提交改动，实施时必须逐块保留。

## 设计决策

- 使用显式的 V3 文件差异 DTO，避免让 `vw-tui` 依赖 Gateway 或工具内部 DTO。
- bridge 强类型解析 `ToolResultDto`，仅消费 `StructuredPatch` 内容块；缺失或畸形补丁回退为普通工具结果。
- 并行工具期间暂存 diff，执行组完成后按到达顺序提交，避免拆散仍运行的工具组。
- `/diff` 调用已有 Gateway 会话 diff API，不在 TUI 直接执行不受控 Git 子进程。

## 架构说明

- `vw-cli` 拥有 Gateway I/O 与协议转换，`vw-tui` 只持有展示状态。
- 文件路径与补丁文本均视为展示数据；不得读取文件或扩大目录权限。
- 现有 `diff_model`、`diff_render` 与补丁历史样式可修复复用，但活动入口必须由 `lib.rs`、`history_cell` 和 `ChatWidget` 显式接线。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实事件转换和终端渲染；Gateway HTTP 边界使用现有测试服务器。
- 仓库禁止主动运行 `cargo test`，因此用 `cargo clippy --all-targets` 编译新增测试，并在交接中注明行为测试尚未执行。
