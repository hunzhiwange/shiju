# 任务6：完整静态交付

本轮遵守禁测要求：未运行任何Cargo命令、测试、绘制、实机或回归。下列均为源码审查、路径解析、独立rustfmt与diff证据，不表示编译或行为通过。

## 生产实现和真实调用链

- `chat_panel::view → panel::panel_phase / conversation_window / conversation_body / status_reading_column / started_view / placeholder_view`。局部View拥有页面阶段、真实阅读列、窗口切片、缓存回退、metadata、修订键、重内容可见范围、MeasuredChatMessage及scrollable树。宿主只装配会话借用输入、消息snapshot、主题皮肤/轨迹/todo内容端口与事件映射。
- `chat_panel_context_menu_overlay → panel::context_menu_overlay`：唯一PointBelowOverlay在领域库构造；宿主只映射复制/追加/搜索/关闭事件。
- `chat_todo_overlay → panel::todo_is_visible / todo_overlay`：位置、会话匹配与空项策略及布局归库；todo内容由已有输入领域组件装配。
- `input::apply_chat_message_measurements → scroll_update::apply_targeted_message_measurements`：有限数、代际、身份、修订校验及索引投影归库；宿主只补齐已有状态长度并同步兼容字段。
- `tools::render_shared_tool_view → desktop_chat_tools::tools::tool_renderer::view`：真实16类视图分派及TodoCompact→TodoRead→Text回退顺序归库；宿主只传递CardInput/ResultInput及真实Git diff、存储todo和消息映射。已删除失去调用者的单类宿主视图包装，避免平行分派。
- `empty_session_placeholder → EmptySessionInput::from_context`：问候时段、工作模式标题、git标签归库；宿主保留账户profile、系统时间、工作区/修改时间读取。
- `selectable_text::visible_editor_text_for_message → visible_editor_source`：角色选择归库；宿主只接已有附件领域的标记解析并保持清理顺序。
- 测量、缓存、height index、virtualizer、scroll update本已是领域类型；App字段、初始化、消息调用者继续引用原领域类型，本次不复制状态。轨迹原有库入口保留，删除不再使用的子模块包装及重复详情映射。

## 测试归属与断言映射
基线识别到 92 个测试/手动probe用例，每个名称均找到真实归属；详见 `task-6-assertion-map.json`。没有删除旧断言或用零过滤代替执行。
- `desktop-chat-view::host_module_6_tests` 独立注册状态栏、标题栏、消息视图、测量、文本选择、布局、窗口、空态、标签、工具选择器、详情弹窗与完整性能/probe用例。
- `desktop-git-ui::host_module_6_tests` 独立注册advanced/apply_patch/shared tool三组整合用例：使用真实工具库视图及真实Git页面embedded diff注入；保持中文、明暗、布局、事件、draw/screenshot与截图落盘源码。没有以Space假冒diff。
- 选择器点击后调用desktop-session::preferences::select_agent与selection::update；详情打开调用真实tool_detail_from_raw/handle_open_tool_detail。外部系统时钟与账户profile用确定输入替代，未mock被测算法。
- 共享任务5 RenderFixture 的借用snapshot/chat_ui；没有复制App，也未引入Deref或反向include生产源码。新增HeaderFixture/SelectorFixture/LabelFixture只持有本域所需的真实局部值。
- 对前轮desktop_split相关模块做了名称/断言主题核对：同名用例无跨轮重复；前轮基础style/分类/安全函数断言与本轮真实视图、selection、layout/draw组合断言保留各自作用。本次不删除前轮断言。

### 宿主仍保留的真实跨域契约（不是待迁领域测试）
- `header_tests::return_to_parent_session_pressed_emits_open_parent_when_parent_id_present`：真实ProjectMessage打开子会话，再检查HeaderAction经宿主映射后的父会话路径/id；领域HeaderInput测试保留局部导航断言。
- `mod_tests::conversation_shell_borrows_runtime_once`：检查App装配层只记录一次runtime借用且无clone；领域窗口本身不拥有App runtime。
- `mod_tests::conversation_shell_renders_visual_regression_scenarios_without_panic`：真实theme皮肤资产+聊天View+输入区View的组合截图，含7种场景×明暗/皮肤。此跨域组合保留唯一App场景装配夹具，未在领域复制；单条消息、窗口、状态、工具/详情的局部draw已分别归库。
- `mod_tests::chat_panel_view_builds_empty_and_loading_session_states / chat_panel_view_builds_started_message_window`：宿主真实账户/工作区/会话准备状态到领域View的装配契约。
- `mod_tests::chat_panel_switch_preserves_conversation_state`：ChatMessage路由、轨迹与会话两个领域切换后的App回写兼容字段契约。
- `mod_tests::chat_panel_virtualization_has_single_state_owner / chat_panel_context_menu_has_a_single_overlay_owner`：检查宿主跨文件调用链不新增第二个状态/overlay所有者。
- 已移除旧空测试挂载、标题栏测试辅助包装、旧性能probe宿主shim；本任务不修改任务5另属的message_view/host_contract_tests。

## 依赖和公共接线

生产依赖方向保持：desktop-git-ui → desktop-chat-view → desktop-chat-tools；desktop-chat-view → desktop-session。工具库不依赖任何宿主。
新增依赖仅为测试已有类型/真实渲染器：chat-view dev依赖settings-agents/settings-skills/config-agent/config-agent-profile与image、多线程tokio；git-ui dev依赖chat-tools。没有新增crate或根清单配置。Cargo.lock仅同步已有包的依赖表。
公共调用者只改自身测量分支及删除失去调用者的test-only流probe shim；没有改写其它任务正在修改的快捷消息分支。

## 文件职责与真实净减量
口径：Python pathlib read_text().splitlines()，含注释空行。基线 4449 行，当前 1341 行，宿主净减少 3108 行；生产候选 1274→920，减少 354 行。没有新建宿主替代文件。
此基线到当前包含前置阶段对本区域的变化，不把全量净减归因单次提交；生产候选按baseline布尔字段独立列出，不把测试减量算成生产减量。

| 来源 | 基线 | 当前 | 残留职责/函数 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/components/chat_panel/chunk_loader.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/chunk_loader_tests.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/empty.rs` | 30 | 29 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：session_loading_placeholder, empty_session_placeholder |
| `crates/vw-desktop/src/app/components/chat_panel/empty_tests.rs` | 33 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/global_status.rs` | 12 | 18 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：chat_global_status_view |
| `crates/vw-desktop/src/app/components/chat_panel/global_status_tests.rs` | 197 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/header.rs` | 81 | 98 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：show_half_fullscreen_control, input, map, chat_header_view, return_to_parent_session_pressed |
| `crates/vw-desktop/src/app/components/chat_panel/header_tests.rs` | 355 | 55 | 上文列明的真实跨域契约；函数：test_app, session, return_to_parent_session_pressed_emits_open_parent_when_parent_id_present |
| `crates/vw-desktop/src/app/components/chat_panel/height_index.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/message_measure.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/message_measure_tests.rs` | 68 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/message_view_tests.rs` | 322 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/mod.rs` | 393 | 268 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：chat_global_status_reading_column, chat_panel_virtual_window, session_loading_panel, view, tool_dialog_overlay, empty_fullscreen_controls, panel_message, jump_to_bottom_button |
| `crates/vw-desktop/src/app/components/chat_panel/mod_tests.rs` | 525 | 366 | 上文列明的真实跨域契约；函数：message, conversation_shell_borrows_runtime_once, conversation_shell_renders_visual_regression_scenarios_without_panic, name, visual_app, chat_panel_virtualization_has_single_state_owner, keep_element, chat_panel_view_builds_empty_and_loading_session_states, chat_panel_view_builds_started_message_window, chat_panel_switch_preserves_conversation_state, chat_panel_context_menu_has_a_single_overlay_owner |
| `crates/vw-desktop/src/app/components/chat_panel/performance.rs` | 1 | 1 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/performance_tests.rs` | 315 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/selectable_text.rs` | 11 | 22 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：visible_editor_text_for_message |
| `crates/vw-desktop/src/app/components/chat_panel/selectable_text_tests.rs` | 84 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_names.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_selector.rs` | 33 | 33 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：map_message, session_tool_selector_popover |
| `crates/vw-desktop/src/app/components/chat_panel/tool_selector_tests.rs` | 381 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tool_text_support.rs` | 32 | 32 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：text_state, map_message, selected_chat_text_for_message, selected_chat_text_for_tool, selected_chat_text_for_target |
| `crates/vw-desktop/src/app/components/chat_panel/tools/advanced_view.rs` | 22 | 13 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：map_message |
| `crates/vw-desktop/src/app/components/chat_panel/tools/advanced_view_tests.rs` | 354 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_preview.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_view.rs` | 5 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/apply_patch_view_tests.rs` | 104 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/bash_view.rs` | 4 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/card_adapter.rs` | 41 | 41 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：input, map, map_card |
| `crates/vw-desktop/src/app/components/chat_panel/tools/changes.rs` | 2 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/command_summary_view.rs` | 5 | 5 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/config_view.rs` | 4 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/diff_utils.rs` | 2 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/explore_summary_view.rs` | 13 | 13 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/file_link.rs` | 8 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/files_view.rs` | 10 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/git_diff_view.rs` | 12 | 16 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/lsp_view.rs` | 7 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/mcp_view.rs` | 7 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/question_view.rs` | 7 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/read_view.rs` | 7 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/skill_view.rs` | 4 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/text_view.rs` | 4 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/text_view_tests.rs` | 59 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/todo_parse.rs` | 2 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/todo_views.rs` | 8 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_detail_dialog.rs` | 26 | 26 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：input, map, tool_detail_dialog_view |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_meta.rs` | 19 | 1 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_parse.rs` | 32 | 32 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：resolve_output_path |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_permission.rs` | 1 | 1 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_renderer.rs` | 82 | 33 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/tool_renderer_tests.rs` | 305 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/types.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/web_view.rs` | 4 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/tools/workflow_view.rs` | 5 | 5 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/tools.rs` | 155 | 67 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/details.rs` | 16 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/details_content.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/json_tree.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/ledger.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/mod.rs` | 74 | 86 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：view |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/model.rs` | 2 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/projection.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/styles.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/trajectory/timeline.rs` | 1 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/app_state.rs` | 6 | 6 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：get_session_title, current_project_path_label, current_branch_label, is_recent_copy |
| `crates/vw-desktop/src/app/components/chat_panel/utils/app_state_tests.rs` | 72 | 0 | 已删除：领域实现/测试归库，或失去调用者的包装与挂载移除 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/mod.rs` | 43 | 39 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/path.rs` | 4 | 4 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：resolve_path, relative_to_project_root |
| `crates/vw-desktop/src/app/components/chat_panel/utils/text.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/theme.rs` | 1 | 1 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/utils/time.rs` | 4 | 4 | 只读会话/资源输入投影、外域组件注入与宿主消息/效果映射；函数：project_last_modified_ms, relative_modified_label |
| `crates/vw-desktop/src/app/components/chat_panel/utils/ui.rs` | 12 | 12 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |
| `crates/vw-desktop/src/app/components/chat_panel/virtualizer.rs` | 2 | 2 | 领域类型/函数重导出，保持真实调用者导入路径；无宿主领域实现 |

### 保留状态字段组

本任务未新增宿主字段。App中chat_virtualizer、render_cache、height index及测量/滚动兼容字段继续使用已有领域类型：App负责Iced生命周期与跨会话装配，局部更新/校验在ChatUi/scroll_update。会话借用数据不复制、不改变权限或取消/代际隔离。

## 静态证据与未验证风险

- 所有限定路径git diff --check无错误；独立rustfmt仅处理本任务修改文件且skip_children=true；新增mod与include_str字面路径均解析到存在文件；本地依赖路径存在。未使用干净工作区代替差异审查。
- 源码核对生产注册、cfg(test)/wasm分支与真实调用者；92个旧用例均有目标，draw、screenshot与中文字体路径源码仍在。
- 环境Python不含tomllib，故未声称TOML解析通过；以依赖表逐项和本地路径静态核对替代。
- Rust类型/生命周期、native renderer字体整形、图形backend、实际鼠标交互及截图都未运行。旧标签测试中“近期列表空标题时用项目缓存”的期望与原library首项查找/宿主标题回调之间存在静态疑点；保留原期望，未混入行为修复，后续动态验证须核实（本轮未报通过）。

仅以后用户另行授权时可运行：
- `python3 scripts/cargo_slot.py cargo test -p desktop-chat-view --lib host_module_6_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib host_module_6_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::chat_panel::mod_tests -- --test-threads=1`（保留跨域契约）
- `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::components::chat_panel::header::header_tests -- --test-threads=1`
- 宿主目标检查由后续授权验证阶段按实际版本合并，本轮不执行。
