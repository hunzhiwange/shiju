# 任务32：部分实现与未完成缺口

本轮未编译、未测试；遵守 plan.md 禁止任何 Cargo/测试执行的要求。任务尚未满足整组测试迁移契约，保留任务卡。

## 生产调用链

- App::reload_project_chat_preferences → ProjectChatPreferences::reload → PreferencesMessage::ProjectChatPreferencesLoaded → 宿主应用模型选择；代际、项目路径和网关作用域由 loaded 判定。
- App::save_chat_model_preferences / save_automatic_model_preference → DesktopPreferencesPersistence 对应方法 → 既有 enqueue/项目 PUT 串行队列。
- App::retain_unedited_desktop_preferences / desktop_preference_edited / system_settings_edited → 领域状态查询。
- App::queue_gateway_client_snapshot → 领域构造凭据清空 patch；宿主切换认证端点。
- App::system_settings_saved → finished 推进队列 → system_settings_save_feedback；宿主提供本地化 Toast 与 DialogueFlow 跨域消息。
- 已有 ConfigAccess、CLI、cron 与文件管理入口保持原网关/系统资源装配。本轮未引入反向依赖或复制 App。

## 逐文件行数（splitlines，当前提交前 HEAD → 工作区）

| 文件 | 前 | 后 | 归属/残留 |
|---|---:|---:|---|
| config.rs | 68 | 68 | 现有装配或未完成测试拆分，见下 |
| config_agent.rs | 394 | 394 | 现有装配或未完成测试拆分，见下 |
| config_cli_bootstrap.rs | 7 | 7 | 现有装配或未完成测试拆分，见下 |
| config_cron_jobs.rs | 98 | 98 | 现有装配或未完成测试拆分，见下 |
| config_desktop.rs | 338 | 330 | 领域策略已下沉；宿主保留跨域装配 |
| config_desktop_tests.rs | 820 | 820 | 现有装配或未完成测试拆分，见下 |
| config_file_manager.rs | 1 | 1 | 现有装配或未完成测试拆分，见下 |
| config_gateway.rs | 69 | 69 | 现有装配或未完成测试拆分，见下 |
| config_gateway_tests.rs | 334 | 50 | 现有装配或未完成测试拆分，见下 |
| config_project_chat.rs | 140 | 108 | 领域策略已下沉；宿主保留跨域装配 |
| config_system_settings.rs | 217 | 193 | 领域策略已下沉；宿主保留跨域装配 |
| config_system_settings_tests.rs | 278 | 278 | 现有装配或未完成测试拆分，见下 |
| config_tests.rs | 13 | 13 | 现有装配或未完成测试拆分，见下 |

## 完整旧测试映射（名称与原断言保留）

- `config_desktop_tests.rs::config_desktop_tests_keeps_planned_coverage_targets` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::app_update_source_patch_and_restart_read_use_gateway_preferences` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::app_update_source_gateway_save_failure_reaches_unsaved_toast` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::preference_save_failure_keeps_new_edits_and_preserves_unrelated_fields` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::project_puts_keep_the_original_gateway_and_drain_after_a_failed_save` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::reasoning_and_send_preferences_preserve_unrelated_sessions_and_remove_legacy_key` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_preferences_round_trip_preserves_other_fields` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_late_load_keeps_local_edit` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_failed_save_keeps_edit_and_can_retry` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_old_endpoint_snapshot_cannot_restore_visibility` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_queued_edits_save_latest_without_reopening_window` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_defaults_and_loaded_scale_are_bounded` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_size_failure_keeps_draft_and_retry_commits_once` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_desktop_tests.rs::mini_companion_size_queued_save_waits_for_its_own_response` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_gateway_tests.rs::interactive_config_paths_reject_blocking_gateway_wrappers` → 原宿主文件；检查真实宿主交互入口不阻塞的边界契约。
- `config_gateway_tests.rs::gateway_client_endpoint_trims_auth_and_clamps_port` → desktop-config::host_module_32_tests::gateway（完整原函数及断言）。
- `config_gateway_tests.rs::gateway_client_endpoint_uses_trimmed_bearer_token_when_skey_is_blank` → desktop-config::host_module_32_tests::gateway（完整原函数及断言）。
- `config_gateway_tests.rs::gateway_client_endpoint_omits_auth_key_when_credentials_are_blank` → desktop-config::host_module_32_tests::gateway（完整原函数及断言）。
- `config_gateway_tests.rs::gateway_client_builds_from_configured_endpoint` → desktop-config::host_module_32_tests::gateway（完整原函数及断言）。
- `config_gateway_tests.rs::load_tools_list_via_gateway_returns_normalized_tools` → desktop-config::host_module_32_tests::gateway（完整原函数及断言）。
- `config_gateway_tests.rs::load_tools_list_via_gateway_returns_empty_vec_on_request_error` → desktop-config::host_module_32_tests::gateway（完整原函数及断言）。
- `config_system_settings_tests.rs::config_system_settings_tests_keeps_planned_coverage_targets` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_system_settings_tests.rs::update_gateway_client_config_persists_without_a_gateway` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_system_settings_tests.rs::agent_unified_selection_config_override_isolates_window_bootstrap` → 原宿主文件（未完成领域/跨域断言拆分）。
- `config_tests.rs::config_tests_keeps_config_module_exports_explicit` → 原宿主文件（未完成领域/跨域断言拆分）。

新增领域覆盖：late_desktop_load_preserves_edited_fields_only_in_its_scope、project_read_rejects_old_revision_and_other_project、gateway_save_feedback_requires_active_scope_and_no_existing_error。均仅编写源码，未运行。

## 未完成的实际设计缺口

config_desktop_tests.rs 中 App::new、message::view/chat/update、完整项目与宠物窗口夹具仍承载领域断言；须将队列失败/重试、reasoning 合并与项目写隔离等断言改接真实领域状态和临时服务，宿主只留跨域应用/Toast/窗口契约。目前没有将这些测试改名搬迁掩盖 App 依赖。
config_system_settings_tests.rs 中原有启动路径、局部保存断言与共享 App 夹具仍须拆开；其调用者跨 editor/settings/窗口测试，不应直接删夹具。config_tests.rs 的原注册哨兵保留。
App 偏好字段已使用 DesktopPreferencesPersistence；宠物参数和模型应用属于跨域宿主状态，当前未进一步改动。

## 静态验证与后续命令

修改文件独立 rustfmt 解析成功；聚焦 git diff --check 记录在提交前。全工作区 diff --check 发现其它任务 terminal/focus_tests.rs EOF 空行，未修改该文件。
后续用户另行授权才运行：`python3 scripts/cargo_slot.py cargo test -p desktop-config --lib host_module_32_tests -- --test-threads=1`。该命令尚不能代表未迁移测试已覆盖，必须先完成上述拆分。
