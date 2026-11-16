# 任务34：终端面板与会话路由

静态交付；未编译、未测试。遵守本轮禁测约束，仅独立 rustfmt 与限定路径 git diff --check。

## 实际调用及残留

- 宿主 components/terminal_panel::view 仅借用 TerminalState 构建领域 ViewInput，并映射 Terminal/Batch/Hide；领域 terminal_panel::view 持有布局、绘制、菜单与字体实现。
- message/terminal::update 调用 desktop_terminal::updates::update；配置、事件、标签、重命名、分屏、内容菜单状态转换均由领域持有。宿主只连接编辑器焦点、计时、网关持久化、启动设置、toast、聊天与浏览器同步。
- event::focus_active_terminal 保留真实跨域编辑器 lose_focus 与领域 widget focus Task 装配；config.rs 只重导出领域持久化类型；tabs.rs 只映射资源回收 Task。删除冗余 rename 模块及各分支 update 转发。
- app/terminal.rs 仅领域类型导出及 loopback 授权后调用 local::open_local_directory。授权检查属于宿主系统资源边界，保持拒绝远端启动本机进程。
- App.terminal、terminal_config_persistence 与初始化已使用领域类型，无新增状态镜像或宿主实现。Shell/TerminalTheme/TerminalTab/TerminalPanes 的资源及状态仍由领域持有。
- 宿主 focus_tests 仅保留配置加载、项目切换、退出生命周期，以及真实工作区 toolbar 与聊天追加的跨域契约；无局部布局或编辑器绘制夹具。

## 完整测试映射

保留原测试名、断言及真实 PTY、渲染、IME、像素比较路径；不运行。领域夹具只包含真实 TerminalState、ConfigPersistence、相邻 Editor 和借用输入，不重建 App。宿主项目布局调用另保留真实 UserInterface 边界。

### `crates/vw-desktop/src/app/components/terminal_panel_tests.rs`

- `terminal_drag_divider_resizes_nested_panes` → host_module_34_tests/panel.rs；原断言按顺序保留。
- `terminal_drag_header_docks_panes_without_capturing_terminal_selection` → host_module_34_tests/panel.rs；原断言按顺序保留。
### `crates/vw-desktop/src/app/message/terminal/focus_tests.rs`

- `terminal_context_menu_preserves_selection_and_appends_to_chat_draft` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_focus_click_delivers_keys_in_both_directions` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_focus_background_events_preserve_editor_input` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_focus_select_and_reopen_deliver_keys_without_click` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_focus_rename_modal_does_not_type_into_terminal_or_editor` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_focus_repeated_clicks_and_keys_preserve_tab_and_pty_identity` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_ime_requests_native_composition_only_while_focused` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_ime_writes_committed_utf8_once_to_the_pty` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_fullscreen_preserves_pty_focus_and_output` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_fullscreen_clears_on_hide_project_switch_and_pty_exit` → 宿主原文件（真实 App 生命周期）；原断言按顺序保留。
- `terminal_context_menu_splits_the_clicked_pane` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_split_shortcut_only_runs_in_the_focused_terminal` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_split_toolbar_creates_independent_panes_in_each_direction` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_split_routes_input_and_closes_empty_panes` → host_module_34_tests/focus.rs；原断言按顺序保留。
- `terminal_drag_preserves_pty_identity_and_uses_new_grid_size` → host_module_34_tests/focus.rs；原断言按顺序保留。
### `crates/vw-desktop/src/app/message/terminal/maximize_tests.rs`

- `terminal_title_double_click_maximizes_and_restores_the_original_layout` → host_module_34_tests/maximize_tests.rs；原断言按顺序保留。
### `crates/vw-desktop/src/app/message/terminal/tests.rs`

- `selecting_unknown_tab_closes_context_menu_without_taking_focus` → host_module_34_tests/routing.rs；原断言按顺序保留。
- `rename_cancel_routes_to_rename_and_closes_context_menu_for_unknown_tab` → host_module_34_tests/routing.rs；原断言按顺序保留。
- `drag_started_routes_to_event_and_captures_drag_origin` → host_module_34_tests/routing.rs；原断言按顺序保留。
- `tab_context_close_routes_to_tabs_and_clears_menu_coordinates` → host_module_34_tests/routing.rs；原断言按顺序保留。

跨域断言调整：context-menu 测试在领域断言 AppendChat 的完整 selection；原“现有草稿\nselection”断言移至宿主 terminal_chat_append_effect_preserves_existing_draft，调用真实 chat::update。fullscreen 测试在领域验证同一 PTY 输入、输出及 widget 身份；真实 project::view 工具栏消息路由由宿主 workspace_terminal_toolbar_routes_fullscreen_and_preserves_widget 覆盖。reopen 的真实宿主显隐/项目路由由生命周期契约保留，领域继续验证 focus_active_terminal 的 widget 操作。

## splitlines 清单

| 原文件 | 基线 | 交付宿主残留 |
| --- | ---: | ---: |
| `crates/vw-desktop/src/app/components/terminal_panel.rs` | 29 | 29 |
| `crates/vw-desktop/src/app/components/terminal_panel_tests.rs` | 217 | 0 |
| `crates/vw-desktop/src/app/message/terminal/config.rs` | 5 | 1 |
| `crates/vw-desktop/src/app/message/terminal/event.rs` | 18 | 24 |
| `crates/vw-desktop/src/app/message/terminal/focus_tests.rs` | 1041 | 165 |
| `crates/vw-desktop/src/app/message/terminal/maximize_tests.rs` | 44 | 0 |
| `crates/vw-desktop/src/app/message/terminal/mod.rs` | 65 | 86 |
| `crates/vw-desktop/src/app/message/terminal/rename.rs` | 4 | 0 |
| `crates/vw-desktop/src/app/message/terminal/tabs.rs` | 5 | 5 |
| `crates/vw-desktop/src/app/message/terminal/tests.rs` | 71 | 0 |
| `crates/vw-desktop/src/app/terminal.rs` | 22 | 22 |

基线 1521 行，宿主残留 332 行，可归属净减少 1189 行。新增领域测试见 host_module_34_tests（行数含空行注释）。

## 注册、依赖与后续验证

lib.rs 的 cfg(test) 注册 host_module_34_tests，聚合 support/routing/native panel/native focus，focus 注册 maximize_tests。删除宿主旧测试挂载及源文件；宿主仅注册 native focus_tests。不存在 path/include 反向加载宿主源码。新增 dev-dependencies 仅复用已有 iced_runtime、desktop-editor、iced-code-editor；Cargo.lock 手工同步该包依赖，不运行 Cargo。

以后用户另行授权验证时：
- `python3 scripts/cargo_slot.py cargo test -p desktop-terminal --lib host_module_34_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::message::terminal::focus_tests -- --test-threads=1`

尚未获得编译、运行时焦点、字体整形和绘制通过证据；静态检查不替代行为验证。
