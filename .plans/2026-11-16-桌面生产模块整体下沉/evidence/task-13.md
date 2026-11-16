# 任务13：Git 面板与仓库交互静态交付

完成范围：Git页面组合、diff/复制弹窗/刷新/提交入口与整组领域测试源码归 desktop-git-ui。宿主仅持有 State，并装配跨域与系统效果。此文取代早期提交 a6a84f834 的部分检查点。

## 真实调用链与所有权

- App.git 持有 `desktop_git_ui::state::State`；72个旧字段由领域统一构造。App::new 将两个原配置布尔值原样传给 State::new；复制代码编辑器沿用原语言与初值。真实引用逐项改接 `.git`，项目 DTO 的 selected_branch/branches 保持其原类型字段。
- 宿主 git::update 将仓库消息映射至既有 `message_git::repository::update`；其余消息进入 `message_git::controller::update`，由 diff/modal/refresh 和 update_stage 分派。没有测试专用消息枚举或平行算法。
- 暂存/提交返回携带真实 SelectedCommitRequest 的具名 Commit 效果；宿主使用原网关/租户上下文执行，真实 desktop-git 与网关写入逻辑保持原样。
- 刷新返回 LoadWorktrees/LoadChangedFiles/LoadDiffMetas/LoadDiff 效果；宿主保留已有 spawn_blocking_opt 与网关执行策略。缓存、过期结果校验、加载守卫及提交清理均在领域。
- git_panel::view/embedded_custom_text_diff_view/两种可选模态框投影只读 ViewContext 到领域 page，页面拥有完整子视图组合、帮助层、历史diff消息投影与主题选择。ViewContext只含 State引用及实际主题、locale、路径和局部显示输入；没有App副本、Deref全局借用、反向path/include或循环依赖。
- 帮助沿用 desktop-chat-view 的真实 selectable_titled_text、SelectionMessage 与 desktop-ui-controls；原复制状态、文本上下文菜单和字形渲染路径保持。生产依赖源码遍历419个可达crate，无返回desktop-git-ui边。

## 宿主逐函数保留理由

| 文件/函数 | 保留原因 |
|---|---|
| components/git_panel::input、map、view、embedded_custom_text_diff_view、diff_comment_modal、discard_file_modal | 读取实际工作区/主题/locale、复制反馈、会话路径；映射Git、打开终端、摘要切换、聊天插入、复制与URL跨域消息。所有页面实现由领域返回 |
| components/git_panel::git_repo_path_for_app、changed_files、changed_files_in_repo、get_diff_file_metas_for_repo_path、load_diff_content_for_repo_path | 真实项目/活动会话投影或既有desktop-git函数薄转发；wasm保持空结果分支 |
| components/git_panel::open_terminal | 原生终端所有权及平台错误 |
| message/git::update | 选择仓库映射入口，其余全部走领域路由 |
| message/git/repository::update | 既有仓库局部消息双向映射、网关和租户获取、预览快照/Toast/动画/刷新跨域效果装配 |
| message/git/effects::update、apply | 获取局部上下文，执行预览安全提示、复制/聊天/窗口互斥、首选项写入、后台任务、文件树展开与Toast；不持有领域缓存/选择/表单转换 |
| message/git/stage_commit::apply | 执行真实网关请求、通知/Toast、分支刷新和动画效果 |
| message/git/shared::schedule_commit_button_animation_tick | 文件管理器所属动画消息时钟 |
| message/git/shared::dismiss_preview_transient_ui | 仅清理预览所属弹出层和LSP；Git菜单由领域先清理，避免效果装配清掉新打开菜单 |
| message/git/shared::git_context_path_for_app | 从真实会话获取目录后交给既有领域路径优先级函数 |
| message/git/shared::execute_selected_commit_via_gateway、execute_stage_all_via_gateway、load_git_worktree_options | 真实网关与租户资源装配 |
| message/git/shared::refresh_git_panel_data_after_repo_mutation_task | 对领域after_mutation的Message映射 |
| message/git/shared::persist_file_tree_expanded | 文件树所属首选项持久化 |
| message/git/diff、modal、refresh::update | 仅cfg(test)注册的6行跨域契约调用适配器；没有生产分派实现 |

## 领域字段组

- 仓库与工作树：git_repository、branches、selected_branch、git_worktree_*、selected_git_worktree_directory。
- 提交与暂存：git_commit_*、staged_*、提交帮助显示；开始/完成互斥、请求与清理由领域管理。
- 差异：merge_view、diff_theme、expanded_files/_set、expanded_hunks、context_expansions、git_diff_*（内容/元数据缓存、选择/拖动/菜单/评论、滚动与全屏局部状态）。
- 弹窗与筛选：git_copy_*、git_custom_*、chat_text_diff、file_to_discard、show_git_*、git_filter_*、git_focused_file、git_hovered_file_header、git_panel_header_hovered。
- App只保留领域State字段；项目、实际会话、预览、文件树、窗口和通知各属原领域。

## 统计（pathlib splitlines，含注释空行）

基线分配来源 5488 行 → 当前 1138 行。新增宿主 effects.rs 197 行也计入，真实净减 **4153** 行。
生产候选来源 1971 行 → 449 行；加上新增宿主效果文件后，候选净减 **1325** 行。它仍是baseline文件名分类，不能当作精确业务代码数量或编译提速数据。
共享App状态和初始化另移除72个字段及对应构造；直接调用者主要只增加 `.git` 访问层级，未把其行数减少混入以上来源口径。

| 分配来源 | 基线行数 | 当前行数 | 职责审计 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/git_panel/diff_view/comment_editor.rs` | 5 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/context_menu.rs` | 6 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/file_view/mod.rs` | 36 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/header.rs` | 15 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/header_tests.rs` | 61 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/history.rs` | 41 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/history_tests.rs` | 32 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/markers.rs` | 1 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/mod.rs` | 130 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/mod_tests.rs` | 108 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/selection.rs` | 17 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/styles.rs` | 44 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/tests.rs` | 8 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/text_diff.rs` | 28 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/filter_options.rs` | 25 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/header.rs` | 20 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/header_tests.rs` | 23 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/left_panel.rs` | 30 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs` | 564 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/mod.rs` | 274 | 104 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs` | 178 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/ops.rs` | 21 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs` | 157 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/repository.rs` | 11 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/repository_ops.rs` | 1 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/repository_tests.rs` | 199 | 193 | 仅保留实际预览、窗口布局、文件树与输入反馈跨域契约；纯领域断言已迁出 |
| `crates/vw-desktop/src/app/components/git_panel/tests.rs` | 111 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/ui/disabled_buttons.rs` | 1 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/ui/glyph_buttons.rs` | 1 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/ui/icon_buttons.rs` | 1 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/ui.rs` | 13 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/utils.rs` | 2 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/components/git_panel/view_messages.rs` | 29 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/message/git/diff.rs` | 243 | 6 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs` | 639 | 139 | 仅保留实际预览、窗口布局、文件树与输入反馈跨域契约；纯领域断言已迁出 |
| `crates/vw-desktop/src/app/message/git/mod.rs` | 176 | 66 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/modal.rs` | 131 | 6 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs` | 308 | 82 | 仅保留实际预览、窗口布局、文件树与输入反馈跨域契约；纯领域断言已迁出 |
| `crates/vw-desktop/src/app/message/git/refresh.rs` | 131 | 6 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs` | 580 | 165 | 仅保留实际预览、窗口布局、文件树与输入反馈跨域契约；纯领域断言已迁出 |
| `crates/vw-desktop/src/app/message/git/repository.rs` | 56 | 100 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/repository_tests.rs` | 116 | 110 | 仅保留实际预览、窗口布局、文件树与输入反馈跨域契约；纯领域断言已迁出 |
| `crates/vw-desktop/src/app/message/git/shared.rs` | 204 | 100 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs` | 178 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/message/git/stage_commit.rs` | 278 | 61 | 仅宿主输入投影、消息映射或网关/系统与其它领域效果装配；无独立领域状态 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs` | 181 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |
| `crates/vw-desktop/src/app/message/git/tests.rs` | 74 | 0 | 旧入口已移除；页面/控制器/测试归领域（或无调用包装直接复用既有领域函数） |

## 整组断言映射

135个基线测试函数均找到真实注册归属；静态逐函数计数未发现断言数减少。这是源码核对数量，不是执行通过数。
页面历史/工作区明暗绘制场景留在领域，原预览截图场景单独保留宿主 repository_snapshot_preview_renders_chinese_in_both_themes；真实draw/screenshot与中文文本保留。
控制器纯领域测试使用真实State/Context和领域路由，外部效果通过实际返回类型观察。原先库内diff/modal/refresh测试的小型消息枚举和模拟分派已改为真实路由。
与前轮库测试核对后，没有同名行为场景重复迁入：已有底层选择/颜色/布局/规范化测试继续覆盖其底层入口；新迁移的是整组实际消息和页面装配场景。没有以相同文件名推定等价，也没有删除旧断言去重。

| 原测试 | 新归属/必要宿主契约 | assert宏数（静态） |
|---|---|---:|
| `crates/vw-desktop/src/app/components/git_panel/diff_view/header_tests.rs::build_diff_header_covers_stats_close_and_fullscreen_combinations` | `crates/libs/desktop-git-ui/src/page/diff_view/header_tests.rs::build_diff_header_covers_stats_close_and_fullscreen_combinations` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/header_tests.rs::build_diff_header_without_close_accepts_complete_fullscreen_control` | `crates/libs/desktop-git-ui/src/page/diff_view/header_tests.rs::build_diff_header_without_close_accepts_complete_fullscreen_control` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/header_tests.rs::wrap_diff_header_covers_neutral_added_deleted_and_mixed_styles` | `crates/libs/desktop-git-ui/src/page/diff_view/header_tests.rs::wrap_diff_header_covers_neutral_added_deleted_and_mixed_styles` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/history_tests.rs::historical_diff_blocks_worktree_actions_and_allows_browsing` | `crates/libs/desktop-git-ui/src/page/diff_view/history_tests.rs::historical_diff_blocks_worktree_actions_and_allows_browsing` | 4 → 4 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/mod_tests.rs::diff_render_ctx_snapshots_line_file_and_diff_selection` | `crates/libs/desktop-git-ui/src/page/diff_view/mod_tests.rs::diff_render_ctx_snapshots_line_file_and_diff_selection` | 8 → 8 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/mod_tests.rs::diff_render_ctx_empty_app_has_empty_sets` | `crates/libs/desktop-git-ui/src/page/diff_view/mod_tests.rs::diff_render_ctx_empty_app_has_empty_sets` | 4 → 4 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/mod_tests.rs::diff_render_ctx_matches_line_selection_without_file_stage` | `crates/libs/desktop-git-ui/src/page/diff_view/mod_tests.rs::diff_render_ctx_matches_line_selection_without_file_stage` | 10 → 10 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/mod_tests.rs::diff_render_ctx_deduplicates_inputs_into_sets` | `crates/libs/desktop-git-ui/src/page/diff_view/mod_tests.rs::diff_render_ctx_deduplicates_inputs_into_sets` | 5 → 5 |
| `crates/vw-desktop/src/app/components/git_panel/diff_view/tests.rs::task_713_test_module_is_wired` | `crates/libs/desktop-git-ui/src/page/diff_view/tests.rs::task_713_test_module_is_wired` | 1 → 1 |
| `crates/vw-desktop/src/app/components/git_panel/header_tests.rs::header_toggle_message_helpers_flip_current_state` | `crates/libs/desktop-git-ui/src/page/header_tests.rs::header_toggle_message_helpers_flip_current_state` | 4 → 4 |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs::task_719_test_module_is_wired` | `crates/libs/desktop-git-ui/src/page/left_panel_tests.rs::task_719_test_module_is_wired` | 1 → 1 |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs::selected_changes_detects_each_selection_source` | `crates/libs/desktop-git-ui/src/page/left_panel_tests.rs::selected_changes_detects_each_selection_source` | 5 → 5 |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs::git_commit_form_matches_general_language_select` | `crates/libs/desktop-git-ui/src/page/left_panel_tests.rs::git_commit_form_matches_general_language_select` | 13 → 13 |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs::git_commit_form_preserves_messages_and_validation` | `crates/libs/desktop-git-ui/src/page/left_panel_tests.rs::git_commit_form_preserves_messages_and_validation` | 13 → 13 |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs::help_rollout_26_renders_contextual_help` | `crates/libs/desktop-git-ui/src/page/left_panel_tests.rs::help_rollout_26_renders_contextual_help` | 10 → 10 |
| `crates/vw-desktop/src/app/components/git_panel/left_panel_tests.rs::help_rollout_26_menu_uses_static_explanations` | `crates/libs/desktop-git-ui/src/page/left_panel_tests.rs::help_rollout_26_menu_uses_static_explanations` | 6 → 6 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs::task_720_mod_test_module_is_wired` | `crates/libs/desktop-git-ui/src/page/mod_tests.rs::task_720_mod_test_module_is_wired` | 1 → 1 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs::embedded_custom_diff_and_main_view_cover_modal_and_filter_paths` | `crates/libs/desktop-git-ui/src/page/mod_tests.rs::embedded_custom_diff_and_main_view_cover_modal_and_filter_paths` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs::main_view_covers_help_and_copy_modal_layers` | `crates/libs/desktop-git-ui/src/page/mod_tests.rs::main_view_covers_help_and_copy_modal_layers` | 2 → 2 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs::main_view_filters_included_files_from_all_selection_sources` | `crates/libs/desktop-git-ui/src/page/mod_tests.rs::main_view_filters_included_files_from_all_selection_sources` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs::main_view_covers_empty_and_virtualized_lists` | `crates/libs/desktop-git-ui/src/page/mod_tests.rs::main_view_covers_empty_and_virtualized_lists` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/mod_tests.rs::repository_wrappers_preserve_empty_results_for_missing_repository` | `crates/libs/desktop-git-ui/src/page/mod_tests.rs::repository_wrappers_preserve_empty_results_for_missing_repository` | 5 → 5 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::repo_path_prefers_selected_worktree_and_falls_back_to_project_path` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::repo_path_prefers_selected_worktree_and_falls_back_to_project_path` | 3 → 3 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::repo_path_uses_matching_active_session_and_selected_override_is_authoritative` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::repo_path_uses_matching_active_session_and_selected_override_is_authoritative` | 2 → 2 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::diff_metas_report_sizes_line_counts_and_app_path_content` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::diff_metas_report_sizes_line_counts_and_app_path_content` | 12 → 12 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::repo_path_ignores_unusable_active_session_before_project_fallback` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::repo_path_ignores_unusable_active_session_before_project_fallback` | 3 → 3 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::stage_path` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::stage_path` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::session_info` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::session_info` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/ops_tests.rs::repo_path_rejects_non_git_project_and_accepts_new_repository` | `crates/libs/desktop-git-ui/src/page/ops_tests.rs::repo_path_rejects_non_git_project_and_accepts_new_repository` | 2 → 2 |
| `crates/vw-desktop/src/app/components/git_panel/repository_tests.rs::repository_history_renders_chinese_in_both_themes` | `crates/libs/desktop-git-ui/src/page/repository_tests.rs::repository_history_renders_chinese_in_both_themes`<br>`crates/vw-desktop/src/app/components/git_panel/repository_tests.rs::repository_snapshot_preview_renders_chinese_in_both_themes` | 5 → 8 |
| `crates/vw-desktop/src/app/components/git_panel/tests.rs::changed_files_without_project_path_is_empty_without_repository_access` | `crates/libs/desktop-git-ui/src/page/tests.rs::changed_files_without_project_path_is_empty_without_repository_access` | 1 → 1 |
| `crates/vw-desktop/src/app/components/git_panel/tests.rs::optional_git_modals_require_corresponding_state` | `crates/libs/desktop-git-ui/src/page/tests.rs::optional_git_modals_require_corresponding_state` | 3 → 3 |
| `crates/vw-desktop/src/app/components/git_panel/tests.rs::main_view_constructs_selection_help_and_copy_modal_variants` | `crates/libs/desktop-git-ui/src/page/tests.rs::main_view_constructs_selection_help_and_copy_modal_variants` | 0 → 0 |
| `crates/vw-desktop/src/app/components/git_panel/tests.rs::embedded_diff_constructs_for_both_application_theme_brightnesses` | `crates/libs/desktop-git-ui/src/page/tests.rs::embedded_diff_constructs_for_both_application_theme_brightnesses` | 0 → 0 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::test_module_is_wired` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::test_module_is_wired` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::toggle_diff_highlight_updates_flag` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::toggle_diff_highlight_updates_flag` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::context_menu_selects_line_when_nothing_is_selected` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::context_menu_selects_line_when_nothing_is_selected` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::context_menu_preserves_existing_selection_when_opened_elsewhere` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::context_menu_preserves_existing_selection_when_opened_elsewhere` | 7 → 7 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::file_menu_revert_and_cancel_discard_update_file_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::file_menu_revert_and_cancel_discard_update_file_state` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::close_file_menu_and_confirm_discard_update_only_file_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::close_file_menu_and_confirm_discard_update_only_file_state` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::preview_rejects_unsupported_file_without_external_io` | `crates/vw-desktop/src/app/message/git/diff_tests.rs::preview_rejects_unsupported_file_without_external_io` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::copy_diff_file_prefers_deleted_content_and_falls_back_to_path` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::copy_diff_file_prefers_deleted_content_and_falls_back_to_path` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::preview_and_copy_file_join_project_path_when_available` | `crates/vw-desktop/src/app/message/git/diff_tests.rs::preview_and_copy_file_join_project_path_when_available` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::open_and_cancel_comment_draft_use_context_range` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::open_and_cancel_comment_draft_use_context_range` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::open_comment_draft_without_context_or_selection_is_noop` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::open_comment_draft_without_context_or_selection_is_noop` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::empty_context_stage_actions_leave_existing_stage_selection_unchanged` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::empty_context_stage_actions_leave_existing_stage_selection_unchanged` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::comment_submit_without_draft_preserves_selected_range` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::comment_submit_without_draft_preserves_selected_range` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::comment_editor_action_updates_existing_draft_before_submit` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::comment_editor_action_updates_existing_draft_before_submit` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::context_stage_selection_adds_and_clears_both_diff_sides` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::context_stage_selection_adds_and_clears_both_diff_sides` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::comment_submit_trims_comment_and_formats_one_based_range` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::comment_submit_trims_comment_and_formats_one_based_range` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::blank_comment_submit_emits_single_line_reference_without_comment_marker` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::blank_comment_submit_emits_single_line_reference_without_comment_marker` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::copy_selection_sorts_by_file_side_and_line_before_emitting` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::copy_selection_sorts_by_file_side_and_line_before_emitting` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::insert_selection_comment_groups_files_and_diff_sides` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::insert_selection_comment_groups_files_and_diff_sides` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::hover_exit_only_clears_matching_line` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::hover_exit_only_clears_matching_line` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::empty_copy_and_insert_selection_are_noops_but_close_context_menu` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::empty_copy_and_insert_selection_are_noops_but_close_context_menu` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::insert_selection_to_chat_sorts_lines_before_emitting` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::insert_selection_to_chat_sorts_lines_before_emitting` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::empty_selection_comment_is_noop_and_closes_context_menu` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::empty_selection_comment_is_noop_and_closes_context_menu` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::repo_mutation_messages_without_project_path_clear_transient_file_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::repo_mutation_messages_without_project_path_clear_transient_file_state` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::discard_selection_without_repo_clears_context_menu_only` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::discard_selection_without_repo_clears_context_menu_only` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::discard_selection_with_repo_context_clears_selection_even_when_lines_are_not_changed` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::discard_selection_with_repo_context_clears_selection_even_when_lines_are_not_changed` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::fullscreen_toggles_reset_competing_chat_fullscreen_state` | `crates/vw-desktop/src/app/message/git/diff_tests.rs::fullscreen_toggles_reset_competing_chat_fullscreen_state` | 6 → 6 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::fullscreen_toggle_exits_without_reopening_diff` | `crates/vw-desktop/src/app/message/git/diff_tests.rs::fullscreen_toggle_exits_without_reopening_diff` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::half_fullscreen_enters_and_exits_while_disabling_fullscreen` | `crates/vw-desktop/src/app/message/git/diff_tests.rs::half_fullscreen_enters_and_exits_while_disabling_fullscreen` | 8 → 8 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::expand_all_sets_large_symmetric_context_window` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::expand_all_sets_large_symmetric_context_window` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::toggle_expand_file_collapses_existing_file_and_expands_single_file` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::toggle_expand_file_collapses_existing_file_and_expands_single_file` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::focus_file_resets_preview_state_and_opens_diff` | `crates/vw-desktop/src/app/message/git/diff_tests.rs::focus_file_resets_preview_state_and_opens_diff` | 5 → 5 |
| `crates/vw-desktop/src/app/message/git/diff_tests.rs::hunk_file_context_scroll_and_theme_state_are_updated` | `crates/libs/desktop-git-ui/src/host_module_13_tests/diff_tests.rs::hunk_file_context_scroll_and_theme_state_are_updated` | 6 → 6 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::test_module_is_wired` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::test_module_is_wired` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::custom_diff_result_hides_inputs_and_shows_diff_view` | `crates/vw-desktop/src/app/message/git/modal_tests.rs::custom_diff_result_hides_inputs_and_shows_diff_view` | 7 → 7 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::chat_text_diff_open_and_close_update_panel_state` | `crates/vw-desktop/src/app/message/git/modal_tests.rs::chat_text_diff_open_and_close_update_panel_state` | 12 → 12 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_with_short_text_initializes_both_editors_in_colored_mode` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_with_short_text_initializes_both_editors_in_colored_mode` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_with_text_uses_plain_editor_for_large_text` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_with_text_uses_plain_editor_for_large_text` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_color_toggle_falls_back_for_large_text` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_color_toggle_falls_back_for_large_text` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_color_toggle_on_initializes_code_editor_from_plain_text` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_color_toggle_on_initializes_code_editor_from_plain_text` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_color_toggle_off_restores_code_editor_content_to_plain_editor` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_color_toggle_off_restores_code_editor_content_to_plain_editor` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_plain_editor_action_updates_its_content` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_plain_editor_action_updates_its_content` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::diff_copy_mode_without_project_opens_empty_modal_without_repository_access` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::diff_copy_mode_without_project_opens_empty_modal_without_repository_access` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::diff_copy_mode_with_non_repository_project_falls_back_to_empty_patch` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::diff_copy_mode_with_non_repository_project_falls_back_to_empty_patch` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_from_path_reads_utf8_content` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_from_path_reads_utf8_content` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_from_missing_path_falls_back_to_empty_content` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_from_missing_path_falls_back_to_empty_content` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_modal_close_hides_modal` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_modal_close_hides_modal` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::copy_current_emits_content_from_plain_and_colored_editors` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::copy_current_emits_content_from_plain_and_colored_editors` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::insert_current_emits_plain_editor_content_and_hides_modal` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::insert_current_emits_plain_editor_content_and_hides_modal` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::insert_current_emits_colored_editor_content_and_hides_modal` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::insert_current_emits_colored_editor_content_and_hides_modal` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/modal_tests.rs::insert_explicit_text_emits_that_text_and_hides_modal` | `crates/libs/desktop-git-ui/src/host_module_13_tests/modal_tests.rs::insert_explicit_text_emits_that_text_and_hides_modal` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_worktree_options_without_project_clears_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_worktree_options_without_project_clears_state` | 5 → 5 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_worktree_options_with_project_marks_request_loading` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_worktree_options_with_project_marks_request_loading` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_worktree_options_for_non_git_project_clears_state_without_error` | `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_worktree_options_for_non_git_project_clears_state_without_error` | 6 → 6 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_worktree_options_while_same_project_is_loading_preserves_options` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_worktree_options_while_same_project_is_loading_preserves_options` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_ignores_stale_project_path` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::worktree_options_ready_ignores_stale_project_path` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_selects_matching_directory_and_branch` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::worktree_options_ready_selects_matching_directory_and_branch` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_normalizes_directory_separators` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::worktree_options_ready_normalizes_directory_separators` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_falls_back_to_first_option` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::worktree_options_ready_falls_back_to_first_option` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_with_empty_result_clears_selected_directory` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::worktree_options_ready_with_empty_result_clears_selected_directory` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_error_resets_options_and_reports_error` | `crates/vw-desktop/src/app/message/git/refresh_tests.rs::worktree_options_ready_error_resets_options_and_reports_error` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::select_git_worktree_resets_diff_state_and_updates_branch` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::select_git_worktree_resets_diff_state_and_updates_branch` | 14 → 14 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::toggle_git_worktree_menu_uses_requested_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::toggle_git_worktree_menu_uses_requested_state` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_changed_files_without_repository_clears_cached_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_changed_files_without_repository_clears_cached_state` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_changed_files_while_loading_preserves_cached_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_changed_files_while_loading_preserves_cached_state` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_changed_files_with_project_marks_request_loading` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_changed_files_with_project_marks_request_loading` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_diff_file_metas_without_repository_clears_cached_contents` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_diff_file_metas_without_repository_clears_cached_contents` | 5 → 5 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_diff_file_metas_while_same_repo_is_loading_preserves_contents` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_diff_file_metas_while_same_repo_is_loading_preserves_contents` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::refresh_diff_file_metas_with_project_marks_request_loading` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::refresh_diff_file_metas_with_project_marks_request_loading` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_ignores_stale_repo_path` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::changed_files_ready_ignores_stale_repo_path` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_updates_files_and_clears_selection` | `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_updates_files_and_clears_selection` | 7 → 7 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_expands_change_list_dirs_when_showing_changes` | `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_expands_change_list_dirs_when_showing_changes` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_without_manual_refresh_does_not_create_toast` | `crates/vw-desktop/src/app/message/git/refresh_tests.rs::changed_files_ready_without_manual_refresh_does_not_create_toast` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::diff_content_ready_for_stale_repo_preserves_current_loading_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::diff_content_ready_for_stale_repo_preserves_current_loading_state` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::load_diff_content_without_repository_does_not_mark_file_loading` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::load_diff_content_without_repository_does_not_mark_file_loading` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/refresh_tests.rs::diff_content_ready_ignores_unknown_file_but_clears_loading` | `crates/libs/desktop-git-ui/src/host_module_13_tests/refresh_tests.rs::diff_content_ready_ignores_unknown_file_but_clears_loading` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/repository_tests.rs::history_preview_uses_committed_snapshot_including_deleted_files` | `crates/vw-desktop/src/app/message/git/repository_tests.rs::history_preview_uses_committed_snapshot_including_deleted_files` | 10 → 10 |
| `crates/vw-desktop/src/app/message/git/repository_tests.rs::non_git_project_skips_repository_requests_and_clears_history_error` | `crates/vw-desktop/src/app/message/git/repository_tests.rs::non_git_project_skips_repository_requests_and_clears_history_error` | 14 → 14 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::git_context_path_prefers_trimmed_worktree_and_falls_back_to_project` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::git_context_path_prefers_trimmed_worktree_and_falls_back_to_project` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::git_context_path_returns_none_without_any_context` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::git_context_path_returns_none_without_any_context` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::git_context_path_uses_trimmed_active_session_directory` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::git_context_path_uses_trimmed_active_session_directory` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::dismiss_preview_transient_ui_clears_git_file_menu` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::dismiss_preview_transient_ui_clears_git_file_menu` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::refresh_git_panel_data_task_emits_all_refresh_messages` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::refresh_git_panel_data_task_emits_all_refresh_messages` | 4 → 4 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::take_diff_content_task_rejects_loaded_loading_and_missing_repo_states` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::take_diff_content_task_rejects_loaded_loading_and_missing_repo_states` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::load_missing_diff_contents_without_repo_does_not_mark_files_loading` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::load_missing_diff_contents_without_repo_does_not_mark_files_loading` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::git_repo_path_rejects_explicit_non_repository_worktree` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::git_repo_path_rejects_explicit_non_repository_worktree` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::refresh_after_repo_mutation_clears_stale_loading_guards` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::refresh_after_repo_mutation_clears_stale_loading_guards` | 5 → 5 |
| `crates/vw-desktop/src/app/message/git/shared_tests.rs::selected_lines_from_range_returns_empty_without_diff_content` | `crates/libs/desktop-git-ui/src/host_module_13_tests/shared_tests.rs::selected_lines_from_range_returns_empty_without_diff_content` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_selected_rejects_blank_message_without_starting_commit` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_selected_rejects_blank_message_without_starting_commit` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_all_reuses_commit_validation` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_all_reuses_commit_validation` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::stage_all_requires_project_context_and_reports_finished_states` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::stage_all_requires_project_context_and_reports_finished_states` | 9 → 9 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_selected_while_in_progress_preserves_existing_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_selected_while_in_progress_preserves_existing_state` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_selected_without_project_path_restores_idle_state_and_reports_error` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_selected_without_project_path_restores_idle_state_and_reports_error` | 3 → 3 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_selected_with_valid_context_enters_in_progress_state` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_selected_with_valid_context_enters_in_progress_state` | 2 → 2 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_selected_finished_success_resets_form_and_reports_success` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_selected_finished_success_resets_form_and_reports_success` | 10 → 10 |
| `crates/vw-desktop/src/app/message/git/stage_commit_tests.rs::commit_selected_finished_error_keeps_form_and_reports_error` | `crates/libs/desktop-git-ui/src/host_module_13_tests.rs::commit_selected_finished_error_keeps_form_and_reports_error` | 5 → 5 |
| `crates/vw-desktop/src/app/message/git/tests.rs::test_module_is_wired` | `crates/libs/desktop-git-ui/src/host_module_13_tests/tests.rs::test_module_is_wired` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/tests.rs::update_routes_worktree_menu_message_to_refresh_handler` | `crates/libs/desktop-git-ui/src/host_module_13_tests/tests.rs::update_routes_worktree_menu_message_to_refresh_handler` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/tests.rs::update_routes_line_selection_message_to_diff_handler` | `crates/libs/desktop-git-ui/src/host_module_13_tests/tests.rs::update_routes_line_selection_message_to_diff_handler` | 5 → 5 |
| `crates/vw-desktop/src/app/message/git/tests.rs::update_routes_custom_diff_title_message_to_modal_handler` | `crates/libs/desktop-git-ui/src/host_module_13_tests/tests.rs::update_routes_custom_diff_title_message_to_modal_handler` | 1 → 1 |
| `crates/vw-desktop/src/app/message/git/tests.rs::update_routes_filter_query_message_to_stage_commit_handler` | `crates/libs/desktop-git-ui/src/host_module_13_tests/tests.rs::update_routes_filter_query_message_to_stage_commit_handler` | 1 → 1 |

## 宿主测试保留边界

- diff_tests：不支持文件的预览错误反馈、预览/复制实际宿主消息、窗口全屏与聊天布局互斥、焦点重置实际预览状态。
- modal_tests：自定义/聊天差异打开时的预览清理、show_diff与文件管理器面板联动。
- refresh_tests：输入错误反馈、文件树刷新标志与Toast、真实变化目录展开持久化。
- message/git/repository_tests：历史快照落入真实只读预览、保存拦截、非Git项目跨仓库/差异入口反馈。
- components/git_panel/repository_tests：快照预览面板的实际中文绘制。Git页面自身的明暗/历史/工作区draw场景已经迁入page。

## 静态验证和明确限制

- 对修改的领域/装配文件执行独立rustfmt（skip_children=true），未递归格式化未参与文件；任务路径git diff --check无输出。
- 源码核对所有mod/cfg、135个旧用例的注册映射、72个字段的初始化和直接调用者、wasm元数据空结果分支、依赖方向及完整提交差异。无App反向引用。
- 按用户要求，没有运行任何Cargo命令、单元测试、实机绘制或最终回归；静态交付不表示编译、执行、截图或字体整形通过。
- 已有 `refresh_git_panel_data_task_emits_all_refresh_messages` 的断言期望3条，而原生产已发出4条（含RefreshRepository）；本轮保留原断言与原行为，未来授权测试时需要维护者按真实契约判断。这不是本轮引入的行为修复。
- 未来授权后的聚焦命令（本轮均未执行）：
  - `python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib host_module_13_tests -- --test-threads=1`
  - `python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib page:: -- --test-threads=1`
  - `python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib message_git:: -- --test-threads=1`
  - `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`
- 原生UI、系统权限与实际仓库写入验收须以后另行授权，不自动启动。
