# 统一会话提示词 计划

目标：去掉按模型分流的会话系统提示，改为一份统一提示；优先服从用户本轮要求与 `AGENTS.md`；身份覆盖代码模式与工作模式，不再锁死“只写代码”。

范围：重写并统一 `crates/vw-agent/src/session/prompt/` 下会话系统提示；改 `system.rs` / `messages.rs` 的装载与去重；清理已删除的 `using-superpowers` 残留测试；补聚焦测试与 clippy。

范围外：不把技能注入默认改为 Full（保持 Compact，避免正文撑爆上下文）；不改桌面代码/工作模式 UI；不重写项目根 `AGENTS.md` 业务协议正文；不清理其他技能里历史 `superpowers:` 文案（资产已不在仓库）；不改权限/模型路由。

假设：`using-superpowers` 资产已从 `assets/skills/` 移除，仅 `desktop_skills_tests.rs` 仍引用；`copilot-gpt-5.txt` / `anthropic-20250930.txt` 未被 `include_str!`；会话系统提示经 `system::provider` 与（Codex OAuth 时）`instructions()` 注入；Compact 仅放技能摘要，正文靠 Skill 工具按需加载。

## 设计决策

- 单一提示文件 `unified.txt`；删除按模型的 `codex_header` / `beast` / `gemini` / `anthropic` / `qwen` / `trinity` 及未引用的 orphan 文件；保留 `max-steps.txt`。
- `provider(_)` 恒返回统一提示；`instructions()` 返回同一正文；`is_codex` 不再二次追加，避免全文重复。
- 指令优先级写入统一提示：用户本轮要求 > 已注入项目协议（含 `AGENTS.md`）> 系统默认工具指引；默认 Grep/Read 不得绕过项目协议。
- Compact 保持默认：本计划不改为 Full；按需加载技能正文，防止上下文爆炸。
- 身份改为通用自主助手，同时服务代码模式与工作模式任务，不自我限定为“仅编码 CLI”。

## 架构说明

- 装载：`session/system.rs` → `session/llm/messages.rs` → 流式会话。
- 项目协议仍由 `session/instruction.rs` / `session/prompt.rs` 注入；统一提示只声明优先级，不内嵌某仓库 `AGENTS.md` 细则。
- 内置技能目录：`assets/skills` + `desktop_skills.rs` 的 `include_dir!`；嵌套资源安装测试改用仍带附属文件的内置技能（如 `rustcodegraph`）。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
