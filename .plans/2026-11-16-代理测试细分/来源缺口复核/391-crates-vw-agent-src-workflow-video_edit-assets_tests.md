# 来源复核391：src/workflow/video_edit/assets_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/workflow/video_edit/assets_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/workflow/video_edit/assets_tests.rs`。
- 快照 SHA256：`13a049627e66a380d92e21bb23b42bf7707b753216753c6643482c998fec6eaf`。源码变化后重新阅读。
- 快照词法属性 11 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/workflow/video_edit/mod.rs:32 | crates/vw-agent/src/workflow/video_edit/assets_tests.rs | mod assets_tests | cfg(not(target_arch="wasm32")); cfg(not(target_arch="wasm32")); cfg(test) | serde_json | 未匹配→133 | 未发现 |

## 身份线索

- `video_edit_asset_jobs_unknown_is_not_ready`
- `video_edit_asset_jobs_hold_five_until_reverse_completion`
- `video_edit_asset_jobs_restart_reconciles_lost_receipt_without_post`
- `video_edit_asset_jobs_refusal_retries_share_peak_and_bound_attempts`
- `video_edit_asset_jobs_cancel_retains_remote_slots_and_stops_dispatch`
- `video_edit_asset_jobs_failure_or_unreadable_result_blocks_registry`
- `video_edit_asset_jobs_share_model_budget_and_resume_unknown`
- `video_edit_asset_jobs_unknown_missing_from_remote_is_not_resubmitted`
- `video_edit_asset_jobs_failed_attempt_retries_only_after_terminal_and_keeps_id`
- `video_edit_asset_jobs_deduplicate_content_parameters_and_scope_without_guessing_recipe`
- `video_edit_asset_jobs_restart_does_not_resume_cancelled_dispatch`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
