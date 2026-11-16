# 来源复核114：session-engine/tests/host_suites/prefetch_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/session-engine/tests/host_suites/prefetch_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/session-engine/tests/host_suites/prefetch_tests.rs`。
- 快照 SHA256：`668b8049789bdf8169b317fd75dd0c91df1b2603908016a09db9d6a388006d39`。源码变化后重新阅读。
- 快照词法属性 12 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/session/processor/prefetch.rs:32 | crates/libs/session-engine/tests/host_suites/prefetch_tests.rs | mod prefetch_tests | cfg(test) | session-engine（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `block_on_drives_pending_future_without_existing_runtime`
- `block_on_runs_future_inside_existing_runtime`
- `app_session_scope_from_root_ignores_missing_or_blank_roots`
- `app_session_scope_from_root_rejects_missing_directory`
- `try_prefetch_read_returns_false_for_missing_paths_and_directories`
- `try_prefetch_read_rejects_binary_files_without_recording_tools`
- `prefetch_seed_context_skips_when_query_contains_explicit_tool_call`
- `prefetch_seed_context_deduplicates_repeated_file_candidates`
- `prefetch_seed_context_leaves_session_unchanged_without_file_candidates`
- `prefetch_seed_context_ignores_http_urls_and_absent_files`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
