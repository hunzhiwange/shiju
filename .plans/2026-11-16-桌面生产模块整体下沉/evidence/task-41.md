# 任务41静态交付证据

实现与本轮静态职责收口完成。未运行 Cargo、单元测试、实机窗口或最终回归；本文不表示编译或行为验证通过。

## 生产调用与依赖方向

- App.lifecycle 由 desktop-platform::lifecycle::State 持有本机启动项/忙状态/关闭到托盘偏好；初始化继续注入本机配置读取结果，系统句柄与 Iced WindowId 留在宿主。
- 通知：completion_task/permission_task → desktop-platform::desktop_notification::{completion,permission} → 阻塞线程系统 API；宿主只把结果映射为 GeneralSettingsMessage。错误反馈经 desktop-ui-controls::toast::show_supplemental_error，保留已有反馈优先级与2200ms隐藏消息。
- 托盘：宿主注入本地配置路径，平台持有原生托盘；供应商资格、菜单与结果更新在 desktop-provider-apps，finish_request 负责 scope/generation/id 一次性接受，宿主获取当前认证上下文、启动 Task 并显示返回反馈。
- 菜单：App 投影 → desktop-navigation::native_menu → platform_snapshot → desktop-platform；消息映射复用 top_bar::menus::map。macOS 宿主仅检查主线程、安装菜单和映射原生订阅事件。
- 订阅：Iced recipes/定时器仍由 App 装配；全局事件解析迁至 desktop-navigation::global_events，语义动作逐项映射回宿主 Message。活动判定在 desktop-platform::subscriptions。Escape/captured顺序、拖放与Unfocused取消、快捷键顺序与wasm cfg保持。
- 文件缓存：workspace::project_state::State 实施索引/目录树一次发布、版本推进、搜索结果缓存更新；宿主仅串联工作区与输入建议两个领域的刷新。
- 标题/透明原生主题在 navigation::window_lifecycle；编辑器主题选择在 appearance::theme_state。
- 更新测试使用 desktop-update::panel::update 与真实 PanelState/DesktopUpdate；只观察 Error/Shutdown 端口效果，不构造 App。绘制测试使用 navigation 的真实 view_inner、可选择文本控件、Tree/layout/draw/screenshot。
- 插件表单测试使用 session::questions 的真实局部状态、web-runtime 的真实 MCP 校验和 chat-view 的真实弹窗；取消/空请求的 Task.units 断言映射为产生/不产生请求效果。
- 原生窗口后台夹具的启动参数、隔离环境、ready轮询与stop/kill/wait清理由 desktop-platform::acceptance 持有；调用者仍显式提供可执行文件和绝对证据目录。

没有新总控库、巨型trait、App副本或反向path/include加载。领域库不依赖vw-desktop；desktop-update的crate::app是库内原有更新模块，与宿主App无关。新增均为已存在依赖包的测试依赖及Windows通知所需tokio平台声明，未引入新外部包。

## 逐文件归属与splitlines

| 文件 | 固定基线 | 当前 | 归属/残留理由 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/desktop_integration_29_tests.rs` | 46 | 18 | 领域反馈断言→ui-controls；宿主仅权限失败关闭设置及不覆盖反馈的跨域消息契约 |
| `crates/vw-desktop/src/app/desktop_integration_46_tests.rs` | 28 | 14 | 哈希/时间/旧结果断言→navigation；宿主仅ClipboardCopyFinished错误映射到toast契约 |
| `crates/vw-desktop/src/app/desktop_integration_47_tests.rs` | 57 | 0 | 整组→desktop-update/host_module_41_tests/release.rs |
| `crates/vw-desktop/src/app/desktop_integration_48_tests.rs` | 48 | 0 | 整组→desktop-update/host_module_41_tests/progress.rs |
| `crates/vw-desktop/src/app/desktop_integration_49_tests.rs` | 52 | 0 | 整组→desktop-update/host_module_41_tests/cancel.rs |
| `crates/vw-desktop/src/app/desktop_lifecycle/native.rs` | 32 | 32 | 配置目录注入、平台托盘事件→宿主消息映射、供应商领域菜单输入装配 |
| `crates/vw-desktop/src/app/desktop_lifecycle/tray_switch.rs` | 68 | 80 | 认证客户端/当前scope获取、跨域Task启动和返回toast；资格/代际/结果状态在provider领域 |
| `crates/vw-desktop/src/app/desktop_lifecycle/window.rs` | 81 | 81 | 主线程窗口ID所有权，open/close/run句柄回调、原生frame恢复及显示效果 |
| `crates/vw-desktop/src/app/desktop_lifecycle.rs` | 44 | 44 | native/wasm平台装配与不支持错误入口；不含业务状态 |
| `crates/vw-desktop/src/app/desktop_models.rs` | 4 | 4 | 稳定模型导出入口，没有实现或局部状态 |
| `crates/vw-desktop/src/app/desktop_notification.rs` | 71 | 33 | 只留Iced Task、结果消息与toast隐藏效果装配 |
| `crates/vw-desktop/src/app/desktop_update.rs` | 2 | 2 | 只导出desktop-update领域API |
| `crates/vw-desktop/src/app/impls/app_basic.rs` | 389 | 318 | 局部算法已调用所属库；保留跨域缓存刷新串联、消息计时、窗口ID到标题/主题投影与状态借用入口 |
| `crates/vw-desktop/src/app/impls/app_basic_tests.rs` | 400 | 78 | 仅保留适配符号挂载守卫和宠物/主窗口ID到标题/主题路由契约 |
| `crates/vw-desktop/src/app/impls/app_subscription.rs` | 464 | 414 | Iced订阅装配、局部活跃信号投影和跨域消息映射；事件解析与活动策略已下沉 |
| `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` | 563 | 272 | 仅保留真实Iced recipes装配、App领域信号投影、运行请求与页面可见性的跨域契约 |
| `crates/vw-desktop/src/app/installer_update.rs` | 13 | 5 | 只导出desktop-update安装API，旧测试挂载已删除 |
| `crates/vw-desktop/src/app/installer_update_tests.rs` | 198 | 0 | 状态/安装校验→desktop-update/host_module_41_tests/installer.rs；真实双主题66帧绘制→navigation/host_module_41_render_tests.rs |
| `crates/vw-desktop/src/app/native_menu/actions.rs` | 98 | 30 | 只投影App各领域为NativeMenuSnapshot并映射领域动作 |
| `crates/vw-desktop/src/app/native_menu/actions_tests.rs` | 158 | 0 | 整组7用例→navigation/host_module_41_tests.rs |
| `crates/vw-desktop/src/app/native_menu/macos.rs` | 29 | 24 | 主线程检查、平台菜单安装与订阅事件映射；菜单构造→navigation/platform_snapshot |
| `crates/vw-desktop/src/app/native_menu/mod.rs` | 21 | 17 | cfg与稳定导出装配，无业务状态 |
| `crates/vw-desktop/src/app/plugin_compat_t25_tests.rs` | 41 | 0 | 整组→chat-view/host_module_41_tests.rs，无App夹具 |
| `crates/vw-desktop/src/app/plugin_compat_window.rs` | 366 | 331 | 保留实际daemon主线程、跨网关安装消息序列与真实中文字体/明暗绘制验收；进程夹具已迁platform |
| `crates/vw-desktop/src/app/recovery_window_tests.rs` | 406 | 364 | 保留实际daemon/Agent流订阅/取消消息/屏幕截图/HTTP调用次数端到端契约；进程夹具已迁platform |
| `crates/vw-desktop/build.rs` | 131（额外归属） | 131 | 保持可执行程序构建期PNG/ICO、Windows manifest/resource编译与链接职责；未移动或改动 |

固定task=41文件基线合计3679行，当前2161行，宿主净减少1518行。build.rs不计入该基线和净减少。该口径不把新领域库行数当成仓库总代码减少。生产候选与测试迁移按真实职责说明，不以文件名判定业务已迁完。

## 残留函数与字段组

- App字段：lifecycle/desktop_update/appearance/workspace_state/input_suggestions均是领域State；main_window_id/task_pet_window_id/released_main_windows是主线程窗口所有权；shutdown_requested/startup_resize_checked是跨订阅、窗口关闭和启动回调协调；startup_state使用navigation已有NativeStartupState，宿主安排Iced启动效果。
- App基础函数：全文下列函数按职责留在宿主，没有本地领域算法副本。
  - `is_chat_fullscreen`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `is_chat_half_fullscreen`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `normalize_file_search_query`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `is_diff_file_expanded`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `toggle_diff_file_expanded`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `ensure_diff_file_expanded`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `replace_expanded_files`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `clear_expanded_files`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `set_single_expanded_file`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `is_file_tree_dir_expanded`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `toggle_file_tree_dir_expanded`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `ensure_file_tree_dir_expanded`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `title`：WindowId与当前领域主题/标题的Iced回调投影。
  - `title_for_window`：WindowId与当前领域主题/标题的Iced回调投影。
  - `current_file_index`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `current_file_tree_model`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `refresh_search_panel_file_cache`：串联workspace和输入建议领域，更新算法已在领域。
  - `cached_search_panel_file_results`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `refresh_file_search_cache`：串联workspace和输入建议领域，更新算法已在领域。
  - `cached_file_search_entries`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `set_file_index`：串联workspace和输入建议领域，更新算法已在领域。
  - `set_file_tree_snapshot`：串联workspace和输入建议领域，更新算法已在领域。
  - `has_file_index`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `can_open_external`：给真实调用者提供领域状态局部借用或直接委托已有领域函数。
  - `push_notification`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `set_localized_input_error`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `render_input_error`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `push_localized_notification`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `show_toast`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `show_localized_toast`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `show_success_toast`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `show_error_toast`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `show_info_toast`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `show_warning_toast`：局部状态借用/领域调用与宿主HideToast消息计时、跨域错误字段投影。
  - `theme`：WindowId与当前领域主题/标题的Iced回调投影。
  - `theme_for_window`：WindowId与当前领域主题/标题的Iced回调投影。
  - `window_style`：WindowId与当前领域主题/标题的Iced回调投影。
  - `effective_editor_theme`：WindowId与当前领域主题/标题的Iced回调投影。
  - `effective_editor_theme_ref`：WindowId与当前领域主题/标题的Iced回调投影。

- window.rs：release_main_window/reopen_main_window/restore_main_window_state/hide_main_window/show_main_window逐个涉及Iced窗口ID或主线程句柄，故保留。
- native.rs：preference_path注入项目目录；close_to_tray_enabled/save_close_to_tray仅传入路径；poll_tray映射宿主枚举；refresh_tray_providers组合provider状态与shutdown标志；其余是领域API导出。
- tray_switch.rs：start获取经过认证的当前客户端并启动具体Task；finish在领域代际接受后回读当前认证scope，再装配toast；不把服务身份检查从宿主偷偷移除。
- app_subscription.rs：terminal_subscription/cron_history_subscription映射领域订阅；needs_activity_animation_tick只投影输入；map_global_event/adapt_global_event仅适配语义动作；subscription组合已有recipes；collect_agent_subscription_requests/workflow_run_subscription_request委托领域；workflow_run_stream注入网关客户端并映射事件；agent_stream_subscription绑定Iced订阅身份。
- 恢复窗口Harness::{begin,update,check_terminal,subscription,view,run}与插件Harness::{update,action,arm,view,subscription,run}是实际主线程窗口验收：保留完整事件序列、截图、字体整形、双主题、网关取消确认/调用计数、显式安装和不自动执行断言。它们不实现领域校验/状态算法；移走它们需要复制App或反向加载宿主，故作为真实系统边界保留，不改名逃避统计。

- build.rs：png_dimensions/write_png_ico/rc_quoted_path/try_compile_windows_resource/embed_windows_icon/main均处理编译期资源格式、资源编译器调用或链接入口，完整保留在可执行程序构建脚本。

## 完整旧断言映射

以下按固定baseline源码逐个断言登记。保留同名用例的断言语义，领域结果由Task输出转换为对应本地消息或显式效果；没有把未运行的断言记为通过。拆分用例同时列出承接的领域与宿主边界。

### crates/vw-desktop/src/app/desktop_integration_29_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `desktop_integration_29_preserves_expected_behavior`：`assert_eq!(toast.id, original.id)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |
| 2 | `desktop_integration_29_preserves_expected_behavior`：`assert_eq!(toast.kind, ToastKind::Success)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |
| 3 | `desktop_integration_29_preserves_expected_behavior`：`assert_eq!(toast.message, original.message)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |
| 4 | `desktop_integration_29_preserves_expected_behavior`：`assert_eq!(app.next_toast_id, next_id)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |
| 5 | `desktop_integration_29_preserves_expected_behavior`：`assert!(!app.task_completion_notification)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |
| 6 | `desktop_integration_29_reports_failure_when_no_toast_is_visible`：`assert_eq!(toast.kind, ToastKind::Error)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |
| 7 | `desktop_integration_29_reports_failure_when_no_toast_is_visible`：`assert!(toast.message.contains("permission denied"))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_29_tests.rs::notification_permission_failure_disables_setting_without_replacing_feedback` |

### crates/vw-desktop/src/app/desktop_integration_46_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `desktop_integration_46_preserves_expected_behavior`：`assert_eq!(app.last_copied_code_hash, None)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_46_tests.rs` |
| 2 | `desktop_integration_46_preserves_expected_behavior`：`assert!(app.last_copy_time.is_none())` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_46_tests.rs` |
| 3 | `desktop_integration_46_preserves_expected_behavior`：`assert_eq!(app.active_toast.as_ref().unwrap().kind, ToastKind::Error)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/desktop_integration_46_tests.rs` |
| 4 | `desktop_integration_46_late_failure_preserves_newer_copy_feedback`：`assert_eq!(app.last_copied_code_hash, Some(47))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |

### crates/vw-desktop/src/app/desktop_integration_47_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `desktop_integration_47_preserves_expected_behavior`：`assert!(app.desktop_update.release.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/release.rs` |
| 2 | `desktop_integration_47_preserves_expected_behavior`：`assert!(!app.desktop_update.installable())` | `crates/libs/desktop-update/src/host_module_41_tests/release.rs` |
| 3 | `desktop_integration_47_download_uses_current_snapshot`：`assert!(app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/release.rs` |
| 4 | `desktop_integration_47_download_uses_current_snapshot`：`assert_eq!(app.desktop_update.release.as_ref(), Some(&snapshot))` | `crates/libs/desktop-update/src/host_module_41_tests/release.rs` |
| 5 | `desktop_integration_47_download_uses_current_snapshot`：`assert!(error.contains("No desktop installers available"), "{error}")` | `crates/libs/desktop-update/src/host_module_41_tests/release.rs` |

### crates/vw-desktop/src/app/desktop_integration_48_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `desktop_integration_48_preserves_expected_behavior`：`assert_eq!(app.app_update_progress, Some(progress.clone()))` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 2 | `desktop_integration_48_preserves_expected_behavior`：`assert_eq!(app.app_update_progress, Some(progress))` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 3 | `desktop_integration_48_preserves_expected_behavior`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 4 | `desktop_integration_48_preserves_expected_behavior`：`assert!(app.app_update_progress_rx.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 5 | `desktop_integration_48_preserves_expected_behavior`：`assert!(app.cli_install_modal_show_install_action)` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 6 | `desktop_integration_48_preserves_expected_behavior`：`assert_eq!(app.desktop_update.release, Some(release))` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 7 | `desktop_integration_48_preserves_expected_behavior`：`assert!(retry.units() > 0)` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 8 | `desktop_integration_48_preserves_expected_behavior`：`assert!(app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 9 | `desktop_integration_48_preserves_expected_behavior`：`assert!(app.app_update_progress.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |
| 10 | `desktop_integration_48_preserves_expected_behavior`：`assert!(app.app_update_progress_rx.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/progress.rs` |

### crates/vw-desktop/src/app/desktop_integration_49_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `desktop_integration_49_preserves_expected_behavior`：`assert!(task.units() > 0)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 2 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.download_abort.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 3 | `desktop_integration_49_preserves_expected_behavior`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 4 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.ready.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 5 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.download_abort.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 6 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.app_update_progress_rx.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 7 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.cli_install_modal_show_install_action)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 8 | `desktop_integration_49_preserves_expected_behavior`：`assert!(!app.cli_install_modal_is_checking_update)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 9 | `desktop_integration_49_preserves_expected_behavior`：`assert_eq!(app.desktop_update.release, Some(release))` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 10 | `desktop_integration_49_preserves_expected_behavior`：`assert!(retry.units() > 0)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 11 | `desktop_integration_49_preserves_expected_behavior`：`assert_ne!(retry_request, cancelled_request)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 12 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 13 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.download_abort.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 14 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.app_update_progress_rx.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 15 | `desktop_integration_49_preserves_expected_behavior`：`assert!(!app.cli_install_modal_message.contains("old download"))` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 16 | `desktop_integration_49_preserves_expected_behavior`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 17 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.download_abort.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 18 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.cli_install_modal_show_install_action)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |
| 19 | `desktop_integration_49_preserves_expected_behavior`：`assert!(app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/cancel.rs` |

### crates/vw-desktop/src/app/impls/app_basic_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `app_basic_tests_keeps_planned_coverage_targets`：`assert!(source_declares_symbol(name), "expected source to declare coverage target {name}")` | `crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 2 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(!app.is_diff_file_expanded("src/lib.rs"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 3 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert_eq!(app.expanded_files, ["src/lib.rs"])` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 4 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(app.is_diff_file_expanded("src/lib.rs"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 5 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(app.expanded_files.is_empty())` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 6 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(!app.is_diff_file_expanded("src/lib.rs"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 7 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert_eq!(app.expanded_files, ["README.md", "src/main.rs"])` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 8 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(app.is_diff_file_expanded("README.md"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 9 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert_eq!(app.expanded_files, ["Cargo.toml"])` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 10 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(app.is_diff_file_expanded("Cargo.toml"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 11 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(!app.is_diff_file_expanded("README.md"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 12 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(app.expanded_files.is_empty())` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 13 | `diff_file_expansion_helpers_keep_order_and_set_in_sync`：`assert!(app.expanded_files_set.is_empty())` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs` |
| 14 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert!(!app.is_file_tree_dir_expanded("src"))` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 15 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert_eq!(app.file_tree_expanded, ["src"])` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 16 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert!(app.is_file_tree_dir_expanded("src"))` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 17 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert!(!app.is_file_tree_dir_expanded("src"))` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 18 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert!(app.file_tree_expanded.is_empty())` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 19 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert_eq!(app.file_tree_expanded, ["crates"])` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 20 | `file_tree_expansion_helpers_toggle_and_deduplicate`：`assert!(app.is_file_tree_dir_expanded("crates"))` | `crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 21 | `expansion_toggles_clear_membership_when_ordered_entries_are_absent`：`assert!(!app.is_diff_file_expanded("src/lib.rs"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs`<br>`crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 22 | `expansion_toggles_clear_membership_when_ordered_entries_are_absent`：`assert!(app.expanded_files.is_empty())` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs`<br>`crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 23 | `expansion_toggles_clear_membership_when_ordered_entries_are_absent`：`assert!(!app.is_file_tree_dir_expanded("src"))` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs`<br>`crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 24 | `expansion_toggles_clear_membership_when_ordered_entries_are_absent`：`assert!(app.file_tree_expanded.is_empty())` | `crates/libs/desktop-git-ui/src/host_module_41_tests.rs`<br>`crates/libs/desktop-workspace/src/host_module_41_tests.rs` |
| 25 | `title_covers_each_screen_and_pet_window_override`：`assert_eq!(app.title(), title)` | `crates/vw-desktop/src/app/impls/app_basic_tests.rs`<br>`crates/libs/desktop-navigation/src/host_module_41_tests.rs::title_covers_each_screen` |
| 26 | `title_covers_each_screen_and_pet_window_override`：`assert_eq!(app.title_for_window(pet_window), "VibeWindow Pet")` | `crates/vw-desktop/src/app/impls/app_basic_tests.rs`<br>`crates/libs/desktop-navigation/src/host_module_41_tests.rs::title_covers_each_screen` |
| 27 | `title_covers_each_screen_and_pet_window_override`：`assert_eq!(app.title_for_window(main_window), "识矩 - 项目")` | `crates/vw-desktop/src/app/impls/app_basic_tests.rs`<br>`crates/libs/desktop-navigation/src/host_module_41_tests.rs::title_covers_each_screen` |
| 28 | `file_index_cache_tracks_project_results_and_tree_model`：`assert!(app.has_file_index("/workspace"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 29 | `file_index_cache_tracks_project_results_and_tree_model`：`assert_eq!(app.current_file_index(), ["src/lib.rs", "src/main.rs", "README.md", "tests/app_basic.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 30 | `file_index_cache_tracks_project_results_and_tree_model`：`assert!(app.current_file_tree_model().is_some())` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 31 | `file_index_cache_tracks_project_results_and_tree_model`：`assert_eq!(app.cached_search_panel_file_results(), ["src/lib.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 32 | `file_index_cache_tracks_project_results_and_tree_model`：`assert!(app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 33 | `file_index_cache_tracks_project_results_and_tree_model`：`assert!(app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/lib.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 34 | `file_index_cache_tracks_project_results_and_tree_model`：`assert_eq!(app.cached_search_panel_file_results(), cached_results)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 35 | `file_index_cache_tracks_project_results_and_tree_model`：`assert!(app.current_file_index().is_empty())` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 36 | `file_index_cache_tracks_project_results_and_tree_model`：`assert!(app.current_file_tree_model().is_none())` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 37 | `search_panel_file_cache_limits_to_first_eight_matches`：`assert_eq!(app.cached_search_panel_file_results().len(), 8)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 38 | `search_panel_file_cache_limits_to_first_eight_matches`：`assert_eq!(app.cached_search_panel_file_results()[0], "src/file_0.rs")` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 39 | `search_panel_file_cache_limits_to_first_eight_matches`：`assert_eq!(app.cached_search_panel_file_results()[7], "src/file_7.rs")` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 40 | `search_caches_keep_entries_when_cache_keys_match`：`assert_eq!(app.cached_search_panel_file_results(), ["kept-panel-result"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 41 | `search_caches_keep_entries_when_cache_keys_match`：`assert_eq!(app.cached_file_search_entries().len(), 1)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 42 | `search_caches_keep_entries_when_cache_keys_match`：`assert_eq!(app.cached_file_search_entries()[0].path, "kept-file-result")` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 43 | `search_caches_keep_entries_when_cache_keys_match`：`assert!(app.cached_file_search_entries()[0].is_dir)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 44 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert_eq!(app.search_panel_file_cache_query, "src/lib")` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 45 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert_eq!(app.cached_search_panel_file_results(), ["src/lib.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 46 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert!(app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/lib.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 47 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert_eq!(app.cached_search_panel_file_results(), ["src/library.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 48 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert!(app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/library.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 49 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert!(!app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/lib.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 50 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert_eq!(app.search_panel_file_cache_revision, 11)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 51 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert_eq!(app.cached_search_panel_file_results(), ["src/lib_new.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 52 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert_eq!(app.file_search_cache_revision, 11)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 53 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert!(app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/lib_new.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 54 | `search_caches_invalidate_for_query_project_and_revision_changes`：`assert!(!app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/library.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 55 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert_eq!(app.file_index_revision, 0)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 56 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert_eq!(app.current_file_index(), ["src/new.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 57 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert!(app.current_file_tree_model().is_some())` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 58 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert_eq!(app.search_panel_file_cache_revision, 0)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 59 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert_eq!(app.cached_search_panel_file_results(), ["src/new.rs"])` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 60 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert_eq!(app.file_search_cache_revision, 0)` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 61 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert!(app.cached_file_search_entries().iter().any(\|entry\| entry.path == "src/new.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 62 | `set_file_index_overwrites_existing_files_and_wraps_revision`：`assert!(!app.cached_file_search_entries().iter().any(\|entry\| entry.path == "old.rs"))` | `crates/libs/desktop-navigation/src/host_module_41_cache_tests.rs` |
| 63 | `can_open_external_defaults_false_and_uses_cached_value`：`assert!(!app.can_open_external(ExternalOpenApp::VSCode))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 64 | `can_open_external_defaults_false_and_uses_cached_value`：`assert!(app.can_open_external(ExternalOpenApp::VSCode))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 65 | `can_open_external_defaults_false_and_uses_cached_value`：`assert!(!app.can_open_external(ExternalOpenApp::Cursor))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 66 | `push_notification_assigns_incrementing_ids`：`assert_eq!(app.notifications.len(), 2)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 67 | `push_notification_assigns_incrementing_ids`：`assert_eq!(app.notifications[0].id, 7)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 68 | `push_notification_assigns_incrementing_ids`：`assert_eq!(app.notifications[0].message, "first")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 69 | `push_notification_assigns_incrementing_ids`：`assert_eq!(app.notifications[1].id, 8)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 70 | `push_notification_assigns_incrementing_ids`：`assert_eq!(app.notifications[1].message, "second")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 71 | `push_notification_assigns_incrementing_ids`：`assert_eq!(app.next_notification_id, 9)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 72 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.id), Some(3))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 73 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.kind), Some(ToastKind::Success))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 74 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.message.as_str()), Some("saved"))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 75 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.id), Some(4))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 76 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.kind), Some(ToastKind::Error))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 77 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.id), Some(5))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 78 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.kind), Some(ToastKind::Info))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 79 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.id), Some(6))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 80 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.active_toast.as_ref().map(\|toast\| toast.kind), Some(ToastKind::Warning))` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 81 | `toast_helpers_store_kind_message_and_increment_id`：`assert_eq!(app.next_toast_id, 7)` | `crates/libs/desktop-ui-controls/src/host_module_41_tests.rs` |
| 82 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.theme().to_string(), Theme::Dark.to_string())` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 83 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.effective_editor_theme().to_string(), Theme::Dark.to_string())` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 84 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.effective_editor_theme_ref().to_string(), Theme::Dark.to_string())` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 85 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.effective_editor_theme().to_string(), Theme::Light.to_string())` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 86 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.effective_editor_theme_ref().to_string(), Theme::Light.to_string())` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 87 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.theme_for_window(main_window).to_string(), Theme::Dark.to_string())` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 88 | `theme_helpers_select_app_editor_and_pet_themes`：`assert_eq!(app.theme_for_window(pet_window).to_string(), "VibeWindow Pet")` | `crates/libs/desktop-settings-appearance/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_basic_tests.rs` |
| 89 | `mini_companion_native_theme_clears_transparently_in_both_modes`：`assert_eq!(App::window_style(&app, &mini_theme).background_color, iced::Color::TRANSPARENT)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 90 | `mini_companion_native_theme_clears_transparently_in_both_modes`：`assert_eq!(mini_theme.base().background_color, theme.base().background_color)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 91 | `mini_companion_native_theme_clears_transparently_in_both_modes`：`assert_eq!(mini_theme.base().text_color, theme.base().text_color)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 92 | `mini_companion_native_theme_clears_transparently_in_both_modes`：`assert_eq!(app.theme_for_window(main).base().background_color, theme.base().background_color)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 93 | `mini_companion_native_theme_clears_transparently_in_both_modes`：`assert_eq!(mini_theme.extended_palette().background.strong.color, theme.extended_palette().background.strong.color)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |

### crates/vw-desktop/src/app/impls/app_subscription_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `map_global_event_maps_pointer_and_file_events`：`assert!(matches!(moved, Some(Message::View(message::ViewMessage::PointerMoved(12.5, 33.0)))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 2 | `map_global_event_maps_pointer_and_file_events`：`assert!(matches!( hovered, Some(Message::View(message::ViewMessage::HoveredFilePath(path))) if path == "/tmp/file.txt" ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 3 | `map_global_event_maps_pointer_and_file_events`：`assert!(matches!(left, Some(Message::View(message::ViewMessage::HoveredFilesLeft))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 4 | `map_global_event_maps_mouse_and_window_events`：`assert!(matches!(released, Some(Message::View(message::ViewMessage::GlobalMouseReleased))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 5 | `map_global_event_maps_mouse_and_window_events`：`assert!(matches!(cursor_left, Some(Message::View(message::ViewMessage::GlobalCursorLeft))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 6 | `map_global_event_maps_mouse_and_window_events`：`assert!(matches!( resized, Some(Message::View(message::ViewMessage::WindowResized(message_id, 800.0, 600.0))) if message_id == id ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 7 | `map_global_event_maps_mouse_and_window_events`：`assert!(matches!( moved, Some(Message::View(message::ViewMessage::WindowMoved(message_id, 4.0, 7.0))) if message_id == id ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 8 | `map_global_event_maps_mouse_and_window_events`：`assert!(matches!( close_requested, Some(Message::View(message::ViewMessage::CloseRequested(message_id))) if message_id == id ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 9 | `map_global_event_maps_mouse_and_window_events`：`assert!(matches!( closed, Some(Message::View(message::ViewMessage::WindowClosed(message_id))) if message_id == id ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 10 | `map_global_event_maps_mouse_and_window_events`：`assert!(unfocused_messages.iter().any(\|message\| matches!( message, Message::View(message::ViewMessage::WindowUnfocused(window_id)) if *window_id == id )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 11 | `map_global_event_maps_mouse_and_window_events`：`assert!(unfocused_messages.iter().any(\|message\| matches!( message, Message::Preview(message::PreviewMessage::WindowUnfocused) )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 12 | `map_global_event_maps_keyboard_shortcuts`：`assert!(matches!( messages.first(), Some(Message::TaskBoard(message::TaskBoardMessage::ContextMenuClosed)) ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 13 | `map_global_event_maps_keyboard_shortcuts`：`assert!(messages .iter() .any(\|message\| matches!(message, Message::Editor(message::EditorMessage::CloseSearch))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 14 | `map_global_event_maps_keyboard_shortcuts`：`assert!(messages.iter().any(\|message\| matches!( message, Message::View(message::ViewMessage::GlobalKeyPressed( keyboard::Key::Named(keyboard::key::Named::Escape), _ )) )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 15 | `map_global_event_maps_keyboard_shortcuts`：`assert!(messages.iter().any(\|message\| matches!( message, Message::Preview(message::PreviewMessage::LspCompletionClosed) )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 16 | `map_global_event_maps_keyboard_shortcuts`：`assert!(messages.iter().any(\|message\| matches!( message, Message::Project(message::ProjectMessage::FileTreeRenameCancel) )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 17 | `map_global_event_maps_keyboard_shortcuts`：`assert!(matches!(search, Some(Message::Editor(message::EditorMessage::OpenSearch))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 18 | `map_global_event_maps_keyboard_shortcuts`：`assert!(matches!(replace, Some(Message::Editor(message::EditorMessage::OpenReplace))))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 19 | `map_global_event_maps_keyboard_shortcuts`：`assert!(matches!(shortcut, Some(Message::Project(message)) if std::mem::discriminant(&message) == std::mem::discriminant(&expected)))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 20 | `map_global_event_maps_keyboard_shortcuts`：`assert!(shifted.is_none())` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 21 | `map_global_event_handles_escape_before_captured_filter`：`assert!(messages.iter().any(\|message\| matches!( message, Message::View(message::ViewMessage::GlobalKeyPressed( keyboard::Key::Named(keyboard::key::Named::Escape), modifiers )) if *modifiers == Modifiers::COMMAND )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 22 | `map_global_event_ignores_captured_non_shortcut_keys`：`assert!(captured.is_none())` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 23 | `map_global_event_ignores_captured_non_shortcut_keys`：`assert!(ignored.is_none())` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 24 | `map_global_event_ignores_captured_non_shortcut_keys`：`assert!(matches!( uncaptured, Some(Message::View(message::ViewMessage::GlobalKeyPressed( keyboard::Key::Character(_), modifiers ))) if modifiers == Modifiers::SHIFT ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 25 | `map_global_event_ignores_unhandled_mouse_and_forwards_window_focus`：`assert!(pressed.is_none())` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 26 | `map_global_event_ignores_unhandled_mouse_and_forwards_window_focus`：`assert!(matches!( focused, Some(Message::View(message::ViewMessage::WindowFocused(window))) if window == id ))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 27 | `requesting_sessions_animate_on_conversation_screen_and_stop_when_idle`：`assert!(!needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 28 | `requesting_sessions_animate_on_conversation_screen_and_stop_when_idle`：`assert!(needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 29 | `requesting_sessions_animate_on_conversation_screen_and_stop_when_idle`：`assert!(!needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 30 | `requesting_sessions_animate_on_conversation_screen_and_stop_when_idle`：`assert!(needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 31 | `requesting_sessions_animate_on_conversation_screen_and_stop_when_idle`：`assert!(!needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 32 | `requesting_sessions_animate_on_conversation_screen_and_stop_when_idle`：`assert!(needs_activity_animation_tick(&app, true))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 33 | `workflow_run_subscription_is_active_only_while_running`：`assert!(workflow_run_subscription_request(&app).is_none())` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 34 | `workflow_run_subscription_is_active_only_while_running`：`assert_eq!(request.uuid, "workflow-1")` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 35 | `workflow_run_subscription_is_active_only_while_running`：`assert_eq!(request.query, "hello")` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 36 | `workflow_run_subscription_is_active_only_while_running`：`assert_eq!(request.inputs.get("topic"), Some(&serde_json::json!("rust")))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 37 | `workflow_run_subscription_is_active_only_while_running`：`assert_eq!(request.user, "desktop-workflow")` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 38 | `workflow_run_subscription_is_active_only_while_running`：`assert!(workflow_run_subscription_request(&app).is_none())` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 39 | `project_subscription_does_not_schedule_periodic_task_or_session_refresh`：`assert!(!source.contains(".map(\|_\| Message::TaskBoard(message::TaskBoardMessage::LoadTasks))"))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 40 | `project_subscription_does_not_schedule_periodic_task_or_session_refresh`：`assert!(!source.contains("ProjectMessage::ProjectSessionsRefreshTick"))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 41 | `acp_diagnostic_animation_ticks_only_while_settings_are_visible_and_test_is_running`：`assert!(super::needs_activity_animation_tick(&app, false))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 42 | `acp_diagnostic_animation_ticks_only_while_settings_are_visible_and_test_is_running`：`assert!(!super::needs_activity_animation_tick(&app, false))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 43 | `acp_diagnostic_animation_ticks_only_while_settings_are_visible_and_test_is_running`：`assert!(super::needs_activity_animation_tick(&app, false))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 44 | `acp_diagnostic_animation_ticks_only_while_settings_are_visible_and_test_is_running`：`assert!(!super::needs_activity_animation_tick(&app, false))` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 45 | `file_tree_move_drop_escape_and_unfocus_cancel_tree_origin`：`assert!(messages.iter().any(\|message\| matches!( message, Message::Project(message::ProjectMessage::FileTreeDragCancel) )))` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 46 | `file_tree_external_drop_native_event_is_observed_without_hover`：`assert!(super::map_global_event( iced::Event::Window(iced::window::Event::FileDropped("/tmp/文件夹".into())), iced::event::Status::Ignored, iced::window::Id::unique(), ) .is_some())` | `crates/libs/desktop-navigation/src/host_module_41_event_tests.rs` |
| 47 | `mini_companion_hidden_stops_avatar_and_expansion_ticks`：`assert!(needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 48 | `mini_companion_hidden_stops_avatar_and_expansion_ticks`：`assert!(!needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 49 | `mini_companion_hidden_stops_avatar_and_expansion_ticks`：`assert!(!needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 50 | `mini_companion_hidden_stops_avatar_and_expansion_ticks`：`assert!(needs_activity_animation_tick(&app, false))` | `crates/libs/desktop-platform/src/host_module_41_tests.rs`<br>`crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 51 | `cron_history_polling_stops_when_page_is_hidden`：`assert_eq!(into_recipes(cron_history_subscription(&app)).len(), 1)` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 52 | `cron_history_polling_stops_when_page_is_hidden`：`assert!(into_recipes(cron_history_subscription(&app)).is_empty())` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 53 | `cron_history_polling_stops_when_page_is_hidden`：`assert!(into_recipes(cron_history_subscription(&app)).is_empty())` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |
| 54 | `cron_history_polling_stops_when_page_is_hidden`：`assert!(into_recipes(cron_history_subscription(&app)).is_empty())` | `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` |

### crates/vw-desktop/src/app/installer_update_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(app.cli_install_modal_message.contains("Ubuntu/Debian"))` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 2 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(app.cli_install_modal_message.contains("DEB"))` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 3 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert_eq!(app.cli_install_modal_title, "自动更新启动失败")` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 4 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(app.active_toast.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 5 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 6 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(!app.shutdown_requested)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 7 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(app.desktop_update.ready.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 8 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(app.shutdown_requested)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 9 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 10 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(app.desktop_update.ready.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 11 | `installer_update_linux_authorizes_install_and_allows_retry`：`assert!(ready.path().exists())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 12 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert_eq!(app.update(Message::View(ViewMessage::OpenAppInstallerDirectory)).units(), 0)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 13 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert_eq!(app.update(Message::View(ViewMessage::OpenAppInstallerDirectory)).units(), 0)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 14 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert!(app.active_toast.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 15 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert!(!app.shutdown_requested)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 16 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 17 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert_eq!(app.desktop_update.ready.as_ref().unwrap().path(), ready.path())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 18 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert!(!app.shutdown_requested)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 19 | `installer_directory_failure_preserves_package_and_does_not_exit`：`assert!(ready.path().exists())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 20 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert_eq!(app.cli_install_modal_title, "安装包已就绪")` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 21 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.cli_install_modal_message.contains(ready.path().to_str().unwrap()))` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 22 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert_eq!(app.desktop_update.ready.as_ref().unwrap().path(), ready.path())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 23 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 24 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.cli_install_modal_is_checking_update)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 25 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert_eq!(app.cli_install_modal_title, "更新中…")` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 26 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.cli_install_modal_message.contains("正在校验安装包"))` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 27 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert_eq!(duplicate.units(), 0)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 28 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(!app.desktop_update.busy)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 29 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(!app.cli_install_modal_is_checking_update)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 30 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(!app.shutdown_requested)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 31 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert_eq!(app.cli_install_modal_title, "自动更新启动失败")` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 32 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.active_toast.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 33 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.cli_install_modal_message.contains(ready.path().to_str().unwrap()))` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 34 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.desktop_update.ready.is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 35 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(app.shutdown_requested)` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 36 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert_eq!(app.cli_install_modal_title, "安装包已就绪")` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 37 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(!app.cli_install_modal_message.contains("更新完成"))` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 38 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(ready.path().exists())` | `crates/libs/desktop-update/src/host_module_41_tests/installer.rs` |
| 39 | `app_update_ready_installer_survives_close_open_failure_and_retry`：`assert!(pixels.chunks_exact(4).any(\|pixel\| pixel[3] > 0))` | `crates/libs/desktop-navigation/src/host_module_41_render_tests.rs::app_update_ready_installer_draws_all_loading_frames_in_both_themes` |

### crates/vw-desktop/src/app/native_menu/actions_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `resolve_project_file_actions_match_topbar_messages`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::FileNewSession), Some(Message::View(message::ViewMessage::ProjectFileNewSession)) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 2 | `resolve_project_file_actions_match_topbar_messages`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::FileSave), Some(Message::Preview(message::PreviewMessage::SaveFile)) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 3 | `resolve_project_edit_clipboard_actions_use_focus_aware_shortcuts`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::EditCut), Some(Message::CutShortcut) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 4 | `resolve_project_edit_clipboard_actions_use_focus_aware_shortcuts`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::EditCopy), Some(Message::CopyShortcut) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 5 | `resolve_project_edit_clipboard_actions_use_focus_aware_shortcuts`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::EditPaste), Some(Message::PasteShortcut) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 6 | `resolve_help_actions_remain_available`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::HelpAbout), Some(Message::View(message::ViewMessage::ToggleAboutModal)) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 7 | `resolve_help_actions_remain_available`：`assert_eq!(help_section().0, "帮助")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 8 | `resolve_non_project_edit_actions_are_unavailable`：`assert!(resolve_native_menu_action(&app, NativeMenuAction::EditCopy).is_none())` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 9 | `resolve_non_project_edit_actions_are_unavailable`：`assert!(matches!( resolve_native_menu_action(&app, NativeMenuAction::FileGoHome), Some(Message::View(message::ViewMessage::GoHome)) ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 10 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert_eq!(sections.len(), 3)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 11 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert_eq!(sections[0].0, "文件")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 12 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert_eq!(sections[1].0, "编辑")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 13 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert_eq!(sections[2].0, "视图")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 14 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert!(!sections.iter().any(\|(title, _)\| *title == "浏览器"))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 15 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert_eq!(work_view.len(), 4)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 16 | `menu_sections_exclude_browser_and_follow_workspace_mode`：`assert!(matches!( &work_view[1], MenuEntry::Item { action: Some(NativeMenuAction::ViewWorkOverview), .. } ))` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 17 | `snapshot_includes_open_window_tabs_for_project_switch`：`assert_eq!(snapshot.open_window_tabs.len(), 2)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 18 | `snapshot_includes_open_window_tabs_for_project_switch`：`assert_eq!(snapshot.open_window_tabs[0].id, "home")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 19 | `snapshot_includes_open_window_tabs_for_project_switch`：`assert!(!snapshot.open_window_tabs[0].active)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 20 | `snapshot_includes_open_window_tabs_for_project_switch`：`assert_eq!(snapshot.open_window_tabs[1].id, "project_a")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 21 | `snapshot_includes_open_window_tabs_for_project_switch`：`assert!(snapshot.open_window_tabs[1].active)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 22 | `snapshot_includes_open_window_tabs_for_project_switch`：`assert_eq!(snapshot.open_window_tabs[1].title, "alpha — README.md")` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 23 | `desktop_gettext_area_17_native_snapshot_tracks_language_changes`：`assert_ne!(english, traditional)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |
| 24 | `desktop_gettext_area_17_native_snapshot_tracks_language_changes`：`assert_eq!(traditional.locale, app.desktop_locale)` | `crates/libs/desktop-navigation/src/host_module_41_tests.rs` |

### crates/vw-desktop/src/app/plugin_compat_t25_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `codex_plugin_compat_t25_renders_and_cancels_server_form`：`assert!(cancel.units() > 0)` | `crates/libs/desktop-chat-view/src/host_module_41_tests.rs` |
| 2 | `codex_plugin_compat_t25_renders_and_cancels_server_form`：`assert_eq!(chat::update(&mut app, ChatMessage::QuestionReject).units(), 0)` | `crates/libs/desktop-chat-view/src/host_module_41_tests.rs` |
| 3 | `codex_plugin_compat_t25_rejects_invalid_input`：`assert!(chat::update(&mut app, ChatMessage::QuestionReject).units() > 0, "invalid form must remain open for correction or cancellation")` | `crates/libs/desktop-chat-view/src/host_module_41_tests.rs` |
| 4 | `codex_plugin_compat_t25_preserves_isolation`：`assert_eq!(chat::update(&mut app, ChatMessage::QuestionSubmit).units(), 0)` | `crates/libs/desktop-chat-view/src/host_module_41_tests.rs` |
| 5 | `codex_plugin_compat_t25_preserves_isolation`：`assert_eq!(second.session_id, "second")` | `crates/libs/desktop-chat-view/src/host_module_41_tests.rs` |
| 6 | `codex_plugin_compat_t25_preserves_isolation`：`assert_eq!(second.mcp_form.unwrap().server, "表单验收")` | `crates/libs/desktop-chat-view/src/host_module_41_tests.rs` |

### crates/vw-desktop/src/app/plugin_compat_window.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `update`：`assert!(self.started.elapsed() < Duration::from_secs(60), "native window acceptance timeout")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 2 | `update`：`assert_eq!(items.len(), 1)` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 3 | `update`：`assert!(items[0] .dependency_checks .iter() .any(\|c\| c.status == "missing_host" && c.dependency == "node"))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 4 | `update`：`assert_eq!(items[0].hook_review, "pending")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 5 | `update`：`assert!(!self.target.exists())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 6 | `update`：`assert!(items.iter().any(\|p\| p.id == "newcomer" && p.version == "2"))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 7 | `update`：`assert!(items.iter().any(\|p\| p.id == "newcomer" && p.version == "3"))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 8 | `update`：`assert!(result.is_err())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 9 | `update`：`assert!(result.is_ok())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 10 | `update`：`assert!(!self.target.exists())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 11 | `update`：`assert!(result.is_ok())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 12 | `update`：`assert!(!self.target.exists())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 13 | `update`：`assert_eq!(preview.old_version.as_deref(), Some("2"))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 14 | `update`：`assert_eq!(preview.version.as_deref(), Some("3"))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 15 | `update`：`assert_eq!(preview.previous_capabilities.as_ref().unwrap().mcp_servers, 0)` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 16 | `update`：`assert_eq!(preview.capabilities.mcp_servers, 1)` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 17 | `update`：`assert!(result.is_ok())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 18 | `update`：`assert!(!self.target.exists())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 19 | `update`：`assert!(self.app.skills_settings.plugin_install.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 20 | `update`：`assert!(result.is_ok(), "{result:?}")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 21 | `update`：`assert!(self.app.skills_settings.plugin_install.is_none())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 22 | `update`：`assert!(self.target.is_dir())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 23 | `update`：`assert!(!self.root.join("MUST_NOT_RUN").exists())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 24 | `update`：`assert!(image.rgba.chunks_exact(4).any(\|p\| p[0] != p[1]))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 25 | `update`：`assert!(!self.target.exists())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 26 | `run`：`assert!(matches!(mode, 55 \| 56))` | `crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 27 | `run`：`assert!(root.is_absolute() && root.is_dir())` | `crates/vw-desktop/src/app/plugin_compat_window.rs` |
| 28 | `run`：`assert!(Instant::now() < deadline, "gateway fixture readiness timeout")` | `crates/libs/desktop-platform/src/acceptance.rs::BackendFixture::ready` |

### crates/vw-desktop/src/app/recovery_window_tests.rs

| 序号 | 原断言 | 新位置（或保留的系统契约） |
|---:|---|---|
| 1 | `update`：`assert_eq!(*id, self.scenario as u64 + 100)` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 2 | `update`：`assert!(matches!(result, Ok(true)), "Gateway must acknowledge explicit cancellation: {result:?}")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 3 | `update`：`assert!(self.waiting_seen, "recovery must follow waiting")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 4 | `update`：`assert!(self.app.current_session_is_requesting())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 5 | `update`：`assert!(crate::app::components::chat_panel::message_view::recovery_notice_text( &self.app ) .is_some())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 6 | `update`：`assert!(self.started.elapsed() < Duration::from_secs(75), "window scenario timed out")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 7 | `update`：`assert!(screenshot.rgba.chunks_exact(4).any(\|pixel\| pixel[0] != pixel[1]))` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 8 | `update`：`assert!(!self.app.current_session_is_requesting())` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 9 | `update`：`assert!(self.app.current_session_runtime_ref().unwrap().model_recovery.is_none(), "cancel button must clear recovery state")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 10 | `update`：`assert!(self.app.active_toast.is_none(), "cancel button must clear recovery toast")` | `crates/libs/desktop-update/src/host_module_41_tests/mod.rs`<br>`crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 11 | `check_terminal`：`assert!(!self.app.current_session_is_requesting())` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 12 | `check_terminal`：`assert!(self.app.current_session_runtime_ref().unwrap().model_recovery.is_none())` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 13 | `check_terminal`：`assert!(self.waiting_seen, "{content}")` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 14 | `check_terminal`：`assert!(content.contains("瞬态故障后恢复成功"), "{content}")` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 15 | `check_terminal`：`assert!(!self.waiting_seen)` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 16 | `check_terminal`：`assert!(content.contains("未重试"), "{content}")` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 17 | `check_terminal`：`assert!(self.waiting_seen, "{content}")` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 18 | `check_terminal`：`assert!(content.contains("已尝试 3 次"), "{content}")` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 19 | `run`：`assert_eq!(args.next().as_deref(), Some("--gateway-test-binary"), "explicit gateway fixture executable is required")` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 20 | `run`：`assert!(root.is_absolute() && root.is_dir())` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 21 | `run`：`assert!(Instant::now() < deadline, "gateway readiness timeout")` | `crates/libs/desktop-platform/src/acceptance.rs::BackendFixture::ready` |
| 22 | `run`：`assert_eq!(stats[format!("transient-{theme}")], 2)` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 23 | `run`：`assert_eq!(stats[format!("exhausted-{theme}")], 3)` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 24 | `run`：`assert_eq!(stats[format!("cancel-{theme}")], 1)` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |
| 25 | `run`：`assert_eq!(stats.get(format!("missing-{theme}")).and_then(Value::as_u64).unwrap_or(0), 0)` | `crates/vw-desktop/src/app/recovery_window_tests.rs` |

## 静态检查与以后可执行的定向命令

本轮执行git diff --check、git diff --cached --check与57个修改Rust文件的独立rustfmt语法解析（--emit stdout，不递归子模块）；静态核对mod/cfg、真实调用者、依赖方向及上述断言映射。没有运行Rust编译、Cargo metadata、测试列表、单元测试、窗口或绘制。

用户以后另行要求验证时，按所在领域运行以下命令，检查有效用例数大于0；本轮未执行：
```sh
python3 scripts/cargo_slot.py cargo test -p desktop-platform --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-update --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-workspace --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-ui-controls --lib host_module_41 -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-settings-appearance --lib host_module_41 -- --test-threads=1
```

原生窗口验收继续由原显式入口执行，必须以后单独授权，不能用源码检查声称字体整形无panic。
