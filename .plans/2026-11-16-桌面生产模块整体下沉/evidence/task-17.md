# 任务17实施证据（未完成）

本轮没有运行 Cargo、测试、绘制或回归；独立 rustfmt 仅解析修改文件。

## 已接线生产入口

- `models::update` → `model_update::persist_status_effect` → `prepare_status_save` → 既有 `model_tasks`；宿主只映射消息、会话自动模型偏好和滚动效果。
- `model_routes::queue_valid_routes` → 领域 `prepare_routes`；领域持有 `ModelRoutesSettingsState` 与校验反馈，宿主保留查询分类跨域回写及共享持久化调度。
- 聚合控制器沿用 `desktop-settings-apps::aggregation`，绘制调用 `desktop-apps-ui::aggregation`，没有新增反向宿主依赖。

## 断言迁移

以下整函数的断言及截图/绘制代码迁移，事件断言去掉宿主枚举包裹，改为真实局部事件：
- `aggregation_omo_tests.rs` → `crates/libs/desktop-settings-apps/src/host_module_17_tests/omo.rs`：standard, aggregation_t64_actual_text_rendering_in_both_themes
- `aggregation_pi_tests.rs` → `crates/libs/desktop-settings-apps/src/host_module_17_tests/pi.rs`：aggregation_t65_renders_dynamic_chinese_in_light_and_dark, traverse, text, text_input
- `aggregation_subagent_tests.rs` → `crates/libs/desktop-settings-apps/src/host_module_17_tests/subagent.rs`：snapshot, loaded, traverse, text, aggregation_t60_actual_text_rendering_in_both_themes
- `aggregation_review_tests.rs` → `crates/libs/desktop-settings-apps/src/host_module_17_tests/review.rs`：aggregation_t61_aggregation_review_preserves_contract, traverse, text, loaded, traverse, text, aggregation_t61_actual_text_rendering_in_both_themes

模型状态断言迁入 `desktop-settings-providers::host_module_17_tests`，复用真实 update/持久化入口：model_toggle_updates_locally_and_starts_save_without_reloading, enabling_model_updates_locally_and_schedules_save_without_forcing_auto_model, provider_set_all_only_changes_models_in_selected_group, provider_reset_optimistically_disables_only_selected_group, models_set_all_updates_every_connected_model_with_one_save, model_status_save_error_rolls_back_local_changes, model_status_save_success_keeps_optimistic_state, invalid_routes_block_persistence_and_valid_routes_clear_feedback

宿主保留模型偏好/会话同步、提供者变化到模型目录的跨域投影。toggle/enabling 的宿主用例还保留部分重复领域断言，后续须缩为跨域契约。

## 来源行数（splitlines，包含空行注释）

| 来源 | 基线 | 当前 |
|---|---:|---:|
| crates/vw-desktop/src/app/components/system_settings_embedding_routes.rs | 13 | 13 |
| crates/vw-desktop/src/app/components/system_settings_embedding_routes_ui_refresh_11_tests.rs | 922 | 922 |
| crates/vw-desktop/src/app/components/system_settings_models.rs | 252 | 252 |
| crates/vw-desktop/src/app/components/system_settings_models_tests.rs | 42 | 42 |
| crates/vw-desktop/src/app/components/system_settings_models_ui_refresh_10_tests.rs | 507 | 507 |
| crates/vw-desktop/src/app/components/system_settings_providers/catalog.rs | 12 | 12 |
| crates/vw-desktop/src/app/components/system_settings_providers/connect.rs | 7 | 7 |
| crates/vw-desktop/src/app/components/system_settings_providers/connected.rs | 4 | 4 |
| crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal.rs | 28 | 28 |
| crates/vw-desktop/src/app/components/system_settings_providers/custom_model_modal_ui_refresh_09_tests.rs | 1177 | 1177 |
| crates/vw-desktop/src/app/components/system_settings_providers/custom_provider.rs | 7 | 7 |
| crates/vw-desktop/src/app/components/system_settings_providers/custom_provider_tests.rs | 64 | 64 |
| crates/vw-desktop/src/app/message/settings/aggregation_clients.rs | 4 | 4 |
| crates/vw-desktop/src/app/message/settings/aggregation_desktop.rs | 48 | 48 |
| crates/vw-desktop/src/app/message/settings/aggregation_desktop_tests.rs | 277 | 277 |
| crates/vw-desktop/src/app/message/settings/aggregation_list.rs | 35 | 35 |
| crates/vw-desktop/src/app/message/settings/aggregation_omo.rs | 44 | 41 |
| crates/vw-desktop/src/app/message/settings/aggregation_omo_tests.rs | 91 | 0 |
| crates/vw-desktop/src/app/message/settings/aggregation_omp.rs | 35 | 35 |
| crates/vw-desktop/src/app/message/settings/aggregation_pi.rs | 42 | 42 |
| crates/vw-desktop/src/app/message/settings/aggregation_pi_tests.rs | 104 | 0 |
| crates/vw-desktop/src/app/message/settings/aggregation_publish.rs | 4 | 4 |
| crates/vw-desktop/src/app/message/settings/aggregation_review_tests.rs | 223 | 0 |
| crates/vw-desktop/src/app/message/settings/aggregation_subagent.rs | 58 | 55 |
| crates/vw-desktop/src/app/message/settings/aggregation_subagent_tests.rs | 160 | 0 |
| crates/vw-desktop/src/app/message/settings/embedding_routes.rs | 10 | 10 |
| crates/vw-desktop/src/app/message/settings/model_routes.rs | 109 | 111 |
| crates/vw-desktop/src/app/message/settings/model_routes_tests.rs | 306 | 306 |
| crates/vw-desktop/src/app/message/settings/models.rs | 110 | 109 |
| crates/vw-desktop/src/app/message/settings/models_tests.rs | 353 | 225 |

来源 5048 → 4337，净减 711 行；本任务没有新增宿主文件。生产候选另计：822 → 817 行。

## 未完成项

- custom_model_modal、models、embedding_routes 页面族的宿主测试仍包含可迁领域行为，尚未完成逐断言分离。
- Desktop 聚合绘制测试仍在宿主，需要真实映射字段 UI 的局部领域夹具。
- 全部来源逐函数残留审计尚未完成；不能将当前减量视为任务验收。
- 保留任务卡，禁止标记完成或宣称编译和测试通过。

## 静态验证

修改文件已由独立 rustfmt 解析；全工作区 diff --check 发现其他任务文件空行告警，本任务路径另行检查。后续只有用户授权后才执行任务卡中的定向验证命令。
