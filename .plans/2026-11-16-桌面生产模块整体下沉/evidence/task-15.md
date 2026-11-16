# 任务15实施证据（尚未完成）

严格遵守禁测要求，未执行任何 Cargo、测试、窗口绘制或回归。

已实现：

- App 直接持有 work_sidebar、work_workspace、workspace_mode 的真实领域 State，删除三处临时拼装/回写。所有实际字段读写同步至所属领域状态。
- view_layout::drag 拥有三类代码面板和工作侧栏的拖拽尺寸、锚点、开始/移动/释放转换；保留首个锚点事件提前返回的原顺序。工作分栏仍复用既有 work_workspace::State。
- panel::apps 接管应用/用量/知识库/工作流/编排页签选择、去重、顺序和最近项目选择；Effect 明确连接宿主的加载和打开项目操作。
- host_module_15_tests 独立注册。侧栏六个用例整函数迁移，布局五个用例整函数迁移，断言未删除。宿主仍保留指针与终端/文件树/偏好网关等跨域契约。

迁移断言映射：

- 原 work_sidebar_filter_messages_select_and_reset_real_filters → crates/libs/desktop-navigation/src/host_module_15_tests.rs::work_sidebar_filter_messages_select_and_reset_real_filters，原断言保留，夹具改用真实局部 State。
- 原 work_sidebar_tabs_and_load_more_keep_independent_limits → crates/libs/desktop-navigation/src/host_module_15_tests.rs::work_sidebar_tabs_and_load_more_keep_independent_limits，原断言保留，夹具改用真实局部 State。
- 原 work_sidebar_space_tasks_load_more_increments_per_space_and_resets_on_filter → crates/libs/desktop-navigation/src/host_module_15_tests.rs::work_sidebar_space_tasks_load_more_increments_per_space_and_resets_on_filter，原断言保留，夹具改用真实局部 State。
- 原 work_space_picker_messages_open_and_close_popover → crates/libs/desktop-navigation/src/host_module_15_tests.rs::work_space_picker_messages_open_and_close_popover，原断言保留，夹具改用真实局部 State。
- 原 work_sidebar_more_and_filter_popovers_are_mutually_exclusive → crates/libs/desktop-navigation/src/host_module_15_tests.rs::work_sidebar_more_and_filter_popovers_are_mutually_exclusive，原断言保留，夹具改用真实局部 State。
- 原 work_sidebar_project_nav_hover_shows_create_entry_and_clears_on_collapse → crates/libs/desktop-navigation/src/host_module_15_tests.rs::work_sidebar_project_nav_hover_shows_create_entry_and_clears_on_collapse，原断言保留，夹具改用真实局部 State。
- 原 drag_start_messages_capture_current_dimensions_and_reset_anchors → crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs::drag_start_messages_capture_current_dimensions_and_reset_anchors，原断言保留，夹具改用真实局部 State。
- 原 work_drag_start_messages_capture_sidebar_width_and_split_ratio → crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs::work_drag_start_messages_capture_sidebar_width_and_split_ratio，原断言保留，夹具改用真实局部 State。
- 原 work_drag_pointer_updates_sidebar_width_and_chat_split → crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs::work_drag_pointer_updates_sidebar_width_and_chat_split，原断言保留，夹具改用真实局部 State。
- 原 file_manager_drag_anchors_then_clamps_inverse_delta → crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs::file_manager_drag_anchors_then_clamps_inverse_delta，原断言保留，夹具改用真实局部 State。
- 原 split_drag_uses_window_width_and_clamps_ratio → crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs::split_drag_uses_window_width_and_clamps_ratio，原断言保留，夹具改用真实局部 State。

尚未完成的必需范围：

- settings.rs、popovers.rs 与 view/mod.rs 的残留局部面板/导航规则仍须逐分支归域，宿主不能保留纯局部状态转换。
- usage、tabs 等现有领域 State 仍通过宿主字段临时拼装；须继续审计其真实共享所有权。
- 其余测试模块尚未逐断言分类迁移或确认仅验证跨域契约。
- 完整来源职责审计与最终净减量仍未完成，因此保留任务卡，不宣称静态交付完成。

来源当前行数（splitlines，含其他任务已接入的公共改动，不作为本任务独立净减量）：

| 来源 | 基线 | 当前 |
|---|---:|---:|
| crates/vw-desktop/src/app/message/view/layout.rs | 293 | 278 |
| crates/vw-desktop/src/app/message/view/layout_tests.rs | 561 | 475 |
| crates/vw-desktop/src/app/message/view/mod.rs | 1168 | 1195 |
| crates/vw-desktop/src/app/message/view/panel/apps.rs | 135 | 55 |
| crates/vw-desktop/src/app/message/view/panel/external.rs | 138 | 138 |
| crates/vw-desktop/src/app/message/view/panel/external_tests.rs | 267 | 267 |
| crates/vw-desktop/src/app/message/view/panel/mod.rs | 218 | 218 |
| crates/vw-desktop/src/app/message/view/panel/popovers.rs | 234 | 234 |
| crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs | 157 | 157 |
| crates/vw-desktop/src/app/message/view/panel/settings.rs | 621 | 621 |
| crates/vw-desktop/src/app/message/view/panel/settings_tests.rs | 591 | 591 |
| crates/vw-desktop/src/app/message/view/panel/tabs.rs | 146 | 146 |
| crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs | 291 | 291 |
| crates/vw-desktop/src/app/message/view/panel/tests.rs | 128 | 128 |
| crates/vw-desktop/src/app/message/view/panel/update_notifications.rs | 36 | 36 |
| crates/vw-desktop/src/app/message/view/panel/update_notifications_tests.rs | 80 | 80 |
| crates/vw-desktop/src/app/message/view/panel/usage.rs | 45 | 45 |
| crates/vw-desktop/src/app/message/view/panel/usage_tests.rs | 59 | 59 |
| crates/vw-desktop/src/app/message/view/panel/web.rs | 71 | 71 |
| crates/vw-desktop/src/app/message/view/panel/web_tests.rs | 50 | 50 |
| crates/vw-desktop/src/app/message/view/panel/work_browser.rs | 70 | 70 |
| crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs | 288 | 288 |
| crates/vw-desktop/src/app/message/view/panel/work_sidebar.rs | 76 | 50 |
| crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs | 286 | 135 |
| crates/vw-desktop/src/app/message/view/panel/work_workspace.rs | 44 | 22 |
| crates/vw-desktop/src/app/message/view/panel/work_workspace_tests.rs | 90 | 90 |
| crates/vw-desktop/src/app/message/view/panel/workspace_mode.rs | 35 | 25 |
| crates/vw-desktop/src/app/message/view/panel/workspace_mode_tests.rs | 37 | 37 |
| crates/vw-desktop/src/app/message/view/tests.rs | 427 | 427 |
| crates/vw-desktop/src/app/message/view/theme.rs | 52 | 52 |
| crates/vw-desktop/src/app/message/view/theme_tests.rs | 211 | 211 |

后续仅在用户另行授权后运行计划中保存的定向测试和宿主检查，本轮不执行。
