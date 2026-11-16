# 任务15：窗口布局与导航消息静态交付

本轮遵守用户禁测要求：未执行 Cargo（含 metadata）、单元测试、构建、实机/窗口绘制或回归。以下计数来自源码，不是运行通过数。

## 实现与真实接线

- ViewMessage → layout::update → view_layout::drag::update/finish；五类拖拽开始、首帧锚点、顺序提前返回、尺寸/比例边界及释放持久化决策均在领域。native pointer 继续依原顺序扇出至宠物、终端、任务板。
- ViewMessage → panel::settings::update_navigation → desktop_navigation::panel::settings::update；局部设置、菜单、弹层、网关标签选择与去重加载决策归域。Effect 明确连接项目创建/加载、系统设置、模型详情、编辑器保存和偏好保存。
- ViewMessage → panel::apps → panel::apps::update；应用页签去重、顺序、选择、最近项目选择归域，OpenUsage/OpenKnowledge/OpenWorkflowTool/OpenWorkflowManager/OpenRecent 为具名效果。
- ViewMessage → panel::tabs → panel::tabs::update；项目预览归档/恢复及页签选择规则归域；GoHome/TabSelected/TabClosed 效果连接终端切换、项目会话、Git、知识库和工作流。
- ViewMessage → panel::popovers → panel::popovers::update → desktop_chat_input::popovers；真实 Visibility 类型直接持有，互斥、请求状态门控、悬停清理及加载决策归域，宿主仅执行加载/保存/发送端口。
- WorkSidebar/WorkWorkspace/WorkspaceMode 直接更新 App 持有的既有领域 State，删除临时 mem::take 拼装和回写；项目主页通过领域判定连接会话缓存释放。
- Usage/Update/Web 分别直接更新 desktop_usage::panel::State、desktop_update::panel::PanelState、desktop_apps_ui::web::State；没有第二份状态或平行算法。
- CompanionNavigation 持有显示、机器人悬停及右键菜单状态；原生 window::Id 和异步关闭/重新打开仍由宿主管理。
- window_lifecycle 归属幂等关机标记、窗口分类、主窗尺寸规范化/启动修正、全屏稳定结束规则；宿主保留原生关闭/托盘/资源退出装配。
- 终端 toggle_panel/finish_pointer_resize、任务板 clear_pending_drag、工作区 discard_pending_drop、宠物 commit_size 在各真实领域实现中接入，旧内联规则已移除。
- 执行期间另一个已提交功能新增移动布局入口。本任务将其真实 MobileLayoutState 和局部导航迁入 desktop_navigation::mobile_layout，旧状态路径仅作兼容导出；项目/设置打开由具名效果连接，保留审批/未确认会话阻断语义。

## 状态、依赖与边界

App 直接持有 navigation_sidebar、navigation_workspace、navigation_mode、layout_drag、popover_visibility、navigation_panels、navigation_tabs、companion_navigation、external_navigation，以及既有领域的 usage_panel/update_panel/web_navigation。MobileLayoutState 通过稳定导出指向导航库。原 input_popovers 是会话输入内容状态，与 popover_visibility 保持独立。

新增生产依赖仅 desktop-navigation → desktop-chat-input、desktop-settings-shell，复用真实类型。新增 iced_runtime 仅为迁入测试的 dev-dependency；Cargo.lock 只更新真实依赖边。静态图核对未发现新增方向返回 desktop-navigation，领域不得依赖 vw-desktop、App、宿主 path/include。未新增根工作区清单或未来能力开关。

原生窗口 ID/位置/尺寸、cursor、关闭到托盘、关机/偏好事务 scope、控制器和异步资源生命周期继续在宿主；导航算法接受局部状态/类型或 native snapshot，不借用整个 App。模型、终端、任务、会话、文件树、LSP、权限和偏好端口使用既有真实领域类型。

## 宿主生产函数保留审计

| 文件/函数 | 唯一保留职责 |
|---|---|
| view/mod.rs：sync_task_pet_window | 持有 native window ID，打开/关闭/聚焦系统窗口，处理异步关闭回调 |
| view/mod.rs：open_task_pet_session | 连接宠物目标会话与项目 OpenProjectSessionPressed，避免重复打开当前会话 |
| view/mod.rs：ViewMessage、update | 主事件枚举与消息/效果映射；原生窗口/动画/焦点和其它领域端口装配。宠物交互调用既有 TaskPetState；大小提交调用 window::commit_size |
| view/mod.rs：handle_global_key、global_command_message、workflow_key_message 导入 | 全局键映射至既有导航快捷键、工作流、Search、Copy、大小模态端口 |
| layout.rs：update | native window/pointer/file-drop 事件与多个领域的顺序扇出，托盘/退出资源清理、LSP 计时器；无拖拽/尺寸算法副本 |
| layout.rs：mobile_message、update_mobile | Host ViewMessage 映射与 OpenProject/OpenSession/CreateSession/OpenSettingsTab 具名效果装配 |
| panel/mod.rs：update、sync_work_browser | 主路由、退出进程及 browser lifecycle 端口调用 |
| panel/apps.rs：update | message 映射和具名效果对应的使用量、知识库、工作流、项目真实端口 |
| panel/popovers.rs：update | message 映射与 ACP/model 加载、偏好保存、会话发送效果。发送行为属于会话配置端口；关闭发送弹层调用既有领域 close |
| panel/settings.rs：update、navigation_message、update_navigation | typed message/Effect 与项目、模型、MCP、插件、编辑器、偏好、终端、Toast 的真实装配 |
| panel/settings.rs：update_panel、panel_message | 安装/更新 domain message 和 effects 映射，不再拼装 State |
| panel/settings.rs：close_requested/start_shutdown/shutdown | 托盘、窗口、语音热键、LSP、浏览器/子进程和活动流的真实生命周期 |
| panel/settings.rs：wait_for_settings_saves、wait_for_editor_settings_saves、arm_shutdown_exit_watchdog | 有界等待 native channels/系统退出看门狗；保留 test cfg 禁用真实退出 |
| panel/settings.rs：take_shutdown_stream_requests、stop_shutdown_streams | 提取实际请求资源，调用原会话/任务计划网关取消端口并有界 join |
| panel/settings.rs：restart_application、begin_shutdown/close_requested_windows 导出 | 当前可执行文件/参数和系统进程重启；纯状态函数仅导出既有领域实现 |
| panel/tabs.rs：update | 按领域具名效果连接项目索引、缓存、会话/偏好、终端、Git/工作流/知识库资源 |
| panel/external.rs：update、open_project_external | gateway 路径判型/原生 reveal、Preview、Clipboard 和外部打开。canonicalize/URL 编码属于 native path 边界；可用性和选择规则已归 external::State |
| panel/update_notifications.rs：map、update | updater message 与配置写入/Toast 端口映射 |
| panel/usage.rs：update | usage message 与 session raw-cache 和 gateway 原始载荷读取连接 |
| panel/web.rs：update | web domain message、保存和 Toast 端口映射 |
| panel/work_browser.rs：map、placement、view、finish、update、sync_lifecycle | 从真实 App 子域投影 Placement，持有 native controller、映射 domain view/events 和 Toast |
| panel/work_browser.rs：sync_lifecycle_result、BROWSER_BOTTOM_INSET（cfg(test)） | 仅验证 native 终端/浏览器联动和 status-bar 实际集成契约 |
| panel/work_sidebar.rs：update | message 映射；domain project_home 判定后的 session stash/释放/失效处理，以及偏好保存 |
| panel/work_workspace.rs：update | message 映射后直接借用领域 State |
| panel/workspace_mode.rs：update、select_workspace_mode（cfg(test)） | typed message 和偏好保存端口；cfg(test) 入口覆盖实际 App 跨域隔离 |
| theme.rs：update | 已有 desktop_theme::selection 处理主题身份；宿主将 native Iced Theme 传播给真实 editor/Git/terminal 并保存偏好 |
| state/mobile_layout.rs | 稳定兼容导出和 App 原生窗口集成测试挂载，无平行状态实现 |

## 断言逐项归属

以下每行代表一个完整原用例，原用例内每项 assert/assert_eq/assert_ne 保留，表内为宏数量而非运行结果。152 个基线用例均有真实归属，总计 623 项显式断言。源码静态比较未发现断言数量变化；事件/绘制代码亦完整迁移。

| 原来源/用例 | 当前归属 | 断言 | 保留或迁移理由 |
|---|---|---:|---|
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::test_module_is_wired | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::test_module_is_wired | 1 → 1 | 宿主 mod/路由实际挂载契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::window_moved_only_updates_main_window_position | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::window_moved_only_updates_main_window_position | 2 → 2 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::task_pet_window_moved_updates_pet_position_without_moving_main_window | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::task_pet_window_moved_updates_pet_position_without_moving_main_window | 2 → 2 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::drag_start_messages_capture_current_dimensions_and_reset_anchors | `crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs`::drag_start_messages_capture_current_dimensions_and_reset_anchors | 9 → 9 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::work_drag_start_messages_capture_sidebar_width_and_split_ratio | `crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs`::work_drag_start_messages_capture_sidebar_width_and_split_ratio | 6 → 6 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::work_drag_pointer_updates_sidebar_width_and_chat_split | `crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs`::work_drag_pointer_updates_sidebar_width_and_chat_split | 4 → 4 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::settings_drag_anchors_then_clamps_panel_width | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::settings_drag_anchors_then_clamps_panel_width | 4 → 4 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_manager_drag_anchors_then_clamps_inverse_delta | `crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs`::file_manager_drag_anchors_then_clamps_inverse_delta | 3 → 3 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::split_drag_uses_window_width_and_clamps_ratio | `crates/libs/desktop-navigation/src/host_module_15_tests/layout.rs`::split_drag_uses_window_width_and_clamps_ratio | 2 → 2 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::terminal_drag_anchors_then_clamps_height_to_window | `crates/libs/desktop-terminal/src/host_module_15_tests.rs`::terminal_drag_anchors_then_clamps_height_to_window | 3 → 3 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::task_board_drag_starts_only_after_threshold_is_exceeded | `crates/libs/desktop-task-board/src/host_module_15_tests.rs`::task_board_drag_starts_only_after_threshold_is_exceeded | 4 → 4 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::window_resize_ignores_other_windows_and_clamps_main_dimensions | `crates/libs/desktop-navigation/src/host_module_15_tests/geometry.rs`::window_resize_ignores_other_windows_and_clamps_main_dimensions | 4 → 4 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::window_resize_during_fullscreen_settling_does_not_consume_startup_check | `crates/libs/desktop-navigation/src/host_module_15_tests/geometry.rs`::window_resize_during_fullscreen_settling_does_not_consume_startup_check | 2 → 2 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::hovered_file_paths_are_unique_and_blocked_during_internal_drag | `crates/libs/desktop-navigation/src/host_module_15_tests/file_hover.rs`::hovered_file_paths_are_unique_and_blocked_during_internal_drag | 2 → 2 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::hovered_files_left_clears_external_hover_but_preserves_internal_drag | `crates/libs/desktop-navigation/src/host_module_15_tests/file_hover.rs`::hovered_files_left_clears_external_hover_but_preserves_internal_drag | 5 → 5 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::fullscreen_layout_settled_clears_settling_flag | `crates/libs/desktop-navigation/src/host_module_15_tests/geometry.rs`::fullscreen_layout_settled_clears_settling_flag | 1 → 1 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::global_release_resets_active_drag_state_without_file_manager_persistence | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::global_release_resets_active_drag_state_without_file_manager_persistence | 11 → 11 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::global_release_transfers_internal_files_to_hovered_input | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::global_release_transfers_internal_files_to_hovered_input | 6 → 6 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::global_cursor_left_makes_visible_lsp_hover_noninteractive_and_schedules_hide | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::global_cursor_left_makes_visible_lsp_hover_noninteractive_and_schedules_hide | 2 → 2 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_move_drop_global_release_submits_once_without_chat_delivery | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_move_drop_global_release_submits_once_without_chat_delivery | 4 → 4 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_move_drop_cursor_leave_cancels_without_pending_chat_files | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_move_drop_cursor_leave_cancels_without_pending_chat_files | 3 → 3 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_move_drop_chat_release_orders_preserve_existing_delivery | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_move_drop_chat_release_orders_preserve_existing_delivery | 3 → 3 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_external_drop_hover_left_and_release_never_import_or_leave_chat_paths | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::file_tree_external_drop_hover_left_and_release_never_import_or_leave_chat_paths | 3 → 3 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::terminal_resize_drag_continues_outside_handle_and_release_stops_updates | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::terminal_resize_drag_continues_outside_handle_and_release_stops_updates | 5 → 5 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::terminal_resize_tiny_window_keeps_height_within_window_limit | `crates/libs/desktop-terminal/src/host_module_15_tests.rs`::terminal_resize_tiny_window_keeps_height_within_window_limit | 1 → 1 | 真实布局/终端/文件交互/任务拖拽局部状态；替换大型 App 夹具 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs`::shutdown_waits_for_pending_layout_preferences | `crates/vw-desktop/src/app/message/view/layout_tests.rs`::shutdown_waits_for_pending_layout_preferences | 10 → 10 | native window/cursor、文件树→项目/Chat/Knowledge、Terminal/LSP/偏好 flush 的路由或顺序契约 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::test_module_is_wired | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::test_module_is_wired | 1 → 1 | 宿主 mod/路由实际挂载契约 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::can_open_external_requires_cached_true_value | `crates/libs/desktop-navigation/src/host_module_15_tests/external.rs`::can_open_external_requires_cached_true_value | 3 → 3 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::preferred_open_without_project_closes_menu_without_scheduling_action | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::preferred_open_without_project_closes_menu_without_scheduling_action | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::preferred_open_rejects_unavailable_target | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::preferred_open_rejects_unavailable_target | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::preferred_open_schedules_available_target_without_polling_gateway | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::preferred_open_schedules_available_target_without_polling_gateway | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::explicit_open_updates_preference_and_closes_menu_without_project | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::explicit_open_updates_preference_and_closes_menu_without_project | 3 → 3 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::explicit_open_rejects_unavailable_target_after_saving_preference | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::explicit_open_rejects_unavailable_target_after_saving_preference | 3 → 3 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::explicit_open_schedules_available_target | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::explicit_open_schedules_available_target | 3 → 3 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::finder_open_schedules_reveal_without_polling_gateway | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::finder_open_schedules_reveal_without_polling_gateway | 1 → 1 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::chat_paths_open_files_in_preview_and_reveal_directories | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::chat_paths_open_files_in_preview_and_reveal_directories | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::finder_open_preserves_current_menu_while_scheduling_reveal | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::finder_open_preserves_current_menu_while_scheduling_reveal | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::copy_without_project_closes_menu_without_scheduling_action | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::copy_without_project_closes_menu_without_scheduling_action | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::copy_existing_project_schedules_clipboard_action_without_polling_clipboard | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::copy_existing_project_schedules_clipboard_action_without_polling_clipboard | 1 → 1 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::copy_missing_project_path_falls_back_and_schedules_clipboard_action | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::copy_missing_project_path_falls_back_and_schedules_clipboard_action | 1 → 1 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::unrelated_message_is_rejected_without_mutating_menu | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::unrelated_message_is_rejected_without_mutating_menu | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::failed_path_reveal_shows_gateway_error_and_target_in_toast | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::failed_path_reveal_shows_gateway_error_and_target_in_toast | 3 → 3 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::successful_path_reveal_does_not_show_toast | `crates/vw-desktop/src/app/message/view/panel/external_tests.rs`::successful_path_reveal_does_not_show_toast | 2 → 2 | 实际 gateway/Preview/Clipboard/偏好/Toast 效果调度，检查 native future 不提前轮询 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::send_mode_popover_stays_closed_when_session_is_not_requesting | `crates/libs/desktop-navigation/src/host_module_15_tests/popovers.rs`::send_mode_popover_stays_closed_when_session_is_not_requesting | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::opening_acp_popover_with_empty_inventory_requests_bootstrap_and_closes_peers | `crates/libs/desktop-navigation/src/host_module_15_tests/popovers.rs`::opening_acp_popover_with_empty_inventory_requests_bootstrap_and_closes_peers | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::close_popovers_clears_every_popover_and_model_hover | `crates/libs/desktop-navigation/src/host_module_15_tests/popovers.rs`::close_popovers_clears_every_popover_and_model_hover | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::individual_close_messages_only_close_their_target | `crates/libs/desktop-navigation/src/host_module_15_tests/popovers.rs`::individual_close_messages_only_close_their_target | 1 → 1 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::closing_model_popover_also_clears_hover_details | `crates/libs/desktop-navigation/src/host_module_15_tests/popovers.rs`::closing_model_popover_also_clears_hover_details | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::model_hover_changed_replaces_and_clears_hover_state | `crates/libs/desktop-navigation/src/host_module_15_tests/popovers.rs`::model_hover_changed_replaces_and_clears_hover_state | 3 → 3 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::unrelated_view_message_leaves_popover_state_unchanged | `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs`::unrelated_view_message_leaves_popover_state_unchanged | 1 → 1 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::test_module_is_wired | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::test_module_is_wired | 1 → 1 | 宿主 mod/路由实际挂载契约 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::main_window_close_starts_shutdown_once | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::main_window_close_starts_shutdown_once | 3 → 3 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::gateway_plugins_tab_selection_starts_plugin_refresh | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::gateway_plugins_tab_selection_starts_plugin_refresh | 2 → 2 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::gateway_mcp_tab_selection_starts_mcp_refresh | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::gateway_mcp_tab_selection_starts_mcp_refresh | 2 → 2 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::gateway_refresh_is_not_restarted_while_loading | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::gateway_refresh_is_not_restarted_while_loading | 3 → 3 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::system_settings_toggle_clears_help_and_flips_visibility | `crates/libs/desktop-navigation/src/host_module_15_tests/settings.rs`::system_settings_toggle_clears_help_and_flips_visibility | 3 → 3 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::opening_system_settings_tab_closes_transient_popovers | `crates/libs/desktop-navigation/src/host_module_15_tests/settings.rs`::opening_system_settings_tab_closes_transient_popovers | 8 → 8 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::opening_model_detail_selects_models_and_closes_transient_popovers | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::opening_model_detail_selects_models_and_closes_transient_popovers | 8 → 8 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::project_file_settings_actions_toggle_panels_tabs_and_loading_state | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::project_file_settings_actions_toggle_panels_tabs_and_loading_state | 8 → 8 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::panel_visibility_and_menu_actions_update_only_local_state | `crates/libs/desktop-navigation/src/host_module_15_tests/settings.rs`::panel_visibility_and_menu_actions_update_only_local_state | 11 → 11 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::work_mode_ignores_code_only_review_toggle | `crates/libs/desktop-navigation/src/host_module_15_tests/settings.rs`::work_mode_ignores_code_only_review_toggle | 2 → 2 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::about_modal_toggle_round_trips_visibility | `crates/libs/desktop-navigation/src/host_module_15_tests/settings.rs`::about_modal_toggle_round_trips_visibility | 2 → 2 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_modal_initializes_application_update_state | `crates/libs/desktop-navigation/src/host_module_15_tests/update.rs`::app_update_modal_initializes_application_update_state | 16 → 16 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::install_and_update_requests_prepare_modal_state_before_async_work_runs | `crates/libs/desktop-navigation/src/host_module_15_tests/update.rs`::install_and_update_requests_prepare_modal_state_before_async_work_runs | 23 → 23 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::restart_failure_is_exposed_without_exiting | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::restart_failure_is_exposed_without_exiting | 1 → 1 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_result_controls_install_action_for_each_outcome | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_result_controls_install_action_for_each_outcome | 5 → 5 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_progress_drains_available_values_and_drops_disconnected_receiver | `crates/libs/desktop-navigation/src/host_module_15_tests/update.rs`::app_update_progress_drains_available_values_and_drops_disconnected_receiver | 2 → 2 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_finish_clears_receiver_and_sets_success_or_failure_state | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_finish_clears_receiver_and_sets_success_or_failure_state | 6 → 6 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::install_result_resets_update_state_and_sets_outcome_copy | `crates/libs/desktop-navigation/src/host_module_15_tests/update.rs`::install_result_resets_update_state_and_sets_outcome_copy | 11 → 11 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::menu_hover_only_switches_an_already_open_menu | `crates/libs/desktop-navigation/src/host_module_15_tests/settings.rs`::menu_hover_only_switches_an_already_open_menu | 2 → 2 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_download_survives_close_and_blocks_duplicate_and_source_change | `crates/libs/desktop-navigation/src/host_module_15_tests/update.rs`::app_update_download_survives_close_and_blocks_duplicate_and_source_change | 6 → 6 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_save_failure_keeps_selection_and_shows_unsaved_toast | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_save_failure_keeps_selection_and_shows_unsaved_toast | 3 → 3 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::app_update_download_consumes_checked_snapshot_without_refetching_latest | `crates/libs/desktop-navigation/src/host_module_15_tests/update.rs`::app_update_download_consumes_checked_snapshot_without_refetching_latest | 1 → 1 | 局部面板/菜单/更新状态；使用真实控制器及最小外部菜单端口 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::terminal_modes_toggle_preserves_session_and_height_across_mode_roundtrip | `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs`::terminal_modes_toggle_preserves_session_and_height_across_mode_roundtrip | 8 → 8 | 真实关机、配置/模型/插件/MCP/项目/终端/Toast 跨域端口接线 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::home_apps_bar_scroll_messages_update_and_clamp_offsets | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::home_apps_bar_scroll_messages_update_and_clamp_offsets | 0 → 0 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_hovered_tracks_and_clears_hover | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_hovered_tracks_and_clears_hover | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::go_home_archives_project_preview_and_clears_project_context | `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::go_home_archives_project_preview_and_clears_project_context | 9 → 9 | 项目 ID/会话清理或知识库加载的真实跨域效果 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_selected_home_archives_previous_project_preview | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_selected_home_archives_previous_project_preview | 6 → 6 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_selected_project_restores_cached_preview_state | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_selected_project_restores_cached_preview_state | 6 → 6 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_selected_unknown_id_preserves_current_project_preview_state | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_selected_unknown_id_preserves_current_project_preview_state | 6 → 6 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_selected_same_project_changes_screen_without_losing_preview_state | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_selected_same_project_changes_screen_without_losing_preview_state | 6 → 6 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_selected_knowledge_initializes_knowledge_without_project_switch | `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_selected_knowledge_initializes_knowledge_without_project_switch | 4 → 4 | 项目 ID/会话清理或知识库加载的真实跨域效果 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_closed_inactive_removes_only_requested_tab | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_closed_inactive_removes_only_requested_tab | 4 → 4 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_closed_missing_id_preserves_tab_selection | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_closed_missing_id_preserves_tab_selection | 4 → 4 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_closed_active_project_falls_back_home_and_archives_preview | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_closed_active_project_falls_back_home_and_archives_preview | 7 → 7 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_closed_active_tab_on_same_project_preserves_preview_state | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_closed_active_tab_on_same_project_preserves_preview_state | 7 → 7 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_closed_active_project_switches_to_last_project_and_restores_preview | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_closed_active_project_switches_to_last_project_and_restores_preview | 6 → 6 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs`::tab_closed_last_tab_falls_back_to_unselected_home_screen | `crates/libs/desktop-navigation/src/host_module_15_tests/tabs.rs`::tab_closed_last_tab_falls_back_to_unselected_home_screen | 3 → 3 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::settings_message_routes_to_settings_update | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::settings_message_routes_to_settings_update | 1 → 1 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::popover_message_routes_to_popovers_update | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::popover_message_routes_to_popovers_update | 1 → 1 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::tab_message_routes_to_tabs_update | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::tab_message_routes_to_tabs_update | 1 → 1 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::app_message_routes_to_apps_update | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::app_message_routes_to_apps_update | 3 → 3 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::usage_message_routes_to_usage_update | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::usage_message_routes_to_usage_update | 2 → 2 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::web_message_routes_to_web_update | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::web_message_routes_to_web_update | 1 → 1 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::external_message_routes_to_external_update_without_project | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::external_message_routes_to_external_update_without_project | 2 → 2 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::replay_snapshot_message_is_ignored_by_panel_router | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::replay_snapshot_message_is_ignored_by_panel_router | 1 → 1 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::close_request_routes_without_mutating_window_registration | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::close_request_routes_without_mutating_window_registration | 3 → 3 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs`::unrelated_message_is_ignored_by_panel_router | `crates/vw-desktop/src/app/message/view/panel/tests.rs`::unrelated_message_is_ignored_by_panel_router | 1 → 1 | Host panel 路由到各真实领域/系统关闭端口 |
| `crates/vw-desktop/src/app/message/view/panel/update_notifications_tests.rs`::automatic_update_is_silent_preserves_results_on_failure_and_rejects_stale_replies | `crates/libs/desktop-navigation/src/host_module_15_tests/notifications.rs`::automatic_update_is_silent_preserves_results_on_failure_and_rejects_stale_replies | 6 → 6 | 后台检查代际、静默、跳过保存状态 |
| `crates/vw-desktop/src/app/message/view/panel/update_notifications_tests.rs`::skip_only_hides_notification_after_successful_save | `crates/libs/desktop-navigation/src/host_module_15_tests/notifications.rs`::skip_only_hides_notification_after_successful_save | 5 → 5 | 后台检查代际、静默、跳过保存状态 |
| `crates/vw-desktop/src/app/message/view/panel/update_notifications_tests.rs`::automatic_update_does_not_interrupt_manual_update_or_installation | `crates/libs/desktop-navigation/src/host_module_15_tests/notifications.rs`::automatic_update_does_not_interrupt_manual_update_or_installation | 2 → 2 | 后台检查代际、静默、跳过保存状态 |
| `crates/vw-desktop/src/app/message/view/panel/usage_tests.rs`::usage_step_raw_loaded_for_active_session_caches_success_and_error | `crates/vw-desktop/src/app/message/view/panel/usage_tests.rs`::usage_step_raw_loaded_for_active_session_caches_success_and_error | 5 → 5 | 活动会话 raw-cache/索引写回及 Host ViewMessage 拒绝契约 |
| `crates/vw-desktop/src/app/message/view/panel/usage_tests.rs`::unrelated_message_preserves_usage_panel_state | `crates/vw-desktop/src/app/message/view/panel/usage_tests.rs`::unrelated_message_preserves_usage_panel_state | 3 → 3 | 活动会话 raw-cache/索引写回及 Host ViewMessage 拒绝契约 |
| `crates/vw-desktop/src/app/message/view/panel/web_tests.rs`::unrelated_view_message_preserves_web_state | `crates/vw-desktop/src/app/message/view/panel/web_tests.rs`::unrelated_view_message_preserves_web_state | 4 → 4 | Host ViewMessage 拒绝不会污染真实 web state |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_reserves_status_bar_at_bottom | `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_reserves_status_bar_at_bottom | 1 → 1 | native 终端 fullscreen 与浏览器进程联动 / 宿主 status-bar 常量集成 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_content_starts_below_resource_header_and_toolbar | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_content_starts_below_resource_header_and_toolbar | 2 → 2 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::terminal_fullscreen_keeps_native_browser_above_dock_and_hides_it_when_expanded | `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::terminal_fullscreen_keeps_native_browser_above_dock_and_hides_it_when_expanded | 6 → 6 | native 终端 fullscreen 与浏览器进程联动 / 宿主 status-bar 常量集成 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_defaults_to_blank_surface | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_defaults_to_blank_surface | 3 → 3 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_close_terminates_current_page_and_keeps_blank_surface | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_close_terminates_current_page_and_keeps_blank_surface | 6 → 6 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_navigate_reopens_closed_blank_surface | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_navigate_reopens_closed_blank_surface | 2 → 2 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_runtime_tick_updates_address_from_webview_navigation | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_runtime_tick_updates_address_from_webview_navigation | 1 → 1 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_controller_toolbar_uses_compact_height | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_controller_toolbar_uses_compact_height | 1 → 1 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_toolbar_close_action_is_reachable | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_toolbar_close_action_is_reachable | 1 → 1 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_controller_toolbar_external_action_normalizes_safe_url | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_controller_toolbar_external_action_normalizes_safe_url | 2 → 2 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs`::work_browser_controller_toolbar_draws_dark_chinese_error_without_panic | `crates/libs/desktop-web-runtime/src/host_module_15_tests.rs`::work_browser_controller_toolbar_draws_dark_chinese_error_without_panic | 0 → 0 | 浏览器局部状态、受控进程或真实 Headless 布局/事件/draw；保留中文暗黑绘制 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_toggle_preserves_session_list_width | `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_toggle_preserves_session_list_width | 6 → 6 | 实际 session stash/域间宽度隔离或偏好网关前后时序 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_project_home_leaves_the_session_without_leaving_the_project | `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_project_home_leaves_the_session_without_leaving_the_project | 9 → 9 | 实际 session stash/域间宽度隔离或偏好网关前后时序 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_filter_messages_select_and_reset_real_filters | `crates/libs/desktop-navigation/src/host_module_15_tests.rs`::work_sidebar_filter_messages_select_and_reset_real_filters | 8 → 8 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_tabs_and_load_more_keep_independent_limits | `crates/libs/desktop-navigation/src/host_module_15_tests.rs`::work_sidebar_tabs_and_load_more_keep_independent_limits | 8 → 8 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_space_tasks_load_more_increments_per_space_and_resets_on_filter | `crates/libs/desktop-navigation/src/host_module_15_tests.rs`::work_sidebar_space_tasks_load_more_increments_per_space_and_resets_on_filter | 5 → 5 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_space_picker_messages_open_and_close_popover | `crates/libs/desktop-navigation/src/host_module_15_tests.rs`::work_space_picker_messages_open_and_close_popover | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_more_and_filter_popovers_are_mutually_exclusive | `crates/libs/desktop-navigation/src/host_module_15_tests.rs`::work_sidebar_more_and_filter_popovers_are_mutually_exclusive | 5 → 5 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_project_nav_hover_shows_create_entry_and_clears_on_collapse | `crates/libs/desktop-navigation/src/host_module_15_tests.rs`::work_sidebar_project_nav_hover_shows_create_entry_and_clears_on_collapse | 5 → 5 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_updates_before_gateway_reply | `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs`::work_sidebar_updates_before_gateway_reply | 6 → 6 | 实际 session stash/域间宽度隔离或偏好网关前后时序 |
| `crates/vw-desktop/src/app/message/view/panel/work_workspace_tests.rs`::work_workspace_messages_preserve_code_layout_preferences | `crates/vw-desktop/src/app/message/view/panel/work_workspace_tests.rs`::work_workspace_messages_preserve_code_layout_preferences | 16 → 16 | 真实 App 的代码/工作模式、终端、文件/编辑器偏好隔离契约 |
| `crates/vw-desktop/src/app/message/view/panel/workspace_mode_tests.rs`::workspace_mode_message_preserves_code_panel_preferences | `crates/vw-desktop/src/app/message/view/panel/workspace_mode_tests.rs`::workspace_mode_message_preserves_code_panel_preferences | 13 → 13 | 真实 App 的代码/工作模式、终端、文件/编辑器偏好隔离契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::test_module_is_wired | `crates/vw-desktop/src/app/message/view/tests.rs`::test_module_is_wired | 1 → 1 | 宿主 mod/路由实际挂载契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::task_pet_view_messages_update_interaction_state | `crates/vw-desktop/src/app/message/view/tests.rs`::task_pet_view_messages_update_interaction_state | 10 → 10 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::task_pet_reply_pressed_again_closes_editor_and_clears_draft | `crates/libs/desktop-task-board/src/host_module_15_tests.rs`::task_pet_reply_pressed_again_closes_editor_and_clears_draft | 2 → 2 | 完整局部消息/状态行为迁入真实领域；原断言与测试名保留 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::update_routes_panel_layout_theme_and_activity_messages | `crates/vw-desktop/src/app/message/view/tests.rs`::update_routes_panel_layout_theme_and_activity_messages | 7 → 7 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::update_handles_task_pet_drag_remove_and_empty_reply_submit | `crates/vw-desktop/src/app/message/view/tests.rs`::update_handles_task_pet_drag_remove_and_empty_reply_submit | 3 → 3 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::global_key_update_routes_to_global_command_outside_workflow_screen | `crates/vw-desktop/src/app/message/view/tests.rs`::global_key_update_routes_to_global_command_outside_workflow_screen | 0 → 0 | 全局键→工作流/Search/Copy 的实际路由 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::system_settings_messages_hide_and_restore_running_work_browser | `crates/vw-desktop/src/app/message/view/tests.rs`::system_settings_messages_hide_and_restore_running_work_browser | 7 → 7 | 设置与 native browser process 可见性联动 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_hidden_and_avatar_are_independent | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_hidden_and_avatar_are_independent | 10 → 10 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_close_request_keeps_window_until_closed | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_close_request_keeps_window_until_closed | 5 → 5 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_invalid_scale_does_not_enqueue_a_save | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_invalid_scale_does_not_enqueue_a_save | 3 → 3 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_show_is_idempotent | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_show_is_idempotent | 2 → 2 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_hide_preserves_avatar_and_tasks | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_hide_preserves_avatar_and_tasks | 7 → 7 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_show_while_closing_reopens_after_callback | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_show_while_closing_reopens_after_callback | 5 → 5 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_write_keeps_current_session_and_draft | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_write_keeps_current_session_and_draft | 3 → 3 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_reply_route_preserves_target_session | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_reply_route_preserves_target_session | 3 → 3 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_cancel_restores_committed_value | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_cancel_restores_committed_value | 4 → 4 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_save_persists_once | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_save_persists_once | 5 → 5 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_escape_rolls_back_and_modal_blocks_actions | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_escape_rolls_back_and_modal_blocks_actions | 4 → 4 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_none_ignores_slider | `crates/vw-desktop/src/app/message/view/tests.rs`::mini_companion_size_none_ignores_slider | 1 → 1 | 宠物/窗口/偏好事务/会话输入等真实域之间的消息及 native 生命周期契约 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::test_module_is_wired | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::test_module_is_wired | 1 → 1 | 宿主 mod/路由实际挂载契约 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_updates_theme_when_editor_is_independent | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_updates_theme_when_editor_is_independent | 3 → 3 | Iced Theme 与真实 editor/Git/terminal 或偏好 PATCH/ThemeStore 联动 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_changes_effective_editor_theme_when_following_system | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_changes_effective_editor_theme_when_following_system | 2 → 2 | Iced Theme 与真实 editor/Git/terminal 或偏好 PATCH/ThemeStore 联动 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_with_preview_tabs_preserves_editor_data | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_with_preview_tabs_preserves_editor_data | 5 → 5 | Iced Theme 与真实 editor/Git/terminal 或偏好 PATCH/ThemeStore 联动 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::unrelated_view_message_leaves_theme_state_unchanged | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::unrelated_view_message_leaves_theme_state_unchanged | 3 → 3 | Iced Theme 与真实 editor/Git/terminal 或偏好 PATCH/ThemeStore 联动 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_persists_custom_prefix_for_custom_theme | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::app_theme_selected_persists_custom_prefix_for_custom_theme | 9 → 9 | Iced Theme 与真实 editor/Git/terminal 或偏好 PATCH/ThemeStore 联动 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs`::theme_selection_does_not_wait_for_system_settings_gateway | `crates/vw-desktop/src/app/message/view/theme_tests.rs`::theme_selection_does_not_wait_for_system_settings_gateway | 12 → 12 | Iced Theme 与真实 editor/Git/terminal 或偏好 PATCH/ThemeStore 联动 |

与前轮测试核对：迁入用例没有在宿主再保留副本。desktop_split_30_tests 的原生关闭分类仍覆盖 mark_window_closed；本次 geometry 用例验证尺寸/稳定标记，契约不同。desktop-chat-input::popovers_tests 覆盖原始 Visibility helper，本次 popovers 控制器用例另验证 requesting/hover/bootstrap 输入及指令映射。desktop-update::panel_tests 原用例验证源切换/过期响应，迁入 update 用例保留进度 channel、安装文案和 checked-snapshot 契约。task_pet_reply_pressed_again 的空任务清单夹具与 desktop_split_02_tests 的已有卡片清单区分，保留无 inventory 情况的原两项断言。未复制整个 App、未 mock 被测算法。

另有执行期间新增移动布局的 7 个纯布局用例迁入 host_module_15_tests::mobile；其原生主窗往返/业务隔离用例保留在宿主。它们不计入上述固定 152 个基线用例。

## 行数审计

Python pathlib / splitlines，含注释与空行，沿 baseline.json 的文件与 production_candidate 口径；不将测试减量算成生产减量。源码分域以以上逐函数审计为准。

| 来源 | 基线 | 当前 |
|---|---:|---:|
| `crates/vw-desktop/src/app/message/view/layout.rs` | 293 | 227 |
| `crates/vw-desktop/src/app/message/view/layout_tests.rs` | 561 | 343 |
| `crates/vw-desktop/src/app/message/view/mod.rs` | 1168 | 1187 |
| `crates/vw-desktop/src/app/message/view/panel/apps.rs` | 135 | 55 |
| `crates/vw-desktop/src/app/message/view/panel/external.rs` | 138 | 122 |
| `crates/vw-desktop/src/app/message/view/panel/external_tests.rs` | 267 | 255 |
| `crates/vw-desktop/src/app/message/view/panel/mod.rs` | 218 | 218 |
| `crates/vw-desktop/src/app/message/view/panel/popovers.rs` | 234 | 106 |
| `crates/vw-desktop/src/app/message/view/panel/popovers_tests.rs` | 157 | 46 |
| `crates/vw-desktop/src/app/message/view/panel/settings.rs` | 621 | 597 |
| `crates/vw-desktop/src/app/message/view/panel/settings_tests.rs` | 591 | 258 |
| `crates/vw-desktop/src/app/message/view/panel/tabs.rs` | 146 | 133 |
| `crates/vw-desktop/src/app/message/view/panel/tabs_tests.rs` | 291 | 71 |
| `crates/vw-desktop/src/app/message/view/panel/tests.rs` | 128 | 128 |
| `crates/vw-desktop/src/app/message/view/panel/update_notifications.rs` | 36 | 33 |
| `crates/vw-desktop/src/app/message/view/panel/update_notifications_tests.rs` | 80 | 0 |
| `crates/vw-desktop/src/app/message/view/panel/usage.rs` | 45 | 33 |
| `crates/vw-desktop/src/app/message/view/panel/usage_tests.rs` | 59 | 59 |
| `crates/vw-desktop/src/app/message/view/panel/web.rs` | 71 | 41 |
| `crates/vw-desktop/src/app/message/view/panel/web_tests.rs` | 50 | 50 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser.rs` | 70 | 66 |
| `crates/vw-desktop/src/app/message/view/panel/work_browser_tests.rs` | 288 | 97 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar.rs` | 76 | 46 |
| `crates/vw-desktop/src/app/message/view/panel/work_sidebar_tests.rs` | 286 | 135 |
| `crates/vw-desktop/src/app/message/view/panel/work_workspace.rs` | 44 | 22 |
| `crates/vw-desktop/src/app/message/view/panel/work_workspace_tests.rs` | 90 | 90 |
| `crates/vw-desktop/src/app/message/view/panel/workspace_mode.rs` | 35 | 24 |
| `crates/vw-desktop/src/app/message/view/panel/workspace_mode_tests.rs` | 37 | 37 |
| `crates/vw-desktop/src/app/message/view/tests.rs` | 427 | 415 |
| `crates/vw-desktop/src/app/message/view/theme.rs` | 52 | 52 |
| `crates/vw-desktop/src/app/message/view/theme_tests.rs` | 211 | 211 |

生产候选：3382 → 2962，减少 420 行。

测试来源：3523 → 2195，减少 1328 行。

固定来源合计：6905 → 5157，净减少 1748 行。本任务未新建宿主替代实现文件。

新增功能的基线外直接入口 `crates/vw-desktop/src/app/state/mobile_layout.rs`：执行时 71 → 当前 5 行；单列，不冒算固定基线减量。

新增功能的基线外直接入口 `crates/vw-desktop/src/app/state/mobile_layout_tests.rs`：执行时 104 → 当前 36 行；单列，不冒算固定基线减量。

App 字段组、初始化和实际调用者的变更另由本任务提交差异完整列明；大量调用点仅替换字段所属类型，不以路径字符串增长/格式变化宣称减量。不得据此承诺编译提速。

## 静态验证和未验证风险

- 审查真实 mod 注册、cfg 分支、字段唯一性、旧字段引用和直接调用者；无宿主反向 path/include/App 依赖。
- 对本任务实际修改 Rust 源码执行独立 rustfmt 的 emit-stdout 语法解析，未执行源码；对交付差异执行 git diff --check。
- 审查私有暂存快照与当前 HEAD 的实际差异，排除其它并行任务的暂存及无关文件；本任务先前提交为 32e2f4b2875b1e5b86e50c4a57d6fc6d9057ad7d。
- 这是静态交付，未编译、未测试、未绘制。跨 crate 的类型/借用、feature/平台组合、原生行为及布局结果均待用户另行授权后验证。无通过数、无运行/截图结果。

以下仅记录未来命令，本轮严禁执行：

```sh
python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_15_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-terminal --lib host_module_15_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_15_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-web-runtime --lib host_module_15_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib panel::workspace_mode::tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib
```

其它已迁移既有 tests 注册保留其原模块；workspace_mode::tests 的签名适配是本任务直接受影响的源码，未来定向过滤为 `panel::workspace_mode::tests`。宿主跨域契约仍按各来源模块在后续授权时定向验证。
