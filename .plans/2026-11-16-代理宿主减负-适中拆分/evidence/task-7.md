# 任务7：压缩溢出预算

实现完成。按用户本次覆盖要求，未运行任何 Cargo 命令、单元测试、实机测试或工具链验证，验证统一延期；无测试退出码、有效通过数、失败数及 ignored 数。删除自身编号任务文件，plan.md/prompt.md 保持只读。

前置证据读取 evidence/baseline.md 和 evidence/task-2.md。当前目录当前分支实现。

## 断言映射

下列原宿主测试完整迁入 session-engine/src/host_split_7_tests.rs，保留同名，各两条断言；不保留宿主重复断言：

| 原用例（新领域用例同名） | 完整迁移的领域断言 |
| --- | --- |
| overflow_uses_fallback_for_unknown_and_respects_explicit_input | 未知窗口使用默认预算；显式输入限制下判定溢出 |
| overflow_uses_total_when_present_and_rounds_threshold | total 优先于分量；89999 未溢出、90000 溢出 |
| overflow_falls_back_to_component_sum_and_input_limit | 无 total 时累计 input/output/cache.read/cache.write；输入限制下99999 未溢出、100000 溢出 |
| overflow_treats_negative_counts_as_zero | 负 total 不回退正分量；负分量总和按零处理 |

源码候选数量：领域新增4条，宿主移除4条，保留指定宿主用例0条。没有执行编译清单，以上不是实际测试发现或通过数量。

## 接线与范围

- session-engine/src/lib.rs 在 cfg(test) 下独立注册 host_split_7_tests；直接使用 crate::compaction::is_overflow、crate::message 和已有 model-types 依赖，无宿主反向依赖，无新依赖。
- 宿主 compaction.rs 原已 re-export session_engine::compaction::is_overflow，真实调用接线沿用；没有复制算法或修改生产行为。
- 局部复制 model_with_limits/capability_io/token_info 最小输入夹具。原宿主夹具仍供 overflow_reserves_summary_output_space 及其它原有测试使用，不移除或改共享可见性，不建跨库框架。
- 对照 context_governance_tests.rs 的 auto_compaction_caps_total_threshold_at_input_and_output_budgets 和 auto_compaction_unknown_window_uses_runtime_fallback_and_kimi_keeps_input_room：其覆盖 should_compact 的 u64/ModelLimit 纯预算，不覆盖 is_overflow 的 TokenInfo.total 选择、分量求和与负数截断；保留已有纯预算测试，本组通过真实 is_overflow 覆盖适配行为，无等价整条用例可去重。
- 没有修改摘要生成、checkpoint、UI 快照或其它测试族。原宿主 overflow_reserves_summary_output_space 保持不动。

后续统一验证须执行原任务规定的领域 host_split_7_tests 定向测试（源码预期4条），并编译宿主 lib-test/获取注册清单，核对四条原名不再归属宿主。迁移前基线未运行，不能补称通过；未执行检查不构成行为验收。
