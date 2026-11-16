# 任务9：Cron 测试归属迁移

代码完成，编译/行为未验证。本轮按公共规则未运行 Cargo（含 list/fmt/check）、单元测试、构建或实机检查；仅核对源码、注册、依赖及差异。

## 所有者与生产路径

实际调用为 bootstrap::client_router → 宿主 handlers::cron 的七个 HTTP 入口 → gateway_cron::handlers 同名入口。真实所有者为 gateway-cron（scope.json 已列为候选），因此不向 agent-gateway 添加测试或依赖。既有生产下沉不计本轮生产迁移收益；生产业务迁移为 0 行。

宿主 cron.rs 保留 cron_state（注入共享配置、require_auth 与 scheduler::Host）及 handle_api_cron_preview/list/history/runs/update/add/delete 七个薄适配入口。它们连接真实 AppState 与领域 CronState，均有路由调用者，不新增框架或反向依赖。

领域通过 lib.rs 的 cfg(all(test, not(wasm32))) 自注册 host_migration_9_tests，直接调用本库 handlers 与 agent-cron 真实 SQLite 存储。夹具仅构造 CronState，不复制 AppState；认证拒绝端口只用于领域 preview 拒绝断言，真实鉴权仍由宿主测试覆盖。NoDelivery 意外 webhook/send 操作明确失败；latest_config 返回输入快照只用于未配置企微的校验，真实租户配置回读保留宿主。新增 dev 依赖 async-trait/tempfile，Cargo.lock 仅补现有包依赖边。未添加 path/include 宿主源码加载。

## 全文件行数（splitlines）

| 文件 | 原宿主生产 | 原宿主测试 | 现宿主生产 | 现宿主测试 |
|---|---:|---:|---:|---:|
| cron.rs | 41 | 5 | 41 | 3 |
| cron_tests.rs | 0 | 1023 | 0 | 307 |
| cron_schedule_tests.rs | 0 | 438 | 0 | 0 |

来源 1507 → 351 行，净减 1156 行（76.71%），达到70%门槛。领域生产新增0行；测试新增如下（含夹具与注册）：

- `crates/libs/gateway-cron/src/host_migration_9_tests.rs`：149 行。
- `crates/libs/gateway-cron/src/host_migration_9_tests/delivery.rs`：47 行。
- `crates/libs/gateway-cron/src/host_migration_9_tests/history.rs`：105 行。
- `crates/libs/gateway-cron/src/host_migration_9_tests/lifecycle.rs`：277 行。
- `crates/libs/gateway-cron/src/host_migration_9_tests/policy.rs`：281 行。
- `crates/libs/gateway-cron/src/host_migration_9_tests/schedule.rs`：405 行。

lib.rs 新增3行测试注册，Cargo.toml 新增4行 dev依赖。宿主 cron_tests.rs 保留共享 AppState/请求夹具、2条真实鉴权测试、history 真实路由认证、双别名 preview 路由及真实租户配置投递往返；cron_schedule_tests.rs 整文件删除，其挂载删除；cron.rs 删除领域 helper 的测试专用导入。

## 全用例映射（保留名称与全部断言）

| 原文件 / 用例 | 归属及目标模块 |
|---|---|
| cron_tests.rs / `cron_json_serializes_schedule_variants_and_delivery_fields` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_json_serializes_absent_optional_job_and_run_fields_as_null` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_add_schedule_infers_and_validates_kinds` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_add_schedule_reports_missing_values_and_accepts_localized_kinds` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_update_schedule_supports_partial_updates_and_localized_kinds` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_update_schedule_rejects_invalid_at_and_every_values` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_add_job_type_delivery_patch_and_log_fields_normalize_input` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_add_delivery_and_patch_apply_safe_defaults_for_blank_values` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_add_job_type_defaults_from_prompt_and_command` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_update_schedule_treats_blank_kind_as_default_and_requires_matching_value` | gateway_cron::host_migration_9_tests::policy |
| cron_tests.rs / `cron_handlers_reject_requests_when_pairing_is_required` | 宿主 cron::cron_tests（真实鉴权/路由/租户投递边界） |
| cron_tests.rs / `cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required` | 宿主 cron::cron_tests（真实鉴权/路由/租户投递边界） |
| cron_tests.rs / `cron_handlers_manage_shell_job_lifecycle` | gateway_cron::host_migration_9_tests::lifecycle |
| cron_tests.rs / `cron_add_handler_covers_agent_and_bad_request_branches` | gateway_cron::host_migration_9_tests::lifecycle |
| cron_tests.rs / `cron_update_and_delete_handlers_report_errors` | gateway_cron::host_migration_9_tests::lifecycle |
| cron_tests.rs / `cron_update_handler_reports_missing_job_after_valid_patch` | gateway_cron::host_migration_9_tests::lifecycle |
| cron_tests.rs / `cron_history_endpoint_requires_auth` | 宿主 cron::cron_tests（真实鉴权/路由/租户投递边界） |
| cron_tests.rs / `cron_history_endpoint_merges_active_and_finished` | gateway_cron::host_migration_9_tests::history |
| cron_tests.rs / `cron_history_endpoint_reports_finished_window_and_workspace_isolation` | gateway_cron::host_migration_9_tests::history |
| cron_tests.rs / `cron_history_endpoint_respects_retention_and_cascade_deletion` | gateway_cron::host_migration_9_tests::history |
| cron_tests.rs / `cron_wecom_delivery_add_rejects_invalid_target_without_creating_job` | gateway_cron::host_migration_9_tests::delivery |
| cron_tests.rs / `cron_wecom_delivery_add_requires_configuration_but_none_does_not` | gateway_cron::host_migration_9_tests::delivery |
| cron_tests.rs / `cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target` | 宿主 cron::cron_tests（真实鉴权/路由/租户投递边界） |
| cron_schedule_tests.rs / `wb_schedule_api_timezone_round_trip` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_biweekly_round_trip` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_rejects_conflicting_at` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_rejects_conflicting_windows` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_rejects_invalid_timezone` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_preview_matches_persisted_schedule` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_preview_requires_authentication` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_local_at_converts_and_returns_one_candidate` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_local_at_rejects_dst_gap_and_overlap` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_date_window_uses_next_local_midnight` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_window_patch_and_timezone_only_update` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_rejects_invalid_parameters_and_empty_windows` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_date_boundary_rejects_missing_and_ambiguous_midnight` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_reads_ended_jobs` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_preview_routes_enforce_auth_and_return_candidates` | 宿主 cron::cron_tests（真实鉴权/路由/租户投递边界） |
| cron_schedule_tests.rs / `wb_schedule_api_calendar_cron_and_every_share_domain_candidates` | gateway_cron::host_migration_9_tests::schedule |
| cron_schedule_tests.rs / `wb_schedule_api_local_at_patch_requires_timezone` | gateway_cron::host_migration_9_tests::schedule |

领域 history_request 改为直接调用 handle_api_cron_history，三个历史合并/窗口/保留删除测试断言不变；history 路由鉴权仍用真实 client_router。其余迁出函数仅改类型/路径及最小夹具。原 state_with_config 的 AppState 夹具只留宿主；temp_config/response_json/add_body/update_body 按各自真实调用局部保留；job 与两个 wb helper 归领域。不抽取通用 Host 框架。

## 未来验证（本轮全部未运行）

- `python3 scripts/cargo_slot.py cargo test -p gateway-cron --lib host_migration_9_tests -- --test-threads=1`：静态预期35条，须实际 ran>0、failed=0。
- `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib gateway::api::handlers::cron::cron_tests -- --test-threads=1`：静态预期5条，仅宿主边界；先用同过滤加 `--list` 核实35条领域旧挂载消失，不能以零测试认定通过。若真实宿主目标前缀不同，先据实际 list 确认完整名再运行。
- 生产适配未改变；未来同时确认真实路由鉴权与配置回读。当前没有编译耗时或行为通过证据。


## 收口恢复：继续分离断言驱动

任务59 recovery-priorities 的 owner9 五个符号全部处理。此段替代上文“宿主保留完整断言”的最终归属说明；第一轮35条自注册领域测试不变。

- `cron_handlers_reject_requests_when_pairing_is_required`、`cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required`：完整断言移至 `gateway_cron::host_contracts::denial`，宿主同名测试将真实 `cron_state(state_with_config(..., true))` 注入。真实 PairingGuard 与 auth::require_auth 不替换。
- `cron_history_endpoint_requires_auth`、`wb_schedule_api_preview_routes_enforce_auth_and_return_candidates`：请求与断言移至 `host_contracts::routes`，宿主同名测试注入真实 client_router 的 Router。双别名、租户请求头、鉴权拒绝/允许、候选数、持久化无副作用全部保留；领域不伪造路由/中间件。
- `cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target`：请求数据、合并目标/敏感信息/持久化断言移至 `host_contracts::delivery`；宿主保留原租户作用域、真实配置保存、scheduler::Host 注入。配置数据构造归 delivery；没有模拟 latest_config 或 webhook 校验。

五条仍为宿主测试注册，调用领域断言驱动，**不是新增独立领域测试**。领域提供 feature=test-support 的三个明确行为模块，仅接受既有 CronState/Router，无 AppState 克隆和反向依赖。共23个 assert 调用完整保留（源码静态核对），新增 test-support 与可选 tower/util 仅供真实路由契约使用；vw-agent dev-dependency 启用。生产调用路径与权限未变。

本轮宿主 cron_tests.rs 307→109 行，净减198行；cron.rs 44行保持真实投影及七个适配入口。当前宿主文件族153行，相对原1507行净减1354行（89.85%）。领域生产业务新增0行，增加测试支持模块及功能注册；主模块持有请求夹具，denial/delivery/routes 各持有对应行为断言，不搬成新单体。

未来命令仍为上文 gateway-cron 的 host_migration_9_tests（静态35条）与 vw-agent 的 cron::cron_tests（静态5条）；后者会执行三个领域驱动模块，需用真实宿主确认投影/路由/配置回读。测试支持 feature 在宿主测试构建启用。本轮仍未运行任何 Cargo、构建、测试或可执行产物，未声称编译或行为通过。编号任务卡上一轮已删除，不重建。
