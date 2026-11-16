# 任务14 静态交付证据

本轮未运行 Cargo、测试、绘制或回归；以下仅为源码审查。

## 生产入口与依赖

- 复用 desktop-editor 既有 preview::{tab,editor,lsp,snapshot,selection} 和 components::preview_panel::view；没有复制宿主 App、引入反向依赖或第二套编辑算法。
- EditorMessage::TogglePopover → desktop_editor::message::editor::open_popover → 活动 Editor；清理菜单、设置、LSP 浮层并关闭命令面板。宿主只映射领域 Event。
- PreviewMessage 标签导航 → preview::tab::prepare_popover → 既有 tab::update；宿主继续处理工作区、文件树、顶层标签、网关与通知效果。
- view::editor_popovers → visible_popover → 既有 tools；菜单互斥优先级由领域维护。
- 原编辑动作、选项、搜索替换、弹层状态和设置局部断言注册为 host_module_14_tests 五个子模块；局部夹具仅持有领域参数，调用真实领域入口。
- 新增 iced_runtime 仅作为 dev-dependency，沿用锁文件已有版本，用于保留 Task 输出和中文真实 draw 路径。

## 基线同口径统计

来源 38 文件，splitlines：5668 → 4641，宿主净减少 1027 行；其中生产候选 1087 → 1002，减少 85 行。没有新增宿主替代文件。生产候选包含 cfg(test) 辅助代码，不能把候选减量全部视为生产运行逻辑。

| 来源 | 基线行 | 当前行 | 职责保留理由 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay.rs` | 40 | 40 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs` | 119 | 119 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/mod.rs` | 34 | 34 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/styles.rs` | 16 | 16 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/tabs.rs` | 15 | 15 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/content.rs` | 25 | 25 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs` | 96 | 96 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_actions.rs` | 1 | 0 | 已移入领域测试/删除失去调用者的测试转发 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_actions_tests.rs` | 197 | 0 | 已移入领域测试/删除失去调用者的测试转发 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_options.rs` | 5 | 0 | 已移入领域测试/删除失去调用者的测试转发 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_options_tests.rs` | 225 | 0 | 已移入领域测试/删除失去调用者的测试转发 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_popovers.rs` | 9 | 9 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_popovers_tests.rs` | 177 | 139 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_search_tests.rs` | 186 | 107 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/editor_tools_acceptance_tests.rs` | 114 | 114 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs.rs` | 59 | 59 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs_tests.rs` | 91 | 91 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/menus.rs` | 29 | 29 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs` | 110 | 110 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/mod.rs` | 141 | 131 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs` | 151 | 151 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/settings.rs` | 79 | 24 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs` | 162 | 68 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/view/settings_ui_refresh_40_tests.rs` | 564 | 564 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/components/preview_panel/widgets.rs` | 1 | 1 | App/Screen/WorkspaceMode/Git 面板、帮助字段和消息类型的装配投影；绘制委托领域组件 |
| `crates/vw-desktop/src/app/message/editor.rs` | 180 | 169 | 网关持久化、启动设置、Git 复制编辑器和安装通知的跨域装配；局部编辑调用领域入口 |
| `crates/vw-desktop/src/app/message/editor_tests.rs` | 646 | 646 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/message/preview/editor.rs` | 95 | 92 | App 字段借用、系统资源生命周期、通知/网关/工作区跨域效果装配；更新算法在既有领域入口 |
| `crates/vw-desktop/src/app/message/preview/editor_tests.rs` | 603 | 294 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/message/preview/lsp.rs` | 54 | 54 | App 字段借用、系统资源生命周期、通知/网关/工作区跨域效果装配；更新算法在既有领域入口 |
| `crates/vw-desktop/src/app/message/preview/lsp_tests.rs` | 334 | 334 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/message/preview/mod.rs` | 165 | 165 | App 字段借用、系统资源生命周期、通知/网关/工作区跨域效果装配；更新算法在既有领域入口 |
| `crates/vw-desktop/src/app/message/preview/search.rs` | 15 | 15 | App 字段借用、系统资源生命周期、通知/网关/工作区跨域效果装配；更新算法在既有领域入口 |
| `crates/vw-desktop/src/app/message/preview/search_tests.rs` | 40 | 40 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/message/preview/snapshot.rs` | 16 | 16 | App 字段借用、系统资源生命周期、通知/网关/工作区跨域效果装配；更新算法在既有领域入口 |
| `crates/vw-desktop/src/app/message/preview/tab.rs` | 108 | 108 | App 字段借用、系统资源生命周期、通知/网关/工作区跨域效果装配；更新算法在既有领域入口 |
| `crates/vw-desktop/src/app/message/preview/tab_tests.rs` | 673 | 673 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |
| `crates/vw-desktop/src/app/message/preview/tests.rs` | 93 | 93 | 宿主真实消息映射、App 投影与跨域组合契约；局部交互断言另见下表 |

## 旧断言归属清单

以下逐测试列出归属；同名迁移保留原断言与真实 draw/布局源码。迁移文件删除 App 启动、网关配置和子进程隔离夹具，其进程退出断言属于旧启动基础设施，未作为领域行为覆盖保留。自动保存断言改查领域 AutoSave 效果的 path/revision，宿主 700ms 延迟映射保持原实现。既有 desktop_split_08 和 preview/*_tests 覆盖基础组件与位置/存储算法，本轮迁移覆盖完整交互序列，不复制既有模块。

| 旧测试 | 新归属/保留契约 |
|---|---|
| `lsp_overlay_tests.rs::lsp_overlay_tests_are_wired` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::overlay_is_empty_without_active_preview_path` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::overlay_is_empty_when_overlay_path_does_not_match_active_path` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::overlay_is_empty_when_matching_tab_is_missing` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::overlay_builds_for_matching_active_tab` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::message_mapper_preserves_hover_direction` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::message_mapper_preserves_completion_selection_index` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_overlay_tests.rs::message_mapper_preserves_completion_controls` | `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `content_tests.rs::content_tests_are_wired` | `crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `content_tests.rs::content_shows_empty_selection_message_in_preview_screen` | `crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `content_tests.rs::content_selects_the_tab_matching_the_active_path` | `crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `content_tests.rs::content_shows_empty_selection_message_when_project_changes_are_hidden` | `crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `content_tests.rs::content_uses_git_panel_when_changes_view_is_enabled` | `crates/vw-desktop/src/app/components/preview_panel/view/content_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_actions_tests.rs::ecp02_menu_action_changes_only_active_document` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_actions_tests.rs` |
| `editor_actions_tests.rs::ecp02_mutating_actions_respect_read_only` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_actions_tests.rs` |
| `editor_actions_tests.rs::ecp02_line_action_updates_dirty_revision_and_undo` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_actions_tests.rs` |
| `editor_actions_tests.rs::ecp02_selection_multicursor_folding_and_menu_visibility` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_actions_tests.rs` |
| `editor_actions_tests.rs::ecp02_move_comment_close_menu_and_resume_editing` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_actions_tests.rs` |
| `editor_actions_tests.rs::ecp02_line_action_schedules_existing_auto_save_revision` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_actions_tests.rs` |
| `editor_options_tests.rs::ecp04_message_changes_only_selected_tab` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_options_tests.rs` |
| `editor_options_tests.rs::editor_config_app_options_keep_settings_open` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_options_tests.rs` |
| `editor_options_tests.rs::editor_config_app_wrap_column_validates_active_instance` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_options_tests.rs` |
| `editor_options_tests.rs::editor_config_app_messages_change_real_editing` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_options_tests.rs` |
| `editor_options_tests.rs::editor_config_app_renders_chinese_and_runtime_wrap` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_options_tests.rs` |
| `editor_popovers_tests.rs::ecp01_popovers_close_on_escape_outside_and_tab_switch` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_popovers_tests.rs` |
| `editor_popovers_tests.rs::ecp01_opening_popover_preserves_editor_bounds` | `crates/vw-desktop/src/app/components/preview_panel/view/editor_popovers_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_search_tests.rs::ecp06_search_closes_surrounding_panels` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_search_tests.rs` |
| `editor_search_tests.rs::ecp06_search_replace_updates_only_active_document` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_search_tests.rs` |
| `editor_search_tests.rs::ecp06_read_only_blocks_replace` | `crates/libs/desktop-editor/src/host_module_14_tests/editor_search_tests.rs` |
| `editor_search_tests.rs::ecp06_search_overlay_preserves_editor_bounds` | `crates/vw-desktop/src/app/components/preview_panel/view/editor_search_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tools_acceptance_tests.rs::ecp07_popover_layout_is_stable_across_sizes` | `crates/vw-desktop/src/app/components/preview_panel/view/editor_tools_acceptance_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `header_tabs_tests.rs::header_tabs_tests_are_wired` | `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `header_tabs_tests.rs::header_tabs_keep_open_file_entry_last_for_empty_and_multiple_tabs` | `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `header_tabs_tests.rs::tab_messages_preserve_paths_and_pointer_position` | `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `header_tabs_tests.rs::open_file_entry_reuses_existing_project_open_file_message` | `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `menus_tests.rs::menus_tests_are_wired` | `crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `menus_tests.rs::menu_ui_handles_context_menu_without_target_or_position` | `crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `menus_tests.rs::menu_ui_builds_context_menu_for_matching_tab` | `crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `menus_tests.rs::menu_ui_uses_empty_context_items_for_missing_tab` | `crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `menus_tests.rs::nav_popup_item_message_closes_popup_then_opens_target` | `crates/vw-desktop/src/app/components/preview_panel/view/menus_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::view_builds_project_preview_with_tabs_and_content` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::app_view_builds_preview_settings_overlay_when_enabled` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::view_builds_changes_layout_without_preview_header` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::view_builds_preview_without_an_active_file` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::view_builds_breadcrumb_when_active_file_has_no_matching_tab` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::view_builds_context_menu_with_and_without_anchor` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::view_builds_fullscreen_exit_controls` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::preview_fullscreen_controls_only_show_in_code_mode` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::active_preview_lsp_badge_requires_matching_active_tab` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `mod_tests.rs::active_preview_lsp_badge_builds_working_state_from_progress` | `crates/vw-desktop/src/app/components/preview_panel/view/mod_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `settings_tests.rs::settings_tests_are_wired` | `crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `settings_tests.rs::close_control_maps_to_toggle_settings` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::numeric_controls_preserve_slider_values` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::boolean_controls_preserve_enabled_state` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::language_control_preserves_selected_language` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::theme_control_preserves_selected_theme` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::selected_editor_theme_uses_manual_theme_when_system_following_is_disabled` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::selected_editor_theme_uses_app_theme_when_system_following_is_enabled` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::badge_labels_use_declared_precision_at_slider_boundaries` | `crates/libs/desktop-editor/src/host_module_14_tests/settings_tests.rs` |
| `settings_tests.rs::settings_overlay_builds_with_default_editor_settings` | `crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `settings_tests.rs::settings_overlay_builds_with_manual_line_height_and_theme` | `crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `settings_tests.rs::settings_overlay_uses_app_theme_when_following_system_theme` | `crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `settings_tests.rs::settings_overlay_keeps_base_and_modal_stack_as_separate_layers` | `crates/vw-desktop/src/app/components/preview_panel/view/settings_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::test_module_is_wired` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::toggle_settings_flips_preview_settings_panel` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::font_size_changed_updates_auto_line_height` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::font_size_changed_without_auto_line_height_preserves_height_and_clamps_modal_font` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::line_height_changed_disables_auto_adjustment` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::toggle_auto_line_height_recomputes_when_enabled` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::toggle_auto_line_height_off_keeps_current_height` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::language_changed_updates_current_language` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::language_changed_propagates_to_every_preview_editor` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::theme_changed_disables_follow_system_theme` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::toggle_follow_system_theme_updates_flag` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_actions_without_active_preview_are_noops` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_actions_with_missing_active_tab_leave_other_tabs_unchanged` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::clipboard_content_undo_and_redo_mutate_only_the_active_editor` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_metrics_preview_commits_on_latest_deadline` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_metrics_preview_preserves_latest_input_when_toggling_auto_or_closing` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_metrics_preview_commits_latest_once` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_settings_save_ignores_gateway_delay_and_persists_latest` | `crates/vw-desktop/src/app/message/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_events_update_context_target_for_click_and_drag` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::context_target_ignores_missing_active_tab_and_invalid_gutter_drag` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::context_target_drag_with_mismatched_path_starts_new_selection` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::context_target_ignores_non_pointer_editor_events` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::update_context_menu_and_fullscreen_flags_are_local` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::context_menu_open_preserves_valid_selection_and_requires_active_path` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::context_menu_edit_commands_close_menu_and_clear_transient_completion` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::preview_context_menu_custom_actions_route_before_editor_update` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::preview_context_menu_request_preserves_selection_and_clears_lsp_overlay` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::update_editor_event_sets_focus_dirty_and_autosave_revision` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::enter_uses_editor_modified_state_for_dirty_and_autosave` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::non_editing_editor_event_preserves_existing_dirty_state` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::close_search_clears_visible_completion_before_editor_focus_changes` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `editor_tests.rs::editor_mouse_hover_deadlines_follow_overlay_path_and_interactivity` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::lsp_hover_enter_enables_interaction_and_cancels_hide_deadline` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::completion_navigation_wraps_in_both_directions` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::completion_navigation_ignores_hidden_or_empty_lists` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::closing_completion_clears_only_local_overlay_state` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::editor_event_completion_left_right_and_punctuation_clear_completion` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::editor_event_mouse_release_schedules_hover_hide_when_hover_is_visible` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::completion_select_and_confirm_clear_overlay_even_when_index_is_missing` | `crates/libs/desktop-editor/src/host_module_14_tests/preview_editor_tests.rs` |
| `editor_tests.rs::completion_confirm_ignores_hidden_overlay_and_lsp_toggle_updates_status` | `crates/vw-desktop/src/app/message/preview/editor_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::test_module_is_wired` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::lsp_results_rebind_overlay_to_active_preview` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::detach_lsp_for_tab_clears_all_document_metadata` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::drain_lsp_events_without_receiver_returns_none_and_preserves_state` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::drain_lsp_events_retains_empty_receiver_when_channel_is_open` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::drain_lsp_events_handles_hover_completion_progress_and_log` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::drain_lsp_events_clears_blank_hover_and_completed_progress` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::drain_lsp_events_records_definition_with_decoded_file_path` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::drain_lsp_events_ignores_definition_with_non_file_uri` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::tick_when_disabled_only_advances_spinner` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::tick_when_enabled_processes_lsp_events` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `lsp_tests.rs::focus_and_completion_scroll_tasks_are_constructible` | `crates/vw-desktop/src/app/message/preview/lsp_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `search_tests.rs::test_module_is_wired` | `crates/vw-desktop/src/app/message/preview/search_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `search_tests.rs::update_ignores_all_routed_search_messages_without_scheduling_work` | `crates/vw-desktop/src/app/message/preview/search_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::open_existing_tab_focuses_preview_and_preserves_single_tab` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::open_existing_tab_outside_project_creates_one_selected_top_level_tab` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::open_new_instant_preview_creates_tab_and_only_schedules_preferences_save` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::open_new_text_preview_sets_loading_content_and_schedules_gateway_read` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::select_empty_path_clears_active_preview` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::select_path_sets_active_preview_and_focus` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::close_active_preview_selects_last_remaining_tab` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::close_non_active_or_missing_preview_preserves_active_path` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::close_only_active_preview_clears_active_path` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::open_loaded_updates_existing_tab_but_ignores_missing_path` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::open_loaded_matching_pending_target_clears_navigation_and_schedules_goto` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::save_file_path_rejects_missing_or_clean_tab_without_io` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::save_file_path_dirty_tab_schedules_persist_and_closes_context_menu` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::save_file_finished_success_updates_matching_tab` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::save_file_finished_keeps_newer_editor_content_dirty` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::save_file_finished_reports_success_and_failure_toasts` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::auto_save_delay_rejects_missing_target_without_scheduling_io` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::auto_save_mode_changed_updates_state_and_returns_save_task` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::path_segment_clicked_lists_directories_first_and_hides_dot_entries` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::path_segment_clicked_for_file_lists_parent_at_cursor_position` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::invalid_path_segment_dismisses_existing_popup_without_opening_another` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::close_nav_popup_clears_popup` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_right_click_replaces_other_preview_popups` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_menu_close_clears_path_and_position` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_menu_close_left_removes_tabs_before_target` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_menu_close_right_removes_tabs_after_target` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_menu_close_missing_target_only_dismisses_menu` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_menu_close_right_missing_target_only_dismisses_menu` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tab_tests.rs::tab_menu_close_all_clears_tabs_active_path_and_menu` | `crates/vw-desktop/src/app/message/preview/tab_tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::test_module_is_wired` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::update_routes_search_message_without_side_effects` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::update_routes_tab_message_to_tab_handler` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::update_routes_editor_message_to_editor_handler` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::update_routes_editor_mouse_message_to_editor_handler` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::update_routes_lsp_message_to_editor_handler` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |
| `tests.rs::tick_lsp_routes_to_lsp_tick_when_disabled` | `crates/vw-desktop/src/app/message/preview/tests.rs（保留真实宿主消息/跨域组合入口）` |

## 字段与调用者核对

- preview_tabs/active_preview_path、菜单、历史和保存 revision 继续使用领域 PreviewTab/TabState 借用入口；未新增全局状态副本。
- LSP 服务、事件接收器和系统资源仍由宿主持有，领域更新返回通知与具名效果；wasm 分支保留 cfg。
- App 的字号/主题/语言与 Git 复制弹窗、配置持久化共用，保留其跨域调用；本任务不改写公共状态初始化。
- 头部拖拽关联 Project，菜单插入关联 Chat，空内容关联 Git，安装/权限/保存关联系统与网关，保留宿主映射及相应契约。

## 静态验证与风险

- 本任务路径 git diff --check 无输出。全工作区检查有其它并行任务空行问题，未改无关文件。
- 独立 rustfmt 仅处理新增测试源码；领域根注册、旧 cfg(test) 挂载、消息分支和依赖文本经源码核对。
- 未编译、未测试；类型、渲染、字体整形、物理权限和异步行为均未获运行证据。
- 后续仅在用户授权后：python3 scripts/cargo_slot.py cargo test -p desktop-editor --lib host_module_14_tests -- --test-threads=1；宿主检查 python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib。

## 保留入口逐函数审计

- `crates/vw-desktop/src/app/components/preview_panel/lsp_overlay.rs`：`map_lsp_overlay_message`、`lsp_overlay`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/styles.rs`：`layout`、`should_show_preview_tabs_scrollbar`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/tabs.rs`：`map_message`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/view/content.rs`：`build_content_base`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/view/editor_popovers.rs`：`tools`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/view/header_tabs.rs`：`tab_drag_start_message`、`tab_drag_end_message`、`tab_close_message`、`tab_select_message`、`open_file_message`、`header_tab_item_count`、`tab_right_click_message`、`build_header_tabs`、`map_message`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/view/menus.rs`：`nav_popup_item_message`、`build_menu_ui`、`map_message`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/components/preview_panel/view/mod.rs`：`show_fullscreen_controls`、`view`、`active_preview_lsp_badge`、`map_editor_view_message`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/editor.rs`：`update`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/preview/editor.rs`：`update`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/preview/lsp.rs`：`detach_lsp_for_tab`、`sync_lsp_for_path`、`disable_lsp`、`enable_lsp`、`clear_lsp_hover`、`clear_lsp_completion`、`process_lsp_hover_timers`、`drain_lsp_events`、`focus_hover_overlay`、`completion_scroll_task`、`tick`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/preview/mod.rs`：`dismiss_preview_popup_menus`、`update`、`tick_lsp`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/preview/search.rs`：`update`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/preview/snapshot.rs`：`open`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。
- `crates/vw-desktop/src/app/message/preview/tab.rs`：`state`、`update`；App 局部借用或跨域消息/系统效果装配。无独立领域更新算法副本。

## 本轮迁移断言序号

同名用例按 assert 宏顺序 A1…An 对应原序号；中文、明暗主题和 draw 源码保留。以下仅计数源码，不是测试结果。

- `editor_search_tests.rs::ecp06_search_closes_surrounding_panels`：A1…A2 → 同名用例相同顺序断言。
- `editor_search_tests.rs::ecp06_search_replace_updates_only_active_document`：A1…A8 → 同名用例相同顺序断言。
- `editor_search_tests.rs::ecp06_read_only_blocks_replace`：A1…A3 → 同名用例相同顺序断言。
- `editor_actions_tests.rs::ecp02_menu_action_changes_only_active_document`：A1…A3 → 同名用例相同顺序断言。
- `editor_actions_tests.rs::ecp02_mutating_actions_respect_read_only`：A1…A3 → 同名用例相同顺序断言。
- `editor_actions_tests.rs::ecp02_line_action_updates_dirty_revision_and_undo`：A1…A3 → 同名用例相同顺序断言。
- `editor_actions_tests.rs::ecp02_selection_multicursor_folding_and_menu_visibility`：A1…A7 → 同名用例相同顺序断言。
- `editor_actions_tests.rs::ecp02_move_comment_close_menu_and_resume_editing`：A1…A5 → 同名用例相同顺序断言。
- `editor_actions_tests.rs::ecp02_line_action_schedules_existing_auto_save_revision`：A1…A3 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::editor_events_update_context_target_for_click_and_drag`：A1…A4 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::context_target_ignores_missing_active_tab_and_invalid_gutter_drag`：A1…A2 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::context_target_drag_with_mismatched_path_starts_new_selection`：A1…A2 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::context_target_ignores_non_pointer_editor_events`：A1…A1 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::context_menu_edit_commands_close_menu_and_clear_transient_completion`：A1…A3 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::preview_context_menu_request_preserves_selection_and_clears_lsp_overlay`：A1…A4 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::enter_uses_editor_modified_state_for_dirty_and_autosave`：A1…A3 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::editor_mouse_hover_deadlines_follow_overlay_path_and_interactivity`：A1…A4 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::lsp_hover_enter_enables_interaction_and_cancels_hide_deadline`：A1…A2 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::completion_navigation_wraps_in_both_directions`：A1…A2 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::completion_navigation_ignores_hidden_or_empty_lists`：A1…A2 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::closing_completion_clears_only_local_overlay_state`：A1…A6 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::editor_event_completion_left_right_and_punctuation_clear_completion`：A1…A3 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::editor_event_mouse_release_schedules_hover_hide_when_hover_is_visible`：A1…A2 → 同名用例相同顺序断言。
- `preview_editor_tests.rs::completion_select_and_confirm_clear_overlay_even_when_index_is_missing`：A1…A6 → 同名用例相同顺序断言。
- `editor_options_tests.rs::ecp04_message_changes_only_selected_tab`：A1…A4 → 同名用例相同顺序断言。
- `editor_options_tests.rs::editor_config_app_options_keep_settings_open`：A1…A3 → 同名用例相同顺序断言。
- `editor_options_tests.rs::editor_config_app_wrap_column_validates_active_instance`：A1…A10 → 同名用例相同顺序断言。
- `editor_options_tests.rs::editor_config_app_messages_change_real_editing`：A1…A3 → 同名用例相同顺序断言。
- `editor_options_tests.rs::editor_config_app_renders_chinese_and_runtime_wrap`：A1…A3 → 同名用例相同顺序断言。
- `editor_popovers_tests.rs::ecp01_popovers_close_on_escape_outside_and_tab_switch`：A1…A7 → 同名用例相同顺序断言。
- `settings_tests.rs::close_control_maps_to_toggle_settings`：A1…A1 → 同名用例相同顺序断言。
- `settings_tests.rs::numeric_controls_preserve_slider_values`：A1…A2 → 同名用例相同顺序断言。
- `settings_tests.rs::boolean_controls_preserve_enabled_state`：A1…A4 → 同名用例相同顺序断言。
- `settings_tests.rs::language_control_preserves_selected_language`：A1…A1 → 同名用例相同顺序断言。
- `settings_tests.rs::theme_control_preserves_selected_theme`：A1…A1 → 同名用例相同顺序断言。
- `settings_tests.rs::selected_editor_theme_uses_manual_theme_when_system_following_is_disabled`：A1…A1 → 同名用例相同顺序断言。
- `settings_tests.rs::selected_editor_theme_uses_app_theme_when_system_following_is_enabled`：A1…A1 → 同名用例相同顺序断言。
- `settings_tests.rs::badge_labels_use_declared_precision_at_slider_boundaries`：A1…A4 → 同名用例相同顺序断言。
