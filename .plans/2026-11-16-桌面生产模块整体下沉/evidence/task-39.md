# 任务39静态实施证据

本次为部分实施，尚未满足完整静态交付条件，任务卡保留。未运行任何 Cargo、测试或实机验证。`git diff --check` 已执行无诊断；独立 rustfmt 仅用于本次文件，随后收敛了大文件无关格式变化。

## 已接线的调用链

- 宿主 crate::__ → desktop_i18n::__ → $crate::format::Argument；删除宿主宏定义，保留原调用语法。
- locale 更新 → 宿主 apps 装配稳定 ID/来源标签 → desktop_i18n::apps::refresh_hub_titles；只对匹配项修改标题。
- desktop-editor::preview::lsp → desktop_i18n::editor::set_lsp_status；top_bar/gateway → 宿主局部借用映射 → desktop_i18n::editor::lsp_status；原始服务文本保留。
- search_panel / input_panel → 宿主 other::session_title → desktop_i18n::other::session_title；只读取预览与生成反馈。
- 宿主 tasks 重导出 → desktop-task-model::i18n → desktop-i18n::tasks；状态类型依赖保持由模型指向 i18n。
- App::channel_qr_flow_snapshot → ChannelsSettingsState::qr_flow_snapshot；快照类型与 accepts_result 同归 channels。
- 宿主 runtime::CostSettingsState/CostPriceInput → desktop-settings-runtime::state；默认值与全部原断言迁移。
- 宠物 active/cached chat → 显式 (assistant, content) 输入 → desktop-task-board::pet_interaction::newest_open_thinking_text。

## 尚未收口的具体缺口

- i18n 下 gettext_area_6/7/10/11/12/13/14/15/16/18、settings_models/runtime、project 测试仍含 App 夹具；其中真实跨域路由/渲染契约可保留，但纯局部验证、反馈与绘制夹具仍需逐域拆分。不能将这批测试全部认定为必要装配。
- pet_tests 的局部交互与 summary 测试仍依赖宿主夹具，需迁入 task-board 本地状态夹具；当前保留全部断言，仅适配了显式消息输入。
- pet.rs resize_task_pet_preview 的位置派生、task_pet_session_title 的选择规则仍须进一步下沉；窗口 ID、Iced Task 与运行时/会话/工作区装配可以保留。
- agent_apps.rs 的多域 invalidate 装配须逐字段核对真实职责；本次未扩大其能力或改变生命周期语义。
- App 字段组与所有重导出残留尚未完成最终逐字段审计；以下逐文件表只记录本次事实，不视为验收通过。

## 逐文件行数与归属

|来源|计划基线|本次前|本次后|当前归属/剩余工作|
|---|---:|---:|---:|---|
|`crates/vw-desktop/src/app/i18n/apps.rs`|21|21|21|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/apps_tests.rs`|21|21|0|删除原挂载，实现/断言迁往下表领域测试或共享宏|
|`crates/vw-desktop/src/app/i18n/editor.rs`|38|38|19|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs`|34|34|34|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs`|19|19|19|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs`|77|77|46|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs`|176|176|176|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs`|106|106|106|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs`|28|28|28|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs`|75|75|62|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs`|78|78|66|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs`|157|157|157|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs`|176|176|176|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs`|50|50|50|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/macro.rs`|6|6|0|删除原挂载，实现/断言迁往下表领域测试或共享宏|
|`crates/vw-desktop/src/app/i18n/mod.rs`|40|40|34|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/other.rs`|20|19|15|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/project_tests.rs`|28|28|28|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs`|222|222|222|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs`|67|67|67|保留原测试/装配，待拆分局部夹具并核对跨域契约|
|`crates/vw-desktop/src/app/i18n/tasks.rs`|25|25|1|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/i18n/tasks_tests.rs`|16|16|0|删除原挂载，实现/断言迁往下表领域测试或共享宏|
|`crates/vw-desktop/src/app/models.rs`|18|18|18|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/models_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/account.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/agent.rs`|65|39|39|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/agent_apps.rs`|88|71|71|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/agent_apps_auth.rs`|5|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/agy.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/claude.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/codex.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/desktop.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/dsh.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/edit.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/endpoints.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/env.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/gemini.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/grok.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/hermes.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/icons.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/kimi.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/leave.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/list.rs`|17|17|17|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/mcode.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/mod.rs`|40|40|40|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/mode.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/model_catalog.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/omp.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/openclaw.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/opencode.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/paths.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/pi.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/preview.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/snippets.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_providers/universal.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_state/app_aux.rs`|12|12|12|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/app_state_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/knowledge.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/pet.rs`|241|237|226|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/state/pet_tests.rs`|292|292|292|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/state/plugin_install.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/presentation.rs`|15|19|19|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/presentation_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/project_profiles.rs`|4|4|4|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/runtime_tools.rs`|4|4|4|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/acp_diagnostic.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/agents.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/aggregation_omo.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/archived_chats.rs`|1|1|1|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/channels.rs`|22|22|7|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/state/settings/computer_control.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/cron_frequency.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/cron_history.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/cron_schedule_draft.rs`|2|2|2|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/cron_templates.rs`|3|3|3|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings/infrastructure.rs`|78|44|42|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs`|31|31|0|删除原挂载，实现/断言迁往下表领域测试或共享宏|
|`crates/vw-desktop/src/app/state/settings/provider_model.rs`|42|40|38|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs`|24|24|0|删除原挂载，实现/断言迁往下表领域测试或共享宏|
|`crates/vw-desktop/src/app/state/settings/runtime.rs`|62|62|26|已改接领域；仍存宿主部分见具体缺口|
|`crates/vw-desktop/src/app/state/settings/runtime_tests.rs`|29|29|0|删除原挂载，实现/断言迁往下表领域测试或共享宏|
|`crates/vw-desktop/src/app/state/settings/worktrees.rs`|4|4|4|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings.rs`|54|54|54|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/settings_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/state/workspace.rs`|52|52|52|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/state/workspace_tests.rs`|9|9|9|原断言原位保留；需完成领域归属审计|
|`crates/vw-desktop/src/app/ui/chat.rs`|6|6|6|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/ui/mod.rs`|5|5|5|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/ui_types.rs`|115|115|115|多数为领域重导出/类型参数装配；需完成逐字段残留审计|
|`crates/vw-desktop/src/app/ui_types_tests.rs`|4|4|4|原断言原位保留；需完成领域归属审计|

任务归属文件按 splitlines：本次前 2899，本次后 2597，宿主净减少 302 行（包含测试与注释，非纯生产净减少）。未以行数作为完成条件。

## 旧测试与完整断言映射

表中原位保留表示函数及其断言未删除。迁移函数保留原名称与预期值；默认值注册哨兵只将模块后缀调整为新的真实模块名。LSP 纯测试中的 editor refresh 已从该用例去除，真实已有/新建 editor locale 事件测试仍在宿主 gettext_area_16_tests 原位保留。

|原文件与用例|新位置|
|---|---|
|`crates/vw-desktop/src/app/i18n/apps_tests.rs::desktop_gettext_area_14_hub_title_updates_after_locale_change`|`crates/libs/desktop-i18n/src/host_module_39_tests.rs::desktop_gettext_area_14_hub_title_updates_after_locale_change`|
|`crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs::desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments`|`crates/vw-desktop/src/app/i18n/gettext_area_10_tests.rs::desktop_gettext_area_10_dynamic_feedback_retains_raw_arguments`|
|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs::desktop_gettext_area_11_feedback_switches_language`|`crates/vw-desktop/src/app/i18n/gettext_area_11_tests.rs::desktop_gettext_area_11_feedback_switches_language`|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_feedback_switches_language`|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_feedback_switches_language`|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_status_log_keeps_semantics_and_raw_output`|`crates/libs/desktop-task-model/src/host_module_39_tests.rs::desktop_gettext_area_12_status_log_keeps_semantics_and_raw_output`|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_locale_selection_refreshes_cached_logs`|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_locale_selection_refreshes_cached_logs`|
|`crates/vw-desktop/src/app/i18n/gettext_area_12_tests.rs::desktop_gettext_area_12_cached_message_preserves_arguments`|`crates/libs/desktop-task-model/src/host_module_39_tests.rs::desktop_gettext_area_12_cached_message_preserves_arguments`|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_message_feedback_switches_language`|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_message_feedback_switches_language`|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors`|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_catalog_covers_display_calls_and_descriptors`|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_save_failure_toast_keeps_nested_error_semantics`|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_save_failure_toast_keeps_nested_error_semantics`|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_locale_selection_refreshes_tab_without_navigation`|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_locale_selection_refreshes_tab_without_navigation`|
|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_lifecycle_validation_keeps_template_and_identifier`|`crates/vw-desktop/src/app/i18n/gettext_area_13_tests.rs::desktop_gettext_area_13_lifecycle_validation_keeps_template_and_identifier`|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_feedback_switches_language`|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_feedback_switches_language`|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_cache_error_keeps_semantics_until_render`|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_cache_error_keeps_semantics_until_render`|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_status_display_preserves_stable_identifier`|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_status_display_preserves_stable_identifier`|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_error_metadata_rejects_stale_generation`|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_launch_error_metadata_rejects_stale_generation`|
|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_binding_rejection_keeps_display_semantics`|`crates/vw-desktop/src/app/i18n/gettext_area_14_tests.rs::desktop_gettext_area_14_binding_rejection_keeps_display_semantics`|
|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs::desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language`|`crates/vw-desktop/src/app/i18n/gettext_area_15_tests.rs::desktop_gettext_area_15_failed_skip_preserves_error_and_switches_language`|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_lsp_state_switches_without_translating_server_content`|`crates/libs/desktop-i18n/src/host_module_39_tests.rs::desktop_gettext_area_16_lsp_state_switches_without_translating_server_content`|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_locale_event_updates_existing_and_new_editors`|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_locale_event_updates_existing_and_new_editors`|
|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time`|`crates/vw-desktop/src/app/i18n/gettext_area_16_tests.rs::desktop_gettext_area_16_preview_loading_and_failure_switch_at_display_time`|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_account_labels_follow_current_locale`|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_account_labels_follow_current_locale`|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_knowledge_feedback_switches_after_creation`|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_knowledge_feedback_switches_after_creation`|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles`|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles`|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_account_validation_retains_semantic_feedback`|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_account_validation_retains_semantic_feedback`|
|`crates/vw-desktop/src/app/i18n/gettext_area_18_tests.rs::desktop_gettext_area_18_generated_session_titles_preserve_renames`|`crates/libs/desktop-i18n/src/host_module_39_tests.rs::desktop_gettext_area_18_generated_session_titles_preserve_renames`|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_dynamic_feedback_preserves_raw_arguments`|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_dynamic_feedback_preserves_raw_arguments`|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_search_preserves_chinese_navigation`|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_search_preserves_chinese_navigation`|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_mcp_validation_keeps_semantic_error`|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_mcp_validation_keeps_semantic_error`|
|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`|`crates/vw-desktop/src/app/i18n/gettext_area_6_tests.rs::desktop_gettext_area_6_settings_render_three_locales_in_both_themes`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_quick_message_error_switches_locale_without_changing_raw_error`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_quick_message_error_switches_locale_without_changing_raw_error`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_dialogue_feedback_switches_locale`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_dialogue_feedback_switches_locale`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_acp_validation_is_localized_without_changing_input`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_acp_validation_is_localized_without_changing_input`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_general_search_keeps_chinese_and_stable_keywords`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_general_search_keeps_chinese_and_stable_keywords`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_theme_delete_guard_retains_localized_feedback`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_theme_delete_guard_retains_localized_feedback`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_local_validation_feedback_switches_language`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_local_validation_feedback_switches_language`|
|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_history_rebuilds_on_locale_selection_without_translating_output`|`crates/vw-desktop/src/app/i18n/gettext_area_7_tests.rs::desktop_gettext_area_7_history_rebuilds_on_locale_selection_without_translating_output`|
|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs::desktop_gettext_area_9_existing_toast_renders_current_locale`|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs::desktop_gettext_area_9_existing_toast_renders_current_locale`|
|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs::desktop_gettext_area_9_notifications_and_errors_retain_semantic_source`|`crates/vw-desktop/src/app/i18n/gettext_area_9_tests.rs::desktop_gettext_area_9_notifications_and_errors_retain_semantic_source`|
|`crates/vw-desktop/src/app/i18n/project_tests.rs::desktop_gettext_area_10_attachment_errors_switch_after_collection`|`crates/vw-desktop/src/app/i18n/project_tests.rs::desktop_gettext_area_10_attachment_errors_switch_after_collection`|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_validation_and_sync_keep_semantics_after_switch`|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_validation_and_sync_keep_semantics_after_switch`|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_search_matches_locale_stable_ids_and_chinese_navigation`|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_search_matches_locale_stable_ids_and_chinese_navigation`|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_builds_current_pages_in_all_locales`|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_builds_current_pages_in_all_locales`|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_save_waits_for_gateway_acknowledgment`|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_save_waits_for_gateway_acknowledgment`|
|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_cached_details_translate_without_mutating_service_json`|`crates/vw-desktop/src/app/i18n/settings_models_tests.rs::desktop_i18n_settings_models_cached_details_translate_without_mutating_service_json`|
|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs::desktop_i18n_settings_runtime_views_and_search_follow_locale`|`crates/vw-desktop/src/app/i18n/settings_runtime_tests.rs::desktop_i18n_settings_runtime_views_and_search_follow_locale`|
|`crates/vw-desktop/src/app/i18n/tasks_tests.rs::desktop_gettext_area_12_split_option_keeps_value`|`crates/libs/desktop-task-model/src/host_module_39_tests.rs::desktop_gettext_area_12_split_option_keeps_value`|
|`crates/vw-desktop/src/app/models_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/models_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/app_aux_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/chrome_fields_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/core_fields_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/fields_navigation_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/fields_task_board_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/fields_workspace_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/git_fields_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/settings_preview_fields_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state/workspace_task_fields_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/app_state_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/app_state_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_ignores_visible_answer`|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_ignores_visible_answer`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_returns_none_without_open_think`|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_returns_none_without_open_think`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_skips_user_and_uses_newest_open_assistant`|`crates/vw-desktop/src/app/state/pet_tests.rs::newest_open_thinking_text_skips_user_and_uses_newest_open_assistant`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items`|`crates/vw-desktop/src/app/state/pet_tests.rs::sync_task_pet_from_runtime_adds_updates_completes_and_sorts_items`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_none_keeps_controls`|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_none_keeps_controls`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_scale_rejects_non_finite_and_bounds_values`|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_scale_rejects_non_finite_and_bounds_values`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_stack_prioritizes_running_tasks`|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_stack_prioritizes_running_tasks`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_dismiss_updates_top_card`|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_dismiss_updates_top_card`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_scaled_bounds_include_controls`|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_scaled_bounds_include_controls`|
|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_size_preview_preserves_bottom_edge_and_card_dimensions`|`crates/vw-desktop/src/app/state/pet_tests.rs::mini_companion_size_preview_preserves_bottom_edge_and_card_dimensions`|
|`crates/vw-desktop/src/app/state/presentation_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/presentation_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs::test_module_is_wired`|`crates/libs/desktop-settings-mcp/src/host_module_39_infrastructure_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/settings/infrastructure_tests.rs::mcp_settings_default_opens_servers_with_empty_runtime_state`|`crates/libs/desktop-settings-mcp/src/host_module_39_infrastructure_tests.rs::mcp_settings_default_opens_servers_with_empty_runtime_state`|
|`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs::test_module_is_wired`|`crates/libs/desktop-settings-providers/src/host_module_39_provider_model_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/settings/provider_model_tests.rs::route_classification_and_heartbeat_defaults_are_ready_to_edit`|`crates/libs/desktop-settings-providers/src/host_module_39_provider_model_tests.rs::route_classification_and_heartbeat_defaults_are_ready_to_edit`|
|`crates/vw-desktop/src/app/state/settings/runtime_tests.rs::test_module_is_wired`|`crates/libs/desktop-settings-runtime/src/host_module_39_runtime_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/settings/runtime_tests.rs::gateway_client_agents_coordination_and_cost_defaults_are_stable`|`crates/libs/desktop-settings-runtime/src/host_module_39_runtime_tests.rs::gateway_client_agents_coordination_and_cost_defaults_are_stable`|
|`crates/vw-desktop/src/app/state/settings_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/settings_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/state/workspace_tests.rs::test_module_is_wired`|`crates/vw-desktop/src/app/state/workspace_tests.rs::test_module_is_wired`|
|`crates/vw-desktop/src/app/ui_types_tests.rs::global_messages_exclude_removed_design_domain`|`crates/vw-desktop/src/app/ui_types_tests.rs::global_messages_exclude_removed_design_domain`|

## 后续验证命令（仅另行授权后执行）

- `python3 scripts/cargo_slot.py cargo test -p desktop-i18n --lib host_module_39_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-task-model --lib host_module_39_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-runtime --lib host_module_39_runtime_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-providers --lib host_module_39_provider_model_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-mcp --lib host_module_39_infrastructure_tests -- --test-threads=1`
