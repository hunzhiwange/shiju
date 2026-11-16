# 任务19 静态交付证据

已完成整组归属收口；本轮严格未运行 Cargo、测试、绘制或回归。静态交付不代表编译或行为通过。

## 生产与接线
- 真实调用链：settings 路由 → cron/heartbeat/scheduler 宿主 Inputs 装配 → desktop-settings-cron::update；view/overlays → 领域 ViewData。App 持有既有领域 CronSettingsState、HeartbeatSettingsState、SchedulerSettingsState，初始化使用领域 build_*，无需重复改写共享状态。
- 表单、日期选择、列表、模板、历史、delivery 校验、保存失败/回读及心跳调度配置已由既有领域生产入口拥有，本任务沿用，不复制算法。
- 修复 cron_mapping 的 CronScheduleDraftChanged 双向映射：删除不存在的 crate/state 字段，与两端实际 editing/draft 定义一致。
- 宿主剩余函数：availability（读取 channels 配置）、data（项目/代理/提供商投影）、map（跨域消息与投递配置弹窗映射）、help_content（聊天选择控件接口）、view（调度页组合）、view_overlays；均为装配。CRON_TABS 为搜索页描述符投影，date_time 为公共类型转出。
- update 壳保留配置访问、网关、通知映射；cron_history 保留系统设置可见性与工作区上下文协调、Task 映射；heartbeat/scheduler 仅注入配置访问与返回消息。真实系统服务未迁移、依赖方向未反转。

## 断言映射
- `tabs_and_view_cover_loading_empty_jobs_config_and_add_forms` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/views.rs`
- `overlays_cover_no_modal_runs_modal_states_and_help` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/views.rs`
- `add_tab_covers_shell_every_and_agent_delivery_with_provider_model_options` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/views.rs`
- `runs_modal_falls_back_to_job_id_and_wraps_help_after_runs_modal` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/views.rs`
- `cron_project_selector_clears_selected_path` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `ui_refresh_27_layout_and_interaction` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `ui_refresh_27_preserves_values_and_disabled_states` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `ui_refresh_27_history_scroll_and_job_actions` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `settings_tabs_core_appearance_cron` → `crates/vw-desktop/src/app/components/system_settings_cron_ui_refresh_27_tests.rs（宿主跨域契约）`
- `help_rollout_15_renders_contextual_help` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `help_rollout_15_heartbeat_preserves_status_and_actions` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `help_rollout_15_cron_add_config_and_scheduler_render_help` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `settings_help_gap_10_add_shell_hides_task_type_explanation` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `settings_help_gap_10_edit_agent_hides_fallback_explanation` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `settings_help_gap_10_advanced_hides_workday_example` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `cron_agent_and_model_choices_render_below_project` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `cron_edit_form_omits_separate_acp_selector` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `cron_modal_renders_add_and_edit` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `cron_template_acceptance_modal_sizes_and_cancel` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `cron_template_acceptance_card_columns_and_selection` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/rendering.rs`
- `wb_schedule_feedback_unconfigured_channel_is_explicit_error` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/controller.rs`
- `wb_schedule_feedback_save_error_preserves_draft_and_shows_toast` → `crates/vw-desktop/src/app/message/settings/cron_tests.rs（宿主跨域契约）`
- `cron_history_refresh_error_preserves_rows` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/controller.rs`
- `cron_history_close_and_workspace_change_invalidate_responses` → `crates/vw-desktop/src/app/message/settings/cron_tests.rs（宿主跨域契约）`
- `cron_history_repeated_polling_errors_do_not_repeat_toast` → `crates/vw-desktop/src/app/message/settings/cron_tests.rs（宿主跨域契约）`
- `cron_wecom_form_requires_credentials_and_explicit_target` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/controller.rs`
- `cron_wecom_form_configuration_cancel_preserves_both_drafts` → `crates/vw-desktop/src/app/message/settings/cron_tests.rs（宿主跨域契约）`
- `cron_history_stale_response_is_ignored` → `crates/libs/desktop-settings-cron/src/host_module_19_tests/controller.rs`

领域 views 保留4项原场景与断言，提供商夹具改为真实 ViewData 所消费的 id/models，移除未被视图消费的模型元数据。rendering 保留原布局、字体、绘制、截图、事件、中文与明暗主题断言；心跳场景直接调用真实心跳领域入口。General/Cron 页签外观对照仍留宿主。controller 覆盖原历史错误、过期回包及投递断言，直接调用真实 update/delivery。旧 desktop_split_* 测试未重写，其场景与本组保留的测试名称和完整断言不同，不按名称推断等价删除。

宿主测试保留理由：toast 与重复通知验证真实通知映射；关闭页面/切换工作区验证 App 导航协调；WeCom 凭据投影及配置取消验证 channels 跨域桥接。仅这些契约继续使用 App；领域测试全部改为局部 Fixture，不复制 App。

## 行数（baseline splitlines 同口径）
| 来源 | 基线 | 当前 | 减少 |
|---|---:|---:|---:|
| crates/vw-desktop/src/app/components/system_settings_cron.rs | 155 | 226 | -71 |
| crates/vw-desktop/src/app/components/system_settings_cron_tests.rs | 188 | 0 | 188 |
| crates/vw-desktop/src/app/components/system_settings_cron_ui_refresh_27_tests.rs | 1407 | 24 | 1383 |
| crates/vw-desktop/src/app/message/settings/cron.rs | 23 | 23 | 0 |
| crates/vw-desktop/src/app/message/settings/cron_history.rs | 28 | 28 | 0 |
| crates/vw-desktop/src/app/message/settings/cron_mapping.rs | 228 | 228 | 0 |
| crates/vw-desktop/src/app/message/settings/cron_tests.rs | 269 | 199 | 70 |
| crates/vw-desktop/src/app/message/settings/heartbeat.rs | 12 | 12 | 0 |
| crates/vw-desktop/src/app/message/settings/scheduler.rs | 12 | 12 | 0 |

来源合计 2322 → 752，净减少 1570 行；生产候选 458 → 529，净减少 -71 行（含消息装配，非全部业务实现）。无新建宿主替代文件。领域新增源码另归库统计，不当作宿主生产减少。

## 静态检查及限制
- 对修改文件运行独立 rustfmt（skip_children），git diff --check 无空白错误；核对 lib.rs 测试注册、宿主旧挂载移除、生产消息两端字段、App 状态与初始化真实调用者。
- 新增依赖仅 dev-dependencies：已有 desktop-chat-view 供真实选择文本控件、urlencoding 供保留的帮助菜单断言；Cargo.lock 仅更新该包依赖列表。
- 原帮助回调将 copy/no-op 之外消息判为 unreachable；原绘制测试包含 OpenUrl 上下文动作断言，本轮按原实现迁移此路径，未宣称通过或混入产品行为修复。
- 未运行任何编译、测试或实际截图；类型及绘制风险待用户另行授权验证。
- 后续授权命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-cron --lib host_module_19_tests -- --test-threads=1`；宿主检查 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。
