# 来源复核402：src/workflow/video_edit/prepare_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/vw-agent/src/workflow/video_edit/prepare_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/vw-agent/src/workflow/video_edit/prepare_tests.rs`。
- 快照 SHA256：`4a8df8108a8d4c2138dfcf2f727a6cfaec0e960bc5bd0c3924f11e60cc041b5a`。源码变化后重新阅读。
- 快照词法属性 15 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/workflow/video_edit/mod.rs:21 | crates/vw-agent/src/workflow/video_edit/prepare_tests.rs | mod prepare_tests | cfg(not(target_arch="wasm32")); cfg(not(target_arch="wasm32")); cfg(test) | anyhow | 未匹配→133 | 未发现 |

## 身份线索

- `video_edit_prepare_unbound_readonly_source_reuses_durable_run`
- `video_edit_prepare_unknown_writes_reconcile_without_duplicate_side_effects`
- `video_edit_prepare_lost_session_response_stays_pending_without_resubmission`
- `video_edit_prepare_source_content_and_snapshots_invalidate_cache`
- `video_edit_prepare_replacement_and_nonvideo_never_return_success`
- `video_edit_prepare_two_projects_need_no_binding_and_have_independent_sources`
- `video_edit_prepare_no_space_and_unapproved_choice_have_no_side_effects`
- `video_edit_prepare_multiple_spaces_use_real_question_and_cancel`
- `video_edit_prepare_streaming_hash_obeys_security_and_cancellation`
- `video_edit_prepare_cross_tenant_runs_and_native_mismatch_are_isolated`
- `video_edit_prepare_competing_requests_create_and_upload_only_once`
- `video_edit_prepare_incomplete_put_never_becomes_success_or_reuploads`
- `video_edit_prepare_source_replacement_with_same_bytes_changes_identity`
- `video_edit_prepare_proven_non_submission_retries_same_ledger_without_duplicate_project`
- `video_edit_prepare_unknown_without_matching_project_never_reposts`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
