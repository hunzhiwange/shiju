# 任务42 静态交付证据

静态交付完成，未编译、未测试。遵守本计划禁用全部 Cargo、单元测试及实机验证的要求；独立 rustfmt 与 git diff --check 已执行。

## 实际归属与调用链

- App → 文件树 adapter::inputs/map → desktop-file-tree::view/view_file_manager；只借用领域输入，宿主映射 ProjectMessage。删除无人调用的 find_results/menu 及绘制辅助转发挂载。selection::visible_paths 的模型回退策略迁入 FileTreeView::visible_paths，宿主仅投影 App。
- App → executor::runner（注入 gateway_client_endpoint/gateway_client）→ desktop-task-executor::execution → spawn_blocking → home 检查 → resolve_task_execution_workspace → runner。租约 guard 持有到执行返回；Fresh/ContinueExisting、错误字符串及 Web 拒绝保持原样。移除宿主内部未调用 Git 包装和六个纯导出文件，直接注册领域模块。
- 看板 panel::forms::build_agent_options → desktop-task-board::agent_activity::agent_available → desktop-task-model；settings/slot 转换与 Gateway DTO 投影测试整体迁入看板。未增加依赖。
- 项目视图仍组合跨域聊天、终端、导航、文件树、拖拽和模态层；布局算法和样式既已归 desktop-workspace。profile selection 将领域事件映射宿主 Settings 路由，project_profiles update 检查当前 apps_catalog 上下文并组合 toast，保留真实跨域边界。

## 残留审计

- task/mod.rs、models.rs、store.rs、workflow.rs：既有 API 兼容导出，实体和存储均已在 task-model/task-store；没有宿主局部算法。executor/mod.rs 是公开入口装配；git::build_review_diff_context、runner 的计划/审核/合并/网关提示/执行/重试及 worktree_admin 的 assign/reset/delete/commit 系列仅注入当前宿主网关配置。系统配置函数不能由领域反向读取。
- 文件树 adapter 负责 App 字段借用与完整局部消息映射，mod.rs 保留两种入口；model/inline_rename 仍为真实调用者需要的稳定导出，selection 只做 App 投影。tree_list 两入口由真实宿主消息渲染契约使用。
- project.rs 的 view/uses_work_layout 保留跨域布局组合和 acp_options 宿主模态；project_profiles selection::view 保留 Settings 路由翻译，message/project_profiles 的 message/update 保留客户端准入、目录身份及 toast。

## 完整测试映射

- agent_activity_tests::task_pool_employee_gateway_projection_preserves_run_identity → desktop-task-board::host_module_42_tests 同名测试；run_id/key/task_id/busy_count 四个断言原样迁移；slot/entry 夹具随迁（slot 原为未调用辅助函数，保留源码）。
- task/tests::test_module_is_wired 保留顶层宿主测试注册契约。
- file_tree/mod_tests 两个测试保留：视图投影不得改变宿主代码面板偏好；复制/剪切 clipboard 模式到 can_paste 的 App 转换。
- file_tree/tree_list_tests 三个测试保留：drop release/cleanup 两种顺序、聊天拖拽与 busy 拒绝、真实渲染事件映射至宿主 ProjectMessage 并经宿主 update 切换目录。RowScene/RowBounds/alignment_app 仅供最后一项跨域事件合同；保留 Light/Dark、四种 modifiers、中文截图与真实 SVG/text rasterize 断言。原有局部布局绘制测试已在 desktop-file-tree::desktop_split_09_tests，未删除或改写断言。
- projects_tests 全组保留宿主项目打开与 settings/skills、模型偏好异步代际、手动选择、看板重置、Git 能力、终端缓存及导航/recent edit 跨域装配；不移动 App 夹具到领域。
- views/project_tests 五例保留跨域视图组装（四种宽度与 Work/Code），覆盖宿主 diff/file-manager/terminal 状态不变。局部宽度算法已在 workspace。

以下逐名列出保留宿主用例（原名、断言与夹具不变）：

`crates/vw-desktop/src/app/components/file_tree/mod_tests.rs`：file_manager_view_construction_preserves_code_panel_preferences, clipboard_modes_project_paste_availability。

`crates/vw-desktop/src/app/components/file_tree/tree_list_tests.rs`：file_tree_drop_release_and_cleanup_orders_leave_no_stale_state, file_tree_drop_chat_sources_and_busy_operations_do_not_enable_move_state, traverse, text, text_input, new, label, icon, click, alignment_app, tree_rows_preserve_selection_open_and_toggle_messages。

`crates/vw-desktop/src/app/projects_tests.rs`：test_module_is_wired, collect_task_messages, project_settings_refresh_task_reloads_agents_and_skills_for_project, open_first_recent_project_or_home_goes_home_when_no_projects_remain, open_project_immediately_moves_agent_settings_to_new_project_scope, open_project_does_not_wait_for_chat_preferences, project_preferences_loaded, wait_for_project_preferences, project_preferences_ignore_an_earlier_visit_and_keep_the_draft, late_project_read_preserves_a_manual_model_selection, open_project_resets_git_capability_until_project_info_loads, reset_task_board_project_state_clears_project_scoped_board_runtime, switch_project_terminal_keeps_hidden_project_without_pty, switch_project_terminal_is_noop_for_matching_paths, switch_project_terminal_saves_previous_and_creates_blank_without_new_project, switch_project_terminal_creates_new_terminal_then_restores_cached_state, open_project_removes_apps_tab_creates_project_tab_and_recent_edit_name。

`crates/vw-desktop/src/app/task/tests.rs`：test_module_is_wired。

`crates/vw-desktop/src/app/views/project_tests.rs`：render_project_view_with_settings_width, view_builds_with_regular_settings_width, view_builds_with_too_small_settings_width, view_builds_with_too_large_settings_width, view_builds_with_non_finite_settings_width, work_layout_project_view_routes_each_workspace_mode。

## 逐文件 splitlines 清单

固定基线 → 当前宿主；删除文件为0。之前任务已迁入的领域位置也计入归属，不将整项净减量声称为本次新增。

| 原文件 | 基线 | 当前 | 归属/保留理由 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/file_tree/adapter.rs` | 74 | 74 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/find_results.rs` | 4 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/icons.rs` | 1 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/inline_rename.rs` | 5 | 1 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/menu.rs` | 13 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/mod.rs` | 25 | 17 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/mod_tests.rs` | 44 | 44 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/model.rs` | 1 | 1 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/native_drop.rs` | 1 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/row_layout.rs` | 1 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/selection.rs` | 10 | 5 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/tree_gutter.rs` | 1 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/tree_list.rs` | 7 | 7 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/tree_list_tests.rs` | 266 | 266 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/file_tree/widgets.rs` | 1 | 0 | desktop-file-tree；宿主仅输入/事件合同 |
| `crates/vw-desktop/src/app/components/project_profiles/selection.rs` | 23 | 23 | desktop-workspace；宿主跨域装配/合同 |
| `crates/vw-desktop/src/app/message/project_profiles.rs` | 29 | 29 | desktop-workspace；宿主跨域装配/合同 |
| `crates/vw-desktop/src/app/projects_tests.rs` | 374 | 374 | desktop-workspace；宿主跨域装配/合同 |
| `crates/vw-desktop/src/app/task/agent_activity.rs` | 31 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/agent_activity_tests.rs` | 59 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/backend_output.rs` | 3 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/command_exec.rs` | 3 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/gateway_output.rs` | 3 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/git.rs` | 66 | 13 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/mod.rs` | 51 | 48 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/process_utils.rs` | 3 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/programs.rs` | 3 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/runner.rs` | 88 | 92 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/scheduling.rs` | 1 | 0 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/executor/worktree_admin.rs` | 100 | 150 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/mod.rs` | 159 | 157 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/models.rs` | 2 | 2 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/store.rs` | 3 | 3 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/tests.rs` | 10 | 10 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/task/workflow.rs` | 2 | 2 | task-model/store/executor/board；宿主配置注入/注册 |
| `crates/vw-desktop/src/app/views/project.rs` | 128 | 128 | desktop-workspace；宿主跨域装配/合同 |
| `crates/vw-desktop/src/app/views/project_tests.rs` | 54 | 54 | desktop-workspace；宿主跨域装配/合同 |

基线合计 1649，当前 1500，宿主可归属净减少 149 行。

## 延期验证（仅用户另行要求时）

- `python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_42_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-task-executor --lib runner_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-file-tree --lib desktop_split_09_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::file_tree:: -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::projects_tests:: -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::views::project::project_tests:: -- --test-threads=1`

静态确认 cfg(test) host_module_42_tests 注册在看板 lib.rs，领域没有引用 App、宿主路径 include/path。未运行以上命令，不报告 ran/failed 或字体安全结论。
