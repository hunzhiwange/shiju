# AGENTS 指令与技能协议一比一复刻计划

目标：以 Claude Code 的可观察行为为基准，完整复刻指令与技能运行协议，仅将指令文件族替换为 AGENTS。

范围：覆盖 AGENTS 分层发现、优先级、包含与条件规则、缓存和加载钩子、元用户提醒、技能多源发现与增量目录、Skill Tool 按需注入、压缩恢复、分叉和所有模型入口的一致接入。

范围外：不复刻 Claude Code 的品牌、界面、遥测后端、团队记忆和与指令/技能无关的产品功能。

假设：使用 VibeWindow 原生配置目录、权限系统、插件体系和 provider；行为顺序与消息语义对齐 Claude Code，指令文件仅识别 AGENTS.md、AGENTS.local.md 及 VibeWindow 对应规则目录。

## 设计决策

- 以 `claudemd.ts`、`context.ts`、`api.ts`、`attachments.ts`、`messages.ts`、`loadSkillsDir.ts` 和 `SkillTool` 的行为为基准建立回归契约。
- AGENTS 与技能是独立链路；删除从 AGENTS 正文猜测技能名称或路径并预加载完整技能的逻辑。
- AGENTS 以不可见的元用户 `<system-reminder>` 注入；技能目录只传名称、描述和来源，完整 SKILL.md 仅由 Skill Tool 调用后注入。
- VibeWindow 的安全审计、权限确认和插件信任边界继续生效；同构不允许绕过现有拒绝策略。

## 架构说明

- `session::instruction` 负责分层解析与生命周期，新的上下文装配器负责生成临时模型消息，各运行时不得自行拼接 AGENTS。
- `skills` 负责发现和元数据，`tools::skill` 负责验证与调用，会话状态负责增量目录和已调用技能的压缩恢复。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。

