# 任务24静态交付证据

本轮遵守公共计划：未执行任何 Cargo、单元测试、实机绘制或回归。只读取源码、独立 rustfmt 解析与格式化修改文件、git diff --check，以及 pathlib splitlines/源码清单。静态交付不表示编译或行为通过。

## 生产归属与真实调用链

- App 的 McpSettingsState 通过 infrastructure.rs 别名持有 management_state 的真实类型；默认值、page 代理、mutating/modal_open 和 overlay 优先级归领域。未复制 App/AppState、未增加反向依赖。
- 宿主 mcp::update 分流管理消息；各 mcp_management_*::update 采集当前 GatewayClient、令牌、端点、可信安装/目录及导航输入，然后调用 configuration_update::{form,import,bindings,catalog,resync,tools,probe} 的真实入口。configuration_update 仅重导出既有职责模块，没有平行算法。
- form/import/bindings 原宿主状态机全部删除；表单代际、重开预设隔离、保存后刷新，导入确认、草稿、剪贴板大小与主机校验、未知结果，绑定修订/安装身份/失败重试全部在库。
- list/resync/tools 复用既有管理状态和任务；新增可见性方法由宿主和领域夹具共同调用；Error/Success/Info/Refresh 具名效果由宿主反馈及跨页刷新装配。
- management_probe::invalidate 持有清理与原授权取消逻辑；宿主只提供页面、上下文变更和 mutation 信号。diagnostic_context::current 提取既有合法项目/目录/应用/认证修订校验，并接回真实 agent_apps_diagnostics::context 调用者。
- connection/form、import、resync、tools 的局部视图投影、选项卡片、事件映射归现有库模块；宿主保留跨域面板和消息装配、真实底层 dialog、选中主机与安装观察输入。没有嵌套增加模态遮罩。
- GatewayClient 与 gateway-mcp-read/write/distribution/probe 保持真实认证和请求路径；没有 token 日志、放宽权限、配置开关或新依赖方向。新增 dev-dependencies 仅既有 desktop-assets/iced_runtime，用于原样字体与真实 draw 源码。

## 全部旧用例映射

下列45个基线用例各有唯一挂载；保留原测试名。没有删除领域断言以减少行数。

| 原用例 | 当前归属 |
| --- | --- |
| `view_builds_servers_runtime_and_json_tabs_with_state_variants` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/page.rs` |
| `mcp_error_banner_follows_mcp_tabs` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/page.rs` |
| `delete_confirmation_uses_shared_modal_overlay_and_only_shows_on_servers_tab` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/page.rs` |
| `settings_tabs_capabilities_appearance_mcp` | `crates/vw-desktop/src/app/components/system_settings_mcp_tests.rs` |
| `cc_mcp_t46_unsupported_applications_explain_native_distribution_rejection` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/bindings.rs` |
| `cc_mcp_t46_partial_and_unverified_items_cannot_report_all_success` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/bindings.rs` |
| `cc_mcp_t46_fourteen_applications_and_transport_limits_are_explicit` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/bindings.rs` |
| `cc_mcp_t46_old_context_and_duplicate_completion_cannot_replace_results` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/bindings.rs` |
| `cc_mcp_t46_single_and_bulk_use_sdk_with_revisions_and_preserve_each_target_result` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/bindings.rs` |
| `cc_mcp_t46_matrix_draws_chinese_results_and_preserves_runtime_and_acp_selections` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/bindings_controller.rs` |
| `cc_mcp_t45_preset_is_only_a_draft_with_no_consumer_enablement` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form.rs` |
| `cc_mcp_t45_modal_shapes_and_draws_chinese_and_errors_in_light_and_dark` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form.rs` |
| `cc_mcp_t45_real_sdk_writes_identical_dto_and_keeps_conflict_draft` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form.rs` |
| `cc_mcp_t45_graphical_values_edit_literals_and_references_without_losing_keys` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form.rs` |
| `cc_mcp_t45_save_completion_always_rereads_catalog_and_failure_keeps_draft` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form_controller.rs` |
| `cc_mcp_t45_transport_change_does_not_move_environment_credentials_into_headers` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form.rs` |
| `cc_mcp_t45_old_preset_response_does_not_replace_a_reopened_draft` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/form_controller.rs` |
| `cc_mcp_t47_settings_exposes_explicit_native_import_confirmation` | `crates/vw-desktop/src/app/message/settings/mcp_management_import_tests.rs` |
| `cc_mcp_t47_results_preserve_conflicts_imported_and_unchanged_items` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/import.rs` |
| `cc_mcp_t47_confirm_uses_real_sdk_and_cancel_never_posts_or_reads_local_files` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/import_controller.rs` |
| `cc_mcp_t47_clipboard_draft_requires_confirmation_and_cancellation_discards_payload` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/import_controller.rs` |
| `cc_mcp_t47_native_modal_draws_redacted_preview_and_conflicts_in_both_themes` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/import_controller.rs` |
| `cc_mcp_t47_draft_confirmation_creates_definition_only_through_gateway` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/import_controller.rs` |
| `cc_mcp_t44_chinese_catalog_shapes_and_draws_in_both_themes` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/catalog.rs` |
| `cc_mcp_t49_catalog_load_draws_unchecked_without_starting_probe` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_real_check_uses_generation_and_revision_then_edit_clears_connected` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_late_registration_after_leaving_is_revoked_with_captured_authority` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_stale_generation_revision_scope_job_and_duplicate_results_are_unchecked` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_cancel_before_registration_waits_and_error_cancelled_states_draw` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_cancel_while_registering_and_leaving_registered_job_both_revoke` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_definition_save_and_binding_mutation_clear_completed_results` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t49_cancelling_snapshot_preserves_completed_rows_until_cleanup_terminal` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/probe_controller.rs` |
| `cc_mcp_t48_settings_exposes_resync_progress` | `crates/vw-desktop/src/app/message/settings/mcp_management_resync_tests.rs` |
| `cc_mcp_t48_real_sdk_job_polls_cancels_retains_failure_and_retries_only_failed_app` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/resync_controller.rs` |
| `cc_mcp_t48_unknown_submission_and_changed_credentials_prevent_replay_and_stale_results` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/resync_controller.rs` |
| `cc_mcp_t48_incomplete_foreign_and_duplicate_reports_cannot_finish_job` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/resync_controller.rs` |
| `cc_mcp_t48_cancellation_accepts_empty_unexecuted_items_partial_commit_and_completed_job` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/resync_controller.rs` |
| `cc_mcp_t48_options_modal_draws_chinese_account_and_cancel_does_not_submit` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/resync_controller.rs` |
| `cc_mcp_t48_unverified_native_items_cannot_produce_success_feedback` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/resync_controller.rs` |
| `cc_mcp_t50_settings_exposes_tool_details_without_connecting` | `crates/vw-desktop/src/app/message/settings/mcp_management_tools_tests.rs` |
| `cc_mcp_t50_real_details_request_draws_discovered_specs_and_independent_runtime_usage` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/tools_controller.rs` |
| `cc_mcp_t50_details_reject_stale_or_duplicate_evidence_and_keep_errors_separate` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/tools_controller.rs` |
| `cc_mcp_t50_close_before_registration_cancels_with_the_original_authority` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/tools_controller.rs` |
| `cc_mcp_t50_acp_usage_reads_exact_session_and_rejects_late_or_foreign_identity` | `crates/libs/desktop-settings-mcp/src/host_module_24_tests/tools_controller.rs` |
| `configuration_refresh_maps_to_desktop_tasks` | `crates/vw-desktop/src/app/message/settings/mcp_tests.rs` |

### 断言转移与夹具说明

- 自动源码盘点所有旧用例的 assert/assert_eq/assert_ne 数量：除下述三个 ACP 跨域断言外，各用例数量一致。迁移了真实 HTTP SDK 请求、代际/修订/身份/取消/重复响应、中文明暗 draw、图形与 JSON/凭据编辑、剪贴板草稿确认、原生状态结果及失败重试断言。
- bindings matrix、import confirm、resync real SDK 各一个 `acp_settings.enabled` 隔离断言移到宿主 `managed_mcp_completion_preserves_foreign_acp_selection` 三个断言；领域更新输入不含 ACP 状态，领域原用例仍验证完整成功/失败请求及状态转换。
- resync unverified 的 ToastKind 断言在领域验证最新 Error 反馈效果；宿主 `managed_mcp_feedback_maps_error_effect_to_desktop_toast` 验证效果到真实 Toast 组件状态映射。Refresh 不改变反馈种类。
- probe 用例中的全局 App::update 路由替换为调用真实配置 reducer 后的领域失效入口；宿主 `mcp_desktop_routing_invalidates_completed_probe_during_mutation` 保留实际全局路由清理契约。
- 宿主3个只验证整页面板入口的 smoke 保留真实 App view/draw；原 settings_tabs_capabilities_appearance_mcp 保留原生截图、标签命中、真实 Message/Settings 路由及错误显示断言，属于宿主视图与原生事件的组合契约。
- 新 fixture 只有真实 MCP 聚合状态、捕获的网关/安装/项目输入、on_page 和 effects；不是复制 App。所有更新调用生产 reducer，诊断上下文与可见性也复用生产方法；只有 HTTP 边界使用本地确定性服务。没有模拟被测转换。客户端通过线程局部注入，不创建/修改宿主 bootstrap、HOME 或本地账号。宿主配置夹具仍在原配置测试中。
- 原大 App 夹具、测试挂载已从 form/bindings/probe 整组删除；import/resync/tools 仅剩通用 draw + 整页装配 smoke。

### 前轮去重核对

- desktop_split_02 两个用例只约束新服务器默认值及传输标签；本轮没有再迁/复制这些断言。
- desktop_split_14 的单个用例构造 configured/runtime/new_server 子控件，含资源/模板/提示计数与无 inspect 状态；本轮 page 用例约束完整 page 根节点、全页错误横幅增量及删除遮罩层数，断言层级不同，保留两者。
- desktop_split_29 的11个用例约束配置 reducer（loaded/format/template/add/edit/delete/save 等）及状态/Effect；本轮不复制这些断言。page 与网关管理控制器验证实际视图和独立管理请求。
- management_form_tests 的 JSON 来回切换参数/引用、未知掩码拒绝等纯模型用例原地保留；本轮新增的 graphical values 验证增删键/凭据编辑，transport 验证真实 reducer 清除旧传输凭据；并非同一套断言重复挂载。
- management_list_tests 的 scope/native 状态文本和旧 tenant 丢弃保留；本轮 catalog 用例保留中文 light/dark 原生 draw，控制器用例覆盖带令牌的真实请求及代际响应。没有仅按相近文件名删除。

## 宿主残留逐文件审计

所有残留生产函数仅处理当前端点/系统资源/跨域效果/消息投影。下表列出每个来源中的全部函数；不存在迁入新宿主替代文件的实现。

| 来源 | 旧行数→现行数 | 剩余函数及必要理由 |
| --- | --- | --- |
| `crates/vw-desktop/src/app/components/system_settings_mcp.rs` | 145→182 | event / help / catalog_event / view / view_overlays：跨域面板/设置导航描述符、选中安装观察、Message 映射与宿主原生帮助/底层 dialog 装配；领域卡片/状态逻辑在库。 |
| `crates/vw-desktop/src/app/components/system_settings_mcp_bindings.rs` | 20→20 | panel：跨域面板/设置导航描述符、选中安装观察、Message 映射与宿主原生帮助/底层 dialog 装配；领域卡片/状态逻辑在库。 |
| `crates/vw-desktop/src/app/components/system_settings_mcp_connection.rs` | 34→21 | connection_form_panel：跨域面板/设置导航描述符、选中安装观察、Message 映射与宿主原生帮助/底层 dialog 装配；领域卡片/状态逻辑在库。 |
| `crates/vw-desktop/src/app/components/system_settings_mcp_import.rs` | 21→7 | panel：跨域面板/设置导航描述符、选中安装观察、Message 映射与宿主原生帮助/底层 dialog 装配；领域卡片/状态逻辑在库。 |
| `crates/vw-desktop/src/app/components/system_settings_mcp_resync.rs` | 33→23 | panel / overlay：跨域面板/设置导航描述符、选中安装观察、Message 映射与宿主原生帮助/底层 dialog 装配；领域卡片/状态逻辑在库。 |
| `crates/vw-desktop/src/app/components/system_settings_mcp_tests.rs` | 147→44 | settings_tabs_capabilities_appearance_mcp：仅真实 App 效果、跨域状态隔离、整页装配/原生截图契约；领域断言已拆离。 |
| `crates/vw-desktop/src/app/components/system_settings_mcp_tools.rs` | 24→27 | message / panel / card：跨域面板/设置导航描述符、选中安装观察、Message 映射与宿主原生帮助/底层 dialog 装配；领域卡片/状态逻辑在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp.rs` | 106→106 | refresh_task / inspect_task / refresh_all_task / save_task / save_runtime_overrides_task / update：全局配置读写/inspect 的真实异步资源任务，Settings 路由及具名效果装配；配置算法在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_bindings.rs` | 153→23 | message / visible / update：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_bindings_tests.rs` | 371→0 | 已删除；挂载与断言迁到领域。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_form.rs` | 146→16 | message / update：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_form_tests.rs` | 330→0 | 已删除；挂载与断言迁到领域。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_import.rs` | 308→53 | message / targets / authorized / visible / open_link / update：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_import_tests.rs` | 399→54 | traverse / text / draw / cc_mcp_t47_settings_exposes_explicit_native_import_confirmation：仅真实 App 效果、跨域状态隔离、整页装配/原生截图契约；领域断言已拆离。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_list.rs` | 46→66 | context / loading / visible / update / input：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_list_tests.rs` | 74→0 | 已删除；挂载与断言迁到领域。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_probe.rs` | 45→36 | cleanup / invalidate：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_probe_tests.rs` | 494→0 | 已删除；挂载与断言迁到领域。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_resync.rs` | 27→40 | message / visible / update：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_resync_tests.rs` | 589→54 | traverse / text / draw / cc_mcp_t48_settings_exposes_resync_progress：仅真实 App 效果、跨域状态隔离、整页装配/原生截图契约；领域断言已拆离。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_tools.rs` | 38→65 | message / visible / run / invalidate / update / apply_effects：仅捕获当前客户端、认证/安装/导航输入，调用领域入口并将任务/反馈映射回宿主；可见性、状态转换与校验在库。 |
| `crates/vw-desktop/src/app/message/settings/mcp_management_tools_tests.rs` | 426→54 | traverse / text / draw / cc_mcp_t50_settings_exposes_tool_details_without_connecting：仅真实 App 效果、跨域状态隔离、整页装配/原生截图契约；领域断言已拆离。 |
| `crates/vw-desktop/src/app/message/settings/mcp_tests.rs` | 10→101 | configuration_refresh_maps_to_desktop_tasks / managed_mcp_completion_preserves_foreign_acp_selection / managed_mcp_feedback_maps_error_effect_to_desktop_toast / mcp_desktop_routing_invalidates_completed_probe_during_mutation：仅真实 App 效果、跨域状态隔离、整页装配/原生截图契约；领域断言已拆离。 |

来源总量 splitlines：3986→992，真实净减少 **2994** 行。
生产候选单列：1146→685，减少 **461** 行。测试减少没有充作生产减少。
新增宿主替代文件：0。共享 infrastructure MCP 聚合定义原地改为别名，额外减少34行；真实直接调用者 agent_apps_diagnostics 的上下文校验下沉额外减少约26行，均不混入独占来源统计。

## 静态验证与未验证风险

- 检查基线45个用例都有唯一当前模块归属，所有新领域测试从 lib.rs → host_module_24_tests 注册，原 form/bindings/probe 挂载删除。源码无指向 vw-desktop 的 path/include、crate::app 或宿主依赖。
- 已审查当前源码和已提交 660cbb961、6f93eab2d 的相关变更；当前任务差异 git diff --check 无空白错误。共享 settings/mod.rs 仅本任务 probe 测试注销进入本提交，排除其它任务改动。
- 未编译、未运行测试、未绘制/截图，不能承诺类型检查、运行行为、字体整形无 panic 或构建提速。静态迁移仍须用户另行授权后验证。

以后仅用户授权后执行：
`python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib host_module_24_tests -- --test-threads=1`
宿主直接契约：`python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::message::settings::mcp::mcp_tests -- --test-threads=1`；原生整页 smoke/截图需用户另行授权。
宿主类型检查：`python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。本轮全部未运行。
