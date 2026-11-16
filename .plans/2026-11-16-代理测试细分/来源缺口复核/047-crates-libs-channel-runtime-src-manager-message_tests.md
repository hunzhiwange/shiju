# 来源复核047：channel-runtime/src/manager/message_tests

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】；依赖：无。

状态：未开始

目的：复核 `crates/libs/channel-runtime/src/manager/message_tests.rs` 的具体测试归属与真实宿主依赖。来源：任务133未关联候选，不是已确认遗漏实现。一个实际源模块；混合独立行为时先补窄实现卡。

## 源码和声明证据

- 当前候选：`crates/libs/channel-runtime/src/manager/message_tests.rs`。
- 快照 SHA256：`fc1901cff3533ce0912aba8c0ad1977e444be0e08bc029b655b70dcd8255c25f`。源码变化后重新阅读。
- 快照词法属性 10 个，不是当前 cfg 展开后的有效测试数。

| crates/vw-agent/src/channels/manager/message.rs:24 | crates/libs/channel-runtime/src/manager/message_tests.rs | mod message_tests | cfg(test) | channel-runtime（物理所有者；生产声明见该库 src/lib.rs） | 未匹配→133 | 未发现 |

## 身份线索

- `process_channel_message_returns_immediately_when_cancelled`
- `process_channel_message_stops_when_hook_cancels_message`
- `process_channel_message_consumes_runtime_command_before_model_execution`
- `process_channel_message_notifies_provider_initialization_failure`
- `excluded_tools_snapshot_for_channel_skips_cli_and_clones_runtime_list`
- `prepare_streaming_ui_state_skips_setup_when_streaming_is_disabled`
- `prepare_streaming_ui_state_without_target_keeps_delta_channel_only`
- `prepare_streaming_ui_state_tracks_visible_draft_updates_only`
- `maybe_add_processing_reaction_records_processing_emoji`
- `finish_feedback_tasks_cancels_typing_task`

## 步骤与验收

从当前真实父声明追踪 cfg/path/include 和包目标；核对全部原断言、共享 fixture、环境锁、进程、网络及资源释放。与 ../审计记录、实施提交和所属库现有测试逐项核对，不以文件名或同名属性作等价证明。按生产符号所有者判定每项迁移或保留理由；纯转发不自动构成真实 Host 必要性。需要迁移时先按真实行为补独立实现卡并保留来源证据，不在本审计任务吞并多个行为。

本轮不执行 Cargo 或测试。审计证据需包含完整旧→新身份、每项原断言的归属、真实父接线、cfg、fixture 和源码行数去重口径；未能确认项明确列出。为以后统一验证记录实际包/目标和完整精确过滤名，按有效 cfg 核对预期数，不将上述词法值当通过证据。无新运行证据，不声称来源覆盖或验收通过。
