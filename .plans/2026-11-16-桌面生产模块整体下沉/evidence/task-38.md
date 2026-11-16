# 任务38：静态交付证据

实现与静态职责核对完成；未编译、未测试、未绘制、未运行实机或回归。分两次提交实施，首次255d50643为部分迁移，本证据覆盖完整结果。

## 生产路径与依赖方向

- 弹窗分别调用chat-view、workspace、git-ui、account-ui；导航与反馈调用navigation/ui-controls，没有新增总控库。
- 账户弹窗包装、顶栏排列下沉领域；overlay只保留一次公开重导出，删除六个空转发子文件。
- 字段组：本任务宿主只借用已有领域账户/更新面板/DesktopUpdate/导航面板/项目编辑器/git状态，及进程持有的窗口ID、字体资源、时钟、剪贴板和跨域会话索引。没有新增App副本、全局trait、Deref、能力开关或反向加载。
- 新测试使用真实PanelState/DesktopUpdate、项目Editor、QuestionState、Git Fixture/ViewContext、已有PetView夹具与RenderFixture；仅数据装配和外部端口替身，不复制被测更新算法。
- 测试沿用真实字体加载、tiny-skia、布局、Operation、UserInterface事件与截图代码；移除领域用例对App初始化及环境网关配置的依赖。宿主真实网关契约仍保留子进程隔离。
- navigation的两个新增开发依赖仅用于原有截图与常规设置样式参照；ui-controls新增既有iced_runtime开发依赖。Cargo.lock仅手工同步这两个包已有依赖边，无Cargo调用。

## 全部来源文件与残留理由（splitlines）

| 来源文件 | 固定基线 | 本次前 | 当前 | 去向或逐入口保留理由 |
|---|---:|---:|---:|---|
| `crates/vw-desktop/src/app/components/about_modal.rs` | 13 | 13 | 10 | desktop-navigation::components::about_modal；view仅投影locale/copied hash并映射Close/OpenUrl/CopyCode。 |
| `crates/vw-desktop/src/app/components/about_modal_ui_refresh_41_tests.rs` | 584 | 584 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/components/color_picker/mod.rs` | 46 | 46 | 46 | desktop-ui-controls::color_picker；两个render入口仅将领域颜色/格式/取色意图映射为调用者提供的宿主消息，None映射保持不变。 |
| `crates/vw-desktop/src/app/components/install_cli_modal.rs` | 88 | 88 | 88 | desktop-navigation::components::install_cli_modal；map映射更新/来源/命令页签/WebInstall/CopyCode；help连接实际可选择文本与剪贴板；四个无App入口传参，view_app_update仅投影UpdatePanel/DesktopUpdate及导航命令页签。 |
| `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs` | 173 | 173 | 56 | 仅App真实顶栏投影的字体/绘制系统边界；安装视图/说明hover/命令页签交互已整组迁入navigation。 |
| `crates/vw-desktop/src/app/components/install_web_modal.rs` | 10 | 10 | 10 | desktop-navigation::components::install_web_modal；view将系统安装状态投影为领域Input，映射安装消息。 |
| `crates/vw-desktop/src/app/components/model_hover_tooltip.rs` | 5 | 5 | 5 | desktop-navigation::components::model_hover_tooltip；类型和视图公开兼容路径，无局部算法。 |
| `crates/vw-desktop/src/app/components/notification.rs` | 15 | 15 | 15 | desktop-navigation::components::notification；view仅投影更新、通知状态并映射系统操作消息。 |
| `crates/vw-desktop/src/app/components/overlays/above/mod.rs` | 1 | 1 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/components/overlays/below.rs` | 1 | 1 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/components/overlays/inline_right.rs` | 1 | 1 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/components/overlays/left.rs` | 1 | 1 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/components/overlays/mod.rs` | 28 | 28 | 2 | desktop-ui-controls::overlays::*兼容导出，无定位算法。 |
| `crates/vw-desktop/src/app/components/overlays/point_below.rs` | 1 | 1 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/components/overlays/side.rs` | 1 | 1 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/components/task_pet.rs` | 52 | 52 | 48 | desktop-task-board::task_pet/pet_view；view只投影窗口、时钟、回复及任务列表，map映射系统窗口和任务操作；window与avatar_preview调用真实领域绘制。 |
| `crates/vw-desktop/src/app/components/task_pet_tests.rs` | 230 | 230 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/components/toast.rs` | 31 | 31 | 27 | desktop-ui-controls::toast；cache_sync_retry_message读取跨技能库PendingRetry并映射宿主重试；view投影active_toast/locale；modal_with_feedback组合跨域卡片与提示。 |
| `crates/vw-desktop/src/app/components/toast_tests.rs` | 35 | 35 | 44 | 仅技能PendingRetry→重试消息及取消确认→项目状态的跨域效果契约。 |
| `crates/vw-desktop/src/app/components/toast_ui_refresh_33_tests.rs` | 706 | 706 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/components/top_bar/controls.rs` | 13 | 13 | 13 | 投影窗口、终端、导航与文件管理状态；真实controls绘制在导航领域。 |
| `crates/vw-desktop/src/app/components/top_bar/drag.rs` | 3 | 3 | 3 | 导航拖动控件意图映射为宿主窗口命令。 |
| `crates/vw-desktop/src/app/components/top_bar/external.rs` | 6 | 6 | 6 | 投影当前页面/项目和外部程序探测结果；外部启动由系统端口执行。 |
| `crates/vw-desktop/src/app/components/top_bar/gateway.rs` | 24 | 24 | 22 | with_state仅借用网络/MCP/插件/LSP数据，转换为领域State；gateway_services_module映射消息。 |
| `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs` | 122 | 122 | 38 | 仅管理插件跨设置页签Batch及真实宿主菜单消息契约；4个LSP领域展示用例已迁出。 |
| `crates/vw-desktop/src/app/components/top_bar/mapping.rs` | 51 | 51 | 51 | 仅全局/领域消息和MenuType双向转换。 |
| `crates/vw-desktop/src/app/components/top_bar/menu_actions.rs` | 1 | 1 | 1 | desktop-platform原生菜单动作兼容导出。 |
| `crates/vw-desktop/src/app/components/top_bar/menus.rs` | 59 | 59 | 59 | MenuEntry/adapt及各entries函数用于原生菜单映射；inputs投影跨域状态，四个view入口与map保留真实消息转换。 |
| `crates/vw-desktop/src/app/components/top_bar/workspace_mode.rs` | 12 | 12 | 12 | 投影导航模式/弹层状态，并映射模式选择与开关消息。 |
| `crates/vw-desktop/src/app/components/top_bar.rs` | 34 | 34 | 48 | desktop-navigation::components::top_bar::toolbar(Sections)；宿主调用各子域投影与消息适配器，领域固定11分区排列及布局。 |
| `crates/vw-desktop/src/app/components/top_bar_tests.rs` | 77 | 77 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/account_details.rs` | 8 | 8 | 13 | desktop-account-ui::account_details::with_account_details；弹窗包装和关闭动作已迁出；宿主只借用账户/locale并映射AccountMessage。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork.rs` | 18 | 18 | 18 | desktop-chat-view::modals::chat_fork；宿主验证当前会话跨域锚点并映射Fork/Close消息，不持有绘制实现。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs` | 1240 | 1240 | 792 | 仅三个系统装配契约：点击用户行冻结锚点→HTTP分叉/历史读回→项目/运行时切换；重开工作树的网关目录权威；文件树Enter→项目校验→toast保留编辑器。HTTP替身、next_output、断言投影、点击测量与isolated均直接服务这些边界，不再保留纯弹窗绘制夹具。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/chat_reset.rs` | 11 | 11 | 11 | desktop-chat-view::modals::chat_reset；宿主投影reset索引及chat长度并映射历史重置意图。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/file_tree_delete.rs` | 11 | 11 | 11 | desktop-workspace::views::modals::file_tree_delete；投影文件树待删路径并映射真实项目消息。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/git_diff.rs` | 13 | 13 | 9 | desktop-git-ui::modal_overlays；投影git-panel编辑/丢弃卡片，保留宿主消息类型边界。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/git_diff_tests.rs` | 69 | 69 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal.rs` | 18 | 18 | 14 | desktop-chat-view::modals::permission_modal；投影请求/队列与跨域toast，并映射批准/拒绝/切换意图。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/permission_modal_tests.rs` | 175 | 175 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab.rs` | 3 | 3 | 0 | 已删除冗余转发/测试挂载；实际入口在所属领域库。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/general_tab_tests.rs` | 62 | 62 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/project_edit/mod.rs` | 92 | 92 | 90 | desktop-workspace::project_edit_view；map仅转换项目/工作流/剪贴板意图，HelpMessage连接可选择文本；domain_view只投影Editor/locale/theme/工作流加载状态，with_project_edit调用领域。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/question_modal.rs` | 17 | 17 | 13 | desktop-chat-view::modals::question_modal；投影问题/答案/自定义文本与toast，映射交互意图。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/question_modal_tests.rs` | 6 | 6 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/recent_project_remove.rs` | 11 | 11 | 11 | desktop-workspace::views::modals::recent_project_remove；投影待移除路径和locale，映射项目消息。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/rename.rs` | 16 | 16 | 12 | desktop-workspace::views::modals::rename；投影重命名输入和会话列表/索引，映射项目消息。 |
| `crates/vw-desktop/src/app/impls/app_view_modals/rename_tests.rs` | 6 | 6 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |
| `crates/vw-desktop/src/app/impls/app_view_modals.rs` | 35 | 35 | 32 | 仅注册领域适配器并导出真实装配入口，voice/size的跨窗口边界不变。 |
| `crates/vw-desktop/src/app/impls/app_view_modals_tests.rs` | 8 | 8 | 0 | 旧文件及旧挂载已删除；测试逐项去向见下表。 |

独占来源本次前 4243 行，当前 1630 行；本任务宿主净减少 2613 行。固定基线共 4243 行。统计含注释与空行；不将测试减量冒称生产减量。

## 每个旧测试到新测试的断言映射

除下列明确拆分项，所有原用例名及断言按序保留，仅将消息匹配改为真实领域Event和App字段借用改为局部状态。

- `ui_refresh_41_preserves_values_and_disabled_states`拆为apps-ui同名书签用例与navigation的`ui_refresh_41_cli_preserves_values_and_disabled_states`；书签输入/文本编辑器裁剪/来源禁用/CLI失败换行截图断言全部保留。
- `ui_refresh_32_preserves_values_and_disabled_states`的重命名输入、布局、滚动、Enter/保存/取消断言在workspace同名用例；末尾三类空弹窗的输入数/标签数/underlay消息断言按职责分别在chat-view与workspace的`ui_refresh_32_absent_modals_preserve_underlay`。
- `ui_refresh_32_escape_cancels_only_the_visible_modal`按fork/reset与session拆到chat-view、workspace同名用例，保留Escape只关闭最外层的原消息断言；原文件重命名字段从未参与这些视图，移除无关准备。
- `app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes`的安装卡片分支迁navigation，顶栏分支保留宿主；每种原主题/页签组合继续覆盖真实draw与非透明像素断言。
- `ui_refresh_33_confirmation_preserves_messages`的布局/滚动/键盘/确认及取消消息在ui-controls；原App更新后待移除路径为空、最近项目未删两条断言在宿主`confirmation_cancel_keeps_recent_project_and_clears_pending_path`。
- `toast_text_wraps_as_needed_and_leaves_room_for_retry`在ui-controls匹配本地Retry事件；真实PendingRetry→SkillHub重试消息由原toast_tests宿主契约保留。
- 宠物用例的open_reply调用仅用于准备展开回复状态；领域夹具显式设置reply/hover输入，尺寸仍调用真实pet_interaction::task_pet_window_size；原鼠标操作、回复提交、滚动、双主题像素断言保留。

| 原用例 | 去向 | 原断言宏数 |
|---|---|---:|
| `about_modal_ui_refresh_41_tests.rs::ui_refresh_41_layout_and_interaction` | `crates/libs/desktop-navigation/src/host_module_38_tests/information_dialogs.rs::ui_refresh_41_layout_and_interaction` | 25 |
| `about_modal_ui_refresh_41_tests.rs::ui_refresh_41_preserves_values_and_disabled_states` | `crates/libs/desktop-apps-ui/src/host_module_38_tests.rs::ui_refresh_41_preserves_values_and_disabled_states` | 27 |
| `install_cli_modal_tests.rs::app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes` | `crates/libs/desktop-navigation/src/host_module_38_tests/install_dialogs.rs::app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes`, `crates/vw-desktop/src/app/components/install_cli_modal_tests.rs::app_update_modal_draws_chinese_selection_and_dynamic_path_in_both_themes` | 1 |
| `install_cli_modal_tests.rs::settings_tabs_peripheral_appearance_cli_commands` | `crates/libs/desktop-navigation/src/host_module_38_tests/install_dialogs.rs::settings_tabs_peripheral_appearance_cli_commands` | 5 |
| `install_cli_modal_tests.rs::update_modals_hide_explanations_and_draw_hover_help_in_both_themes` | `crates/libs/desktop-navigation/src/host_module_38_tests/install_dialogs.rs::update_modals_hide_explanations_and_draw_hover_help_in_both_themes` | 4 |
| `task_pet_tests.rs::mini_companion_toolbar_actions_do_not_start_drag` | `crates/libs/desktop-task-board/src/host_module_38_tests.rs::mini_companion_toolbar_actions_do_not_start_drag` | 2 |
| `task_pet_tests.rs::mini_companion_none_renders_toolbar` | `crates/libs/desktop-task-board/src/host_module_38_tests.rs::mini_companion_none_renders_toolbar` | 2 |
| `task_pet_tests.rs::mini_companion_stack_has_bounded_depth` | `crates/libs/desktop-task-board/src/host_module_38_tests.rs::mini_companion_stack_has_bounded_depth` | 8 |
| `task_pet_tests.rs::mini_companion_expanded_cards_keep_actions_and_scroll` | `crates/libs/desktop-task-board/src/host_module_38_tests.rs::mini_companion_expanded_cards_keep_actions_and_scroll` | 4 |
| `toast_tests.rs::cache_sync_pending_retry_exposes_retry_action_in_toast` | `crates/vw-desktop/src/app/components/toast_tests.rs::cache_sync_pending_retry_exposes_retry_action_in_toast` | 1 |
| `toast_ui_refresh_33_tests.rs::toast_text_wraps_as_needed_and_leaves_room_for_retry` | `crates/libs/desktop-ui-controls/src/host_module_38_tests.rs::toast_text_wraps_as_needed_and_leaves_room_for_retry` | 4 |
| `toast_ui_refresh_33_tests.rs::ui_refresh_33_layout_and_interaction` | `crates/libs/desktop-chat-view/src/host_module_38_tests/feedback_dialogs.rs::ui_refresh_33_layout_and_interaction` | 14 |
| `toast_ui_refresh_33_tests.rs::ui_refresh_33_confirmation_preserves_messages` | `crates/libs/desktop-ui-controls/src/host_module_38_tests.rs::ui_refresh_33_confirmation_preserves_messages` | 9 |
| `toast_ui_refresh_33_tests.rs::ui_refresh_33_preserves_values_and_disabled_states` | `crates/libs/desktop-chat-view/src/host_module_38_tests/feedback_dialogs.rs::ui_refresh_33_preserves_values_and_disabled_states` | 21 |
| `gateway_tests.rs::manage_plugins_opens_the_standalone_agent_plugin_tab` | `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::manage_plugins_opens_the_standalone_agent_plugin_tab` | 1 |
| `gateway_tests.rs::lsp_tab_content_reports_disabled_state_first` | `crates/libs/desktop-navigation/src/host_module_38_tests/lsp.rs::lsp_tab_content_reports_disabled_state_first` | 1 |
| `gateway_tests.rs::lsp_tab_content_reports_empty_status_when_no_servers_exist` | `crates/libs/desktop-navigation/src/host_module_38_tests/lsp.rs::lsp_tab_content_reports_empty_status_when_no_servers_exist` | 1 |
| `gateway_tests.rs::lsp_tab_content_deduplicates_keys_and_appends_status` | `crates/libs/desktop-navigation/src/host_module_38_tests/lsp.rs::lsp_tab_content_deduplicates_keys_and_appends_status` | 1 |
| `gateway_tests.rs::lsp_tab_content_formats_progress_detail` | `crates/libs/desktop-navigation/src/host_module_38_tests/lsp.rs::lsp_tab_content_formats_progress_detail` | 1 |
| `gateway_tests.rs::gateway_tests_keep_public_view_messages_reachable` | `crates/vw-desktop/src/app/components/top_bar/gateway_tests.rs::gateway_tests_keep_public_view_messages_reachable` | 2 |
| `top_bar_tests.rs::view_builds_single_row_with_all_toolbar_sections` | `crates/libs/desktop-navigation/src/host_module_38_tests/toolbar.rs::view_builds_single_row_with_all_toolbar_sections` | 2 |
| `top_bar_tests.rs::work_top_bar_keeps_all_global_menus_and_project_tools` | `crates/libs/desktop-navigation/src/host_module_38_tests/toolbar.rs::work_top_bar_keeps_all_global_menus_and_project_tools` | 3 |
| `top_bar_tests.rs::workspace_mode_selector_keeps_code_controls` | `crates/libs/desktop-navigation/src/host_module_38_tests/toolbar.rs::workspace_mode_selector_keeps_code_controls` | 1 |
| `chat_fork_ui_refresh_32_tests.rs::fork_desktop_clicks_send_frozen_turn_and_open_gateway_history` | `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_desktop_clicks_send_frozen_turn_and_open_gateway_history` | 32 |
| `chat_fork_ui_refresh_32_tests.rs::fork_desktop_layout_and_interaction` | `crates/libs/desktop-chat-view/src/host_module_38_tests/session_dialogs.rs::fork_desktop_layout_and_interaction` | 15 |
| `chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_preserves_values_and_disabled_states` | `crates/libs/desktop-workspace/src/host_module_38_tests/session_dialogs.rs::ui_refresh_32_preserves_values_and_disabled_states` | 18 |
| `chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_escape_cancels_only_the_visible_modal` | `crates/libs/desktop-chat-view/src/host_module_38_tests/session_dialogs.rs::ui_refresh_32_escape_cancels_only_the_visible_modal`, `crates/libs/desktop-workspace/src/host_module_38_tests/session_dialogs.rs::ui_refresh_32_escape_cancels_only_the_visible_modal` | 2 |
| `chat_fork_ui_refresh_32_tests.rs::file_tree_delete_confirmation_draws_and_routes_actions` | `crates/libs/desktop-workspace/src/host_module_38_tests/session_dialogs.rs::file_tree_delete_confirmation_draws_and_routes_actions` | 4 |
| `chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_invalid_file_name_feedback_keeps_inline_editor` | `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::ui_refresh_32_invalid_file_name_feedback_keeps_inline_editor` | 5 |
| `chat_fork_ui_refresh_32_tests.rs::todo_continuation_status_draws_centered_in_both_themes` | `crates/libs/desktop-chat-view/src/host_module_38_tests/todo_status.rs::todo_continuation_status_draws_centered_in_both_themes` | 2 |
| `chat_fork_ui_refresh_32_tests.rs::fork_selection_reopen_gateway_reads_worktree_authority` | `crates/vw-desktop/src/app/impls/app_view_modals/chat_fork_ui_refresh_32_tests.rs::fork_selection_reopen_gateway_reads_worktree_authority` | 5 |
| `git_diff_tests.rs::with_git_diff_comment_returns_root_when_comment_modal_is_absent` | `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_comment_returns_root_when_comment_modal_is_absent` | 0 |
| `git_diff_tests.rs::with_git_diff_comment_layers_modal_when_comment_draft_exists` | `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_comment_layers_modal_when_comment_draft_exists` | 0 |
| `git_diff_tests.rs::with_git_diff_overlays_builds_empty_overlay_layer_when_modals_are_absent` | `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_builds_empty_overlay_layer_when_modals_are_absent` | 0 |
| `git_diff_tests.rs::with_git_diff_overlays_builds_comment_overlay_without_discard_modal` | `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_builds_comment_overlay_without_discard_modal` | 0 |
| `git_diff_tests.rs::with_git_diff_overlays_builds_discard_overlay_without_comment_modal` | `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_builds_discard_overlay_without_comment_modal` | 0 |
| `git_diff_tests.rs::with_git_diff_overlays_layers_comment_and_discard_modals_together` | `crates/libs/desktop-git-ui/src/host_module_38_tests.rs::with_git_diff_overlays_layers_comment_and_discard_modals_together` | 0 |
| `permission_modal_tests.rs::permission_and_question_modals_preserve_scroll_position` | `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::permission_and_question_modals_preserve_scroll_position` | 2 |
| `permission_modal_tests.rs::with_permission_modal_builds_single_request_dialog_when_queue_is_empty` | `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_builds_single_request_dialog_when_queue_is_empty` | 0 |
| `permission_modal_tests.rs::with_permission_modal_builds_single_and_multi_request_dialogs` | `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_builds_single_and_multi_request_dialogs` | 0 |
| `permission_modal_tests.rs::with_permission_modal_builds_navigation_for_middle_and_last_requests` | `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_builds_navigation_for_middle_and_last_requests` | 0 |
| `permission_modal_tests.rs::with_permission_modal_falls_back_to_first_position_for_request_outside_queue` | `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_falls_back_to_first_position_for_request_outside_queue` | 0 |
| `permission_modal_tests.rs::with_permission_modal_limits_long_pending_request_list` | `crates/libs/desktop-chat-view/src/host_module_38_tests.rs::with_permission_modal_limits_long_pending_request_list` | 0 |
| `general_tab_tests.rs::project_edit_general_form_controls_render_and_update_draft` | `crates/libs/desktop-workspace/src/project_edit_view/host_module_38_tests.rs::project_edit_general_form_controls_render_and_update_draft` | 9 |
| `question_modal_tests.rs::question_modal_tests_module_is_wired` | `crates/libs/desktop-chat-view/src/host_module_38_tests/question_modal_tests.rs::question_modal_tests_module_is_wired` | 1 |
| `rename_tests.rs::rename_tests_module_is_wired` | `crates/libs/desktop-workspace/src/host_module_38_tests/rename_tests.rs::rename_tests_module_is_wired` | 1 |
| `app_view_modals_tests.rs::app_view_modals_tests_module_is_wired` | `crates/libs/desktop-navigation/src/host_module_38_tests/app_view_modals_tests.rs::app_view_modals_tests_module_is_wired` | 1 |

全部 47 个原测试入口均有真实注册去向；断言宏数按原用例文本范围辅助核对，不表示执行数。辅助函数断言随相应套件保留，拆分项以上述说明补充。

## 注册与静态验收

- desktop-navigation、desktop-chat-view、desktop-git-ui、desktop-apps-ui、desktop-workspace、desktop-ui-controls的lib.rs正向挂载host_module_38_tests；任务板在已有task_pet_tests中挂载同名子模块以复用原Fixture；项目表单在project_edit_view中挂载同名子模块。
- 新增path声明均解析到存在的领域库源码，仅复用已存在settings-shell库中的通用TabAppearance断言，不加载宿主源码。
- 全部原用例名称做静态映射，无缺失；新领域套件不引用crate::app、App或宿主源码；已移除旧测试挂载。
- 修改的Rust文件独立rustfmt语法/格式处理与git diff --check已执行；未执行Cargo、测试、绘制或实机。

## 以后用户另行授权才执行的命令

- `python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_38_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib host_module_38_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-apps-ui --lib host_module_38_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-workspace --lib host_module_38_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-ui-controls --lib host_module_38_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_38_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib host_module_38_tests -- --test-threads=1`

上述过滤覆盖真实迁移用例。宿主保留跨域契约以后可用相应具体原测试名定向验证；本轮未运行，不能推断字体无panic、行为通过或编译提速。
