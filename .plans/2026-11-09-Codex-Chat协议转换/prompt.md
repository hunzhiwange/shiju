# Codex ACP Chat Completions 任务提示词

按批次顺序执行。先阅读只读 plan.md 和对应任务文件。

## 任务 1 扩展Responses与Chat转换【批次 1】

依赖：无

```text
请使用技能 ./skills/do-task-plans/SKILL.md 执行 `.plans/2026-11-09-Codex-Chat协议转换/1-扩展Responses与Chat转换.md`。

开始前先阅读 `.plans/2026-11-09-Codex-Chat协议转换/plan.md` 和该任务文件，确认依赖已满足。
本任务涉及 Rust/Cargo：执行任何 Cargo 构建、检查、测试或调用 Cargo 的脚本前，先阅读并遵守 `.plans/2026-11-09-Codex-Chat协议转换/1-扩展Responses与Chat转换.md` 中的“Cargo 共享槽位规则”，动态领取仓库共享槽位 1–10，设置绝对路径 CARGO_TARGET_DIR；阶段结束后清理自己的锁并保留缓存，最终报告实际槽位和 CARGO_TARGET_DIR。
按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
验证通过后标记为已完成，提交实际变更时使用 git-commit 技能并排除无关变更；如果仍有阻塞，只在编号任务文件中记录原因和已运行的检查。最终报告验证证据。不要修改 plan.md。
```


## 任务 2 启用Codex客户端Chat模型【批次 2】

依赖：批次 1

```text
请使用技能 ./skills/do-task-plans/SKILL.md 执行 `.plans/2026-11-09-Codex-Chat协议转换/2-启用Codex客户端Chat模型.md`。

开始前先阅读 `.plans/2026-11-09-Codex-Chat协议转换/plan.md` 和该任务文件，确认依赖已满足。
本任务涉及 Rust/Cargo：执行任何 Cargo 构建、检查、测试或调用 Cargo 的脚本前，先阅读并遵守 `.plans/2026-11-09-Codex-Chat协议转换/2-启用Codex客户端Chat模型.md` 中的“Cargo 共享槽位规则”，动态领取仓库共享槽位 1–10，设置绝对路径 CARGO_TARGET_DIR；阶段结束后清理自己的锁并保留缓存，最终报告实际槽位和 CARGO_TARGET_DIR。
按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
验证通过后标记为已完成，提交实际变更时使用 git-commit 技能并排除无关变更；如果仍有阻塞，只在编号任务文件中记录原因和已运行的检查。最终报告验证证据。不要修改 plan.md。
```
