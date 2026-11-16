# 任务12 静态交付证据

已将工作流完整生命周期接入 desktop-workflow-ui::controller，页面组合接入 desktop-workflow-ui::page。静态交付完成；未运行任何 Cargo 命令、编译、测试、绘制或实机验收。

## 实际调用链与依赖

App.workflow_state 原已是 desktop-workflow-state::WorkflowState，无需重复改字段或初始化。宿主 workflow::update → message::update → controller::update → video_edit/app/run/node/document/canvas，沿用已有 message 子模块与 effects 的真实实现。保存/加载使用 gateway-workflow；运行请求沿用现有 WorkflowRunSubscriptionRequest 与 workflow_run_stream，不添加平行执行器。视图 workflow::view → page::view → 既有 view/app_editor 控件；页面组合与补充节点面板归库，原视图源文件全部删除。

Context 仅借用 WorkflowState，并接收窗口尺寸、当前租户及网关工厂；不引用 App。同步导航请求经 sync_top_tab 标记回写；异步剪贴板重置计时、toast、原生聊天交接用具名 Effect 映射。PageMessage 保留帮助文本选择、复制、上下文菜单和外链事件，宿主映射到原控件消息。未新增 Cargo 依赖，无宿主源码反向引用、include! 或反向 path 注册。

## 残留职责逐组

- workflow/mod.rs：sync_top_tab/refresh_top_tab_title 管理跨域 AppTab；ensure_initialized 连接租户与首次加载生命周期；view/update 为装配转发。
- message/mod.rs：update 只构造局部借用、映射导航与宿主效果；sync_video_tenant 传当前账号租户；load_saved_apps_task/workflow_gateway_client 注入运行时网关；start_native_video_edit 保留项目、ACP、附件、草稿检查及已有聊天队列/权限交接，不把全局会话搬入库。
- view/mod.rs：PageMessage → 桌面 Message 的映射。
- canvas/mod.rs：重导出画布并投影事件；全部 CanvasMessage 分支已移到库 canvas_message。
- model/state：仅重导出既有领域类型。
- mod_tests/tests：保留宿主导航、初始化、消息与视图装配契约及模块挂载；不保留领域夹具。
- message/video_edit_tests：只留原生会话队列、ACP/full_access 权限契约，直接验证宿主交接边界。
- 无新增宿主替代文件。原页面辅助函数和测试全部迁出；controller 复用已有领域处理器，不另建算法。

## 行数审计

pathlib.read_text().splitlines，含空白注释，生产候选沿 baseline 文件标记：来源 5329 → 341，宿主净减 4988；生产候选 1847 → 192，候选净减 1655。候选不等同精确生产量；单列测试迁移，不把其作为生产迁出。

|来源|基线|当前|归属/保留|
|---|---:|---:|---|
|crates/vw-desktop/src/apps/workflow/canvas/mod.rs|26|5|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/message/app.rs|156|0|controller 或 host_module_12_tests/app.rs|
|crates/vw-desktop/src/apps/workflow/message/app_tests.rs|410|0|controller 或 host_module_12_tests/app_tests.rs|
|crates/vw-desktop/src/apps/workflow/message/canvas.rs|6|0|controller 或 host_module_12_tests/canvas.rs|
|crates/vw-desktop/src/apps/workflow/message/document.rs|89|0|controller 或 host_module_12_tests/document.rs|
|crates/vw-desktop/src/apps/workflow/message/document_tests.rs|164|0|controller 或 host_module_12_tests/document_tests.rs|
|crates/vw-desktop/src/apps/workflow/message/mod.rs|134|88|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/message/mod_tests.rs|185|0|controller 或 host_module_12_tests/mod_tests.rs|
|crates/vw-desktop/src/apps/workflow/message/node.rs|6|0|controller 或 host_module_12_tests/node.rs|
|crates/vw-desktop/src/apps/workflow/message/run.rs|81|0|controller 或 host_module_12_tests/run.rs|
|crates/vw-desktop/src/apps/workflow/message/run_tests.rs|62|0|controller 或 host_module_12_tests/run_tests.rs|
|crates/vw-desktop/src/apps/workflow/message/tests.rs|8|0|controller 或 host_module_12_tests/tests.rs|
|crates/vw-desktop/src/apps/workflow/message/video_edit.rs|279|0|controller 或 host_module_12_tests/video_edit.rs|
|crates/vw-desktop/src/apps/workflow/message/video_edit_tests.rs|121|27|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/mod.rs|74|74|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/mod_tests.rs|101|114|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/model/mod.rs|2|2|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/state/mod.rs|2|2|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/tests.rs|8|8|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/view/app_editor.rs|24|0|page/app_editor.rs|
|crates/vw-desktop/src/apps/workflow/view/app_editor_ui_refresh_38_tests.rs|834|0|page/app_editor_ui_refresh_38_tests.rs|
|crates/vw-desktop/src/apps/workflow/view/app_list.rs|24|0|page/app_list.rs|
|crates/vw-desktop/src/apps/workflow/view/canvas_shell.rs|13|0|page/canvas_shell.rs|
|crates/vw-desktop/src/apps/workflow/view/data_nodes.rs|41|0|page/data_nodes.rs|
|crates/vw-desktop/src/apps/workflow/view/data_nodes_ui_refresh_37_tests.rs|570|0|page/data_nodes_ui_refresh_37_tests.rs|
|crates/vw-desktop/src/apps/workflow/view/http_request.rs|27|0|page/http_request.rs|
|crates/vw-desktop/src/apps/workflow/view/if_else.rs|25|0|page/if_else.rs|
|crates/vw-desktop/src/apps/workflow/view/menu.rs|49|0|page/menu.rs|
|crates/vw-desktop/src/apps/workflow/view/mod.rs|128|21|装配/跨域契约|
|crates/vw-desktop/src/apps/workflow/view/mod_ui_refresh_36_tests.rs|430|0|page/mod_ui_refresh_36_tests.rs|
|crates/vw-desktop/src/apps/workflow/view/next_step.rs|18|0|page/next_step.rs|
|crates/vw-desktop/src/apps/workflow/view/node_editor.rs|50|0|page/node_editor.rs|
|crates/vw-desktop/src/apps/workflow/view/node_editor_fields.rs|2|0|page/node_editor_fields.rs|
|crates/vw-desktop/src/apps/workflow/view/node_visual.rs|119|0|page/node_visual.rs|
|crates/vw-desktop/src/apps/workflow/view/node_visual_integrations.rs|126|0|page/node_visual_integrations.rs|
|crates/vw-desktop/src/apps/workflow/view/node_visual_tests.rs|291|0|page/node_visual_tests.rs|
|crates/vw-desktop/src/apps/workflow/view/run_preview.rs|47|0|page/run_preview.rs|
|crates/vw-desktop/src/apps/workflow/view/start_inputs.rs|54|0|page/start_inputs.rs|
|crates/vw-desktop/src/apps/workflow/view/start_meta.rs|61|0|page/start_meta.rs|
|crates/vw-desktop/src/apps/workflow/view/start_panel.rs|52|0|page/start_panel.rs|
|crates/vw-desktop/src/apps/workflow/view/styles.rs|27|0|page/styles.rs|
|crates/vw-desktop/src/apps/workflow/view/tests.rs|298|0|page/tests.rs|
|crates/vw-desktop/src/apps/workflow/view/toolbar.rs|39|0|page/toolbar.rs|
|crates/vw-desktop/src/apps/workflow/view/variables.rs|60|0|page/variables.rs|
|crates/vw-desktop/src/apps/workflow/view/video_edit.rs|6|0|page/video_edit.rs|

## 原测试与断言映射

以下按原模块逐函数保留测试名及断言；App 夹具替换为仅含 WorkflowState/窗口尺寸的 Fixture。唯一跨域拆分是原 apply_loaded 的 screen/tab 两条断言移到宿主 loaded_workflow_navigates_to_workflow_tab，其余标题、视口与位置断言仍在领域，新增真实导航标记断言。

- `app_tests.rs::saved_app_list_messages_update_loading_search_menu_and_delete_state` → `host_module_12_tests::app_tests::saved_app_list_messages_update_loading_search_menu_and_delete_state`。
- `app_tests.rs::saved_app_errors_and_copy_state_are_recorded` → `host_module_12_tests::app_tests::saved_app_errors_and_copy_state_are_recorded`。
- `app_tests.rs::open_saved_app_selects_cached_entry_or_marks_pending_open` → `host_module_12_tests::app_tests::open_saved_app_selects_cached_entry_or_marks_pending_open`。
- `app_tests.rs::open_saved_app_finished_applies_loaded_workflow_and_clears_pending_state` → `host_module_12_tests::app_tests::open_saved_app_finished_applies_loaded_workflow_and_clears_pending_state`。
- `app_tests.rs::open_file_finished_handles_loaded_cancel_and_error_results` → `host_module_12_tests::app_tests::open_file_finished_handles_loaded_cancel_and_error_results`。
- `app_tests.rs::show_saved_apps_loads_once_and_closes_active_panels` → `host_module_12_tests::app_tests::show_saved_apps_loads_once_and_closes_active_panels`。
- `app_tests.rs::inline_yaml_opens_valid_preview_and_reports_invalid_yaml` → `host_module_12_tests::app_tests::inline_yaml_opens_valid_preview_and_reports_invalid_yaml`。
- `app_tests.rs::inline_yaml_reports_missing_nonempty_focus_node_after_loading_preview` → `host_module_12_tests::app_tests::inline_yaml_reports_missing_nonempty_focus_node_after_loading_preview`。
- `app_tests.rs::open_file_closes_floating_panels_before_returning_task` → `host_module_12_tests::app_tests::open_file_closes_floating_panels_before_returning_task`。
- `app_tests.rs::app_editor_messages_edit_and_submit_create_flow` → `host_module_12_tests::app_tests::app_editor_messages_edit_and_submit_create_flow`。
- `app_tests.rs::submit_app_editor_without_editor_is_noop_and_invalid_input_sets_error` → `host_module_12_tests::app_tests::submit_app_editor_without_editor_is_noop_and_invalid_input_sets_error`。
- `app_tests.rs::delete_saved_app_finished_clears_active_app_when_removed_entry_was_active` → `host_module_12_tests::app_tests::delete_saved_app_finished_clears_active_app_when_removed_entry_was_active`。
- `app_tests.rs::organize_active_app_reports_missing_active_and_updates_loaded_document` → `host_module_12_tests::app_tests::organize_active_app_reports_missing_active_and_updates_loaded_document`。
- `app_tests.rs::app_handle_returns_none_for_non_app_messages` → `host_module_12_tests::app_tests::app_handle_returns_none_for_non_app_messages`。
- `document_tests.rs::test_module_is_wired` → `host_module_12_tests::document_tests::test_module_is_wired`。
- `document_tests.rs::save_reload_export_and_floating_panel_messages_update_state` → `host_module_12_tests::document_tests::save_reload_export_and_floating_panel_messages_update_state`。
- `mod_tests.rs::test_module_is_wired` → `host_module_12_tests::mod_tests::test_module_is_wired`。
- `mod_tests.rs::update_dispatches_to_app_node_document_and_canvas_handlers` → `host_module_12_tests::mod_tests::update_dispatches_to_app_node_document_and_canvas_handlers`。
- `mod_tests.rs::update_dispatches_run_preview_retry_and_cancel_messages` → `host_module_12_tests::mod_tests::update_dispatches_run_preview_retry_and_cancel_messages`。
- `mod_tests.rs::apply_loaded_syncs_top_tab_and_suggested_position_accounts_for_viewport` → `host_module_12_tests::mod_tests::apply_loaded_syncs_top_tab_and_suggested_position_accounts_for_viewport`。
- `mod_tests.rs::helper_functions_cover_save_export_and_record_mapping` → `host_module_12_tests::mod_tests::helper_functions_cover_save_export_and_record_mapping`。
- `mod_tests.rs::suggested_node_position_clamps_zero_zoom_and_offsets_by_node_count` → `host_module_12_tests::mod_tests::suggested_node_position_clamps_zero_zoom_and_offsets_by_node_count`。
- `run_tests.rs::handle_submit_saved_app_run_prompt_noops_without_prompt_and_reports_loading_or_invalid_inputs` → `host_module_12_tests::run_tests::handle_submit_saved_app_run_prompt_noops_without_prompt_and_reports_loading_or_invalid_inputs`。
- `run_tests.rs::handle_cancel_prompt_and_ignores_finished_event_for_other_uuid` → `host_module_12_tests::run_tests::handle_cancel_prompt_and_ignores_finished_event_for_other_uuid`。
- `tests.rs::test_module_is_wired` → `host_module_12_tests::tests::test_module_is_wired`。
- `video_edit_tests.rs::video_edit_ui_refresh_and_reopen_preserve_unsaved_draft` → `host_module_12_tests::video_edit_tests::video_edit_ui_refresh_and_reopen_preserve_unsaved_draft`。
- `video_edit_tests.rs::video_edit_ui_stale_tenant_reply_cannot_replace_current_template` → `host_module_12_tests::video_edit_tests::video_edit_ui_stale_tenant_reply_cannot_replace_current_template`。
- `video_edit_tests.rs::video_edit_ui_builtin_run_and_delete_use_safe_actions` → `host_module_12_tests::video_edit_tests::video_edit_ui_builtin_run_and_delete_use_safe_actions`。
- `video_edit_tests.rs::video_edit_ui_start_uses_existing_native_chat_queue_and_permissions` → `宿主 message::video_edit_tests（原生交接）::video_edit_ui_start_uses_existing_native_chat_queue_and_permissions`。
- `video_edit_tests.rs::video_edit_ui_restore_requires_confirmation_and_cancel_keeps_yaml` → `host_module_12_tests::video_edit_tests::video_edit_ui_restore_requires_confirmation_and_cancel_keeps_yaml`。
- `mod_tests.rs::test_module_is_wired` → `宿主 workflow::mod_tests::test_module_is_wired`。
- `mod_tests.rs::sync_top_tab_replaces_apps_tab_and_updates_existing_workflow_tab` → `宿主 workflow::mod_tests::sync_top_tab_replaces_apps_tab_and_updates_existing_workflow_tab`。
- `mod_tests.rs::sync_top_tab_adds_workflow_tab_when_missing` → `宿主 workflow::mod_tests::sync_top_tab_adds_workflow_tab_when_missing`。
- `mod_tests.rs::ensure_initialized_starts_saved_apps_load_once` → `宿主 workflow::mod_tests::ensure_initialized_starts_saved_apps_load_once`。
- `mod_tests.rs::view_builds_workflow_element_from_app_state` → `宿主 workflow::mod_tests::view_builds_workflow_element_from_app_state`。
- `mod_tests.rs::update_forwards_messages_to_workflow_message_module` → `宿主 workflow::mod_tests::update_forwards_messages_to_workflow_message_module`。
- `tests.rs::test_module_is_wired` → `宿主 workflow::tests::test_module_is_wired`。
- `node_visual_tests.rs::node_visual_section_returns_none_without_visual_draft` → `page::node_visual_tests::node_visual_section_returns_none_without_visual_draft`。
- `node_visual_tests.rs::node_visual_section_builds_start_visual` → `page::node_visual_tests::node_visual_section_builds_start_visual`。
- `node_visual_tests.rs::node_visual_section_builds_llm_visual` → `page::node_visual_tests::node_visual_section_builds_llm_visual`。
- `node_visual_tests.rs::node_visual_section_builds_with_validation_errors` → `page::node_visual_tests::node_visual_section_builds_with_validation_errors`。
- `node_visual_tests.rs::node_visual_section_builds_answer_visual` → `page::node_visual_tests::node_visual_section_builds_answer_visual`。
- `node_visual_tests.rs::node_visual_section_dispatches_by_visual_draft_instead_of_block_type` → `page::node_visual_tests::node_visual_section_dispatches_by_visual_draft_instead_of_block_type`。
- `node_visual_tests.rs::node_visual_section_builds_if_else_visual` → `page::node_visual_tests::node_visual_section_builds_if_else_visual`。
- `node_visual_tests.rs::node_visual_section_builds_empty_if_else_visual` → `page::node_visual_tests::node_visual_section_builds_empty_if_else_visual`。
- `node_visual_tests.rs::node_visual_section_builds_knowledge_retrieval_visual` → `page::node_visual_tests::node_visual_section_builds_knowledge_retrieval_visual`。
- `node_visual_tests.rs::node_visual_section_builds_tool_visual` → `page::node_visual_tests::node_visual_section_builds_tool_visual`。
- `node_visual_tests.rs::node_visual_section_builds_agent_visual` → `page::node_visual_tests::node_visual_section_builds_agent_visual`。
- `node_visual_tests.rs::node_visual_section_builds_code_visual` → `page::node_visual_tests::node_visual_section_builds_code_visual`。
- `tests.rs::module_test_anchor` → `page::tests::module_test_anchor`。
- `tests.rs::view_builds_saved_app_layers_without_active_app` → `page::tests::view_builds_saved_app_layers_without_active_app`。
- `tests.rs::view_builds_active_app_layers_for_empty_and_node_documents` → `page::tests::view_builds_active_app_layers_for_empty_and_node_documents`。
- `tests.rs::view_builds_saved_app_run_prompt_detail_and_preview_with_copied_error` → `page::tests::view_builds_saved_app_run_prompt_detail_and_preview_with_copied_error`。
- `tests.rs::view_builds_active_app_node_and_variable_editor_layers` → `page::tests::view_builds_active_app_node_and_variable_editor_layers`。
- `tests.rs::view_builds_start_variable_editor_layer` → `page::tests::view_builds_start_variable_editor_layer`。

原生交接用例改为直接调用 start_native_video_edit，保留队列、query、acp_test、full_access_enabled 和 running_saved_app_uuid 断言。UI 测试保持全部中文内容、暗黑布局、事件、真实 draw/screenshot 路径，字体载入改为 desktop_assets::fonts::load_all。

前轮 desktop_split_21_tests 保留：其断言针对既有局部处理器/基础节点段落；本轮原 app/document/run 测试验证完整生命周期分发，page 测试验证带补充面板的组合入口。未以同名推定等价删除断言，未复制其夹具到宿主。相交的节点草稿断言分别作用于基础段落与完整页面组合；本轮没有删除旧行为覆盖。

## 静态检查与以后验证

已用独立 rustfmt 解析本次新迁文件并格式化；git diff --check 检查本任务范围。已读取并核对 mod/cfg、局部类型引用、真实调用者及清单；不代表类型检查通过。

以后经用户授权才可执行：
- python3 scripts/cargo_slot.py cargo test -p desktop-workflow-ui --lib host_module_12_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo test -p desktop-workflow-ui --lib page:: -- --test-threads=1
- python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib

风险：未编译、未运行迁移后的用例及字体整形；异步 HostEffect 需由宿主事件循环消费，原生聊天与 toast 行为尚无运行证据。未来应验证租户切换、模板版本冲突、原生队列权限以及暗黑中文绘制。

静态测试名称清点：对 HEAD 全部来源的 #[test] 函数名多重集与迁移后领域/宿主源码核对，无遗失；未执行测试。
