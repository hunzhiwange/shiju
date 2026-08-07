# 委托代理配置工具内置必选 计划

目标：委托代理工具白名单拆成「可选 / 内置」两 Tab；内置工具代码写死且不可取消；MCP 动态工具不进入选择列表。

范围：在 `vw-config-types` 固化内置必选工具目录与 MCP 识别；`vw-agent` 会话过滤强制保留内置与 `mcp__*`；`vw-desktop` 过滤列表、合并白名单、工具页 Tab/锁定卡片与元数据。

范围外：不改 MCP 服务器配置页；不新增配置开关；不迁移历史 allowlist 格式；不改工具运行实现本身。

假设：用户所列「AskUseQuestion」指 `AskUserQuestion`；内置匹配含主 ID 与既有别名（如 `bash`/`shell`、`TodoWrite`/`todowrite`）；MCP 形如 `mcp__rustcodegraph__*`。

## 设计决策

- 共享目录放 `vw-config-types`（desktop/agent 已依赖），避免双端漂移。
- UI：Tab1 可选、Tab2 内置；内置始终勾选、不可取消，可打「内置」徽章（对齐 TodoWrite 强制保留思路）。
- 运行时：`allowed_tools` 非空时强制保留内置工具；并对 base 中存在的 `mcp__*` 一律保留，避免 UI 不可选后 MCP 失效。
- 预设/全选/反选只影响可选工具；结果始终并回当前可见的内置工具。

## 架构说明

- 现有入口：`system_settings_agents/tools.rs` 渲染；`message/settings/agents.rs` 处理切换；`session/processor/helpers.rs` 会话过滤。
- MCP ID 约定：`mcp__{server}__{tool}`（见 `mcp_tool::mcp_tool_id`）。
- 风险：`available_tools` 还被员工/自主/会话工具选择复用；过滤 MCP 会一并生效，符合「不进选择列表」。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
