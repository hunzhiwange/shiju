# 技能权限回归评测计划

目标：新增一条会话控制选中 `create-task-plans` 后可成功调用技能并完成 TypeScript 编辑器的 Promptfoo 回归评测。

范围：新增独立 YAML 用例；在通用断言中增加可选的必须成功技能调用检查和辅助写入目录放行；执行配置校验、目标评测与 Clippy。

范围外：不修改产品权限逻辑，不更改现有评测用例的语义。

假设：Gateway 使用仓库根目录作为 Agent 工作目录；`skill` 工具输入中以 `skill` 字段携带技能名。

## 设计决策

- 新用例使用独立 `coding-skill` 评分档案，避免改变现有 coding metric 权重。
- 将 `.plans/` 作为可选辅助写入范围，主产物和测试仍必须在 `examples/editor1` 内。

## 架构说明

- `promptfooconfig.yaml` 通过通配符自动加载 `tests/*.yaml`。
- `assertions.mjs` 根据 `evaluationProfile` 选择确定性轨迹断言。

## 开发策略

- 先让新 YAML 引用尚不存在的评分档案，确认配置验证无法完整覆盖新语义。
- 增加最小断言和档案后运行语法、Promptfoo 配置、目标评测和仓库 Clippy。
- 只根据新用例需要扩展可选变量，不改变旧用例默认行为。
