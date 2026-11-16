# 任务11 静态交付

执行与结果状态继续复用既有 `update::execution::ExecutionState/ExecutionEffect`、`update::execution_results::ResultState/ResultEffect`。没有复制算法、没有反向 App 依赖、没有 Cargo 依赖变更。

## 生产接线

- 宿主 `views/task_board::view` → `page::assemble(PageView, PageBuilders)` → responsive `view` → `build_view`。创建/详情/设置/像素办公室槽位的显示判定由领域执行；宿主闭包只投影各跨域视图。延迟构建顺序保留。
- 宿主 `panel::build_task_panel` → `panel::build_with_controls` → `PanelSlots` → 原 `build_task_panel`。编辑日志页与导入模式的选择器装配判定迁出。推理选项仍由聊天/模型配置投影，局部选择器与面板由领域绘制。
- execution/result update 的原真实调用链保持：宿主投影借用 → 领域状态转换 → 具名效果 → 宿主网关、计时器、通知与日志缓存装配。取消、最终日志 drain、待持久化集合、手工合并队列均未改写。
- 批量持久化继续使用既有领域 bulk 更新与元数据补丁；本任务未创建平行持久化策略。
- App 字段和初始化已满足领域借用入口，不新增冗余持有状态或改动共享字段。

## 断言迁移

以下每个迁移用例的旧断言原样保留（仅字段借用/消息包装/真实入口路径调整），未删成功/失败分支断言。启动用例的 auto-review 截止时间断言移到宿主网关启动契约：这是宿主计时器装配字段，ExecutionState 不持有它。

### `crates/libs/desktop-task-board/src/host_module_11_tests/execution_results_tests.rs`

- `test_module_is_wired`：来源同名用例；保留全部断言。
- `execution_success_uses_workflow_transition_to_completed`：来源同名用例；保留全部断言。
- `task_result_persistence_guards_local_status_until_ack`：来源同名用例；保留全部断言。
- `merge_success_uses_workflow_transition_to_completed`：来源同名用例；保留全部断言。
- `merge_failure_preserves_worktree_context`：来源同名用例；保留全部断言。
- `code_review_success_without_merge_metadata_pauses_and_recycles_worktree`：来源同名用例；保留全部断言。
- `code_review_timeout_error_uses_timeout_pause_reason`：来源同名用例；保留全部断言。
- `task_pool_desktop_success_does_not_append_git_actions`：来源同名用例；保留全部断言。

### `crates/libs/desktop-task-board/src/host_module_11_tests/panel_ui_tests.rs`

- `ui_refresh_35_layout_and_interaction`：来源同名用例；保留全部断言。
- `ui_refresh_35_sidebar_shields_the_board`：来源同名用例；保留全部断言。

### `crates/libs/desktop-task-board/src/host_module_11_tests/execution_tests.rs`

- `start_and_stop_execution_toggle_executor_state_and_scan_cursors`：来源同名用例；保留全部断言。
- `execution_and_periodic_ticks_return_without_touching_state_when_disabled`：来源同名用例；保留全部断言。
- `in_flight_status_tick_filters_project_and_status_then_dedupes_log`：来源同名用例；保留全部断言。
- `simulate_step_finishes_missing_running_task_without_status_change`：来源同名用例；保留全部断言。
- `auto_merge_tick_requires_project_even_when_executor_is_running`：来源同名用例；保留全部断言。
- `execute_task_without_project_marks_the_task_failed`：来源同名用例；保留全部断言。
- `manual_execute_deduplicates_agent_scheduler_start_log`：来源同名用例；保留全部断言。
- `agent_scheduler_wake_response_does_not_reserve_desktop_running_slot`：来源同名用例；保留全部断言。
- `wake_agent_scheduler_create_requests_include_pool_and_filter_terminal_tasks`：来源同名用例；保留全部断言。
- `manual_execute_ignores_task_already_waiting_for_worktree`：来源同名用例；保留全部断言。

### `crates/libs/desktop-task-board/src/host_module_11_tests/forms_ui_tests.rs`

- `ui_refresh_34_layout_and_interaction`：来源同名用例；保留全部断言。
- `ui_refresh_34_preserves_values_and_disabled_states`：来源同名用例；保留全部断言。

局部 PanelFixture 只拥有 PanelView/BoardView/ControlView/DragView 实际字段。使用真实 page::assemble、board、control、drag、modal、panel 与选择器；不启动 App。双语/繁中、明暗主题、真实 draw/screenshot、滚动槽位与事件断言保留。推理槽位在无模型能力目录的布局夹具中为空；已有独立 reasoning 测试不变。ToggleImportMode 使用真实 update_local，未重写更新算法。测试隔离进程仍使用原测试名称与配置。

前轮 desktop_split_20_tests 的独立模型/执行器/推理交互用例仍覆盖局部选择器，本次是页面和面板整组布局断言，不复制它们。execution_results 原有 late_success 辅助用例与本次六种结果更新断言入口不同，保持原注册。wake 注册元数据既有 helpers 用例不删除，本次保留历史整组用例以免未经逐项等价核对删除覆盖。

## 宿主保留职责

- helpers.rs：仅六个子模块导出挂载，无算法；message.rs：仅领域消息重导出。
- execution.rs：App 字段投影、全局网关配置、项目 scope 解析、注册后唤醒、跨域日志缓存同步、计时器与原生工作树效果执行。
- execution_results.rs：ResultState 字段投影、异步网关持久化结果消息包装、日志缓存、全局通知与计时器效果。
- execution_tests.rs：保留真实网关 transport/config 临时环境、异步 Task 输出契约以及跨域工作树确认路由；纯状态用例全部迁出。gateway recording fixture 仍有真实调用者，未复制到领域。
- board.rs/drag.rs/control.rs/modals.rs/panel 各入口：仅 App → 局部 View 投影与 Message 包装。模型跳转映射到 Settings/View，剪贴板/外链/Markdown selection 映射全局消息；reasoning 从聊天模型目录借用能力。common.rs/styles.rs 仅重导出。
- board_adapter_tests：真实 App 注入多域 selector 的宿主契约。
- common_ui_refresh_34_tests：仅导入错误在 App.view 全局 toast/notification 的可见性；两组纯页面布局测试已迁出。保留其 renderer/布局/截图辅助函数供这个真实全局契约使用。
- executor_selector_ui_refresh_35_tests：仅五个模型选择器全局 Settings/View/TaskBoard 路由断言；所有页面/面板绘制测试与大型夹具已迁出。
- control/integration_tests：实际 App 消息入口 → 工作树确认 → 异步完成日志 → 控制面板的跨域生命周期。
- modals_tests：系统设置 tab_appearance App 场景与全局 help overlay 的 CopyCode/View 路由；panel/tests 共享 help fixture，保留真实跨域消费者。领域已有独立 modals_tests 覆盖局部标签/布局。
- panel/tests：表单帮助与 App 组合（外链、选择文本及全局消息），聊天 priority/drag-drop；reasoning_selector_tests：任务编辑/批量参数 → 聊天 session runtime → AddTaskFromInputWithOptions 跨域契约。

仅这些效果装配、全局状态投影与跨域断言留宿主；没有新增宿主替代文件。

## 基线统计

pathlib.read_text().splitlines 口径：来源 3039 → 1807，净减 1232；生产候选 454 → 433，净减 21。生产候选只是文件名筛选，本次生产职责迁移为页面可见槽位装配与面板选择器装配，不能将测试减少算成生产减少。

| 来源 | 基线 | 当前 | 净减 |
|---|---:|---:|---:|
| `crates/vw-desktop/src/app/message/task_board/helpers.rs` | 42 | 26 | 16 |
| `crates/vw-desktop/src/app/message/task_board/message.rs` | 2 | 2 | 0 |
| `crates/vw-desktop/src/app/message/task_board/update/execution.rs` | 107 | 107 | 0 |
| `crates/vw-desktop/src/app/message/task_board/update/execution_results.rs` | 62 | 59 | 3 |
| `crates/vw-desktop/src/app/message/task_board/update/execution_results_tests.rs` | 223 | 0 | 223 |
| `crates/vw-desktop/src/app/message/task_board/update/execution_tests.rs` | 525 | 207 | 318 |
| `crates/vw-desktop/src/app/views/task_board/board.rs` | 37 | 37 | 0 |
| `crates/vw-desktop/src/app/views/task_board/board_adapter_tests.rs` | 20 | 20 | 0 |
| `crates/vw-desktop/src/app/views/task_board/common.rs` | 5 | 5 | 0 |
| `crates/vw-desktop/src/app/views/task_board/common_ui_refresh_34_tests.rs` | 515 | 240 | 275 |
| `crates/vw-desktop/src/app/views/task_board/control/integration_tests.rs` | 99 | 99 | 0 |
| `crates/vw-desktop/src/app/views/task_board/control.rs` | 32 | 32 | 0 |
| `crates/vw-desktop/src/app/views/task_board/drag.rs` | 7 | 7 | 0 |
| `crates/vw-desktop/src/app/views/task_board/mod.rs` | 18 | 20 | -2 |
| `crates/vw-desktop/src/app/views/task_board/modals.rs` | 14 | 14 | 0 |
| `crates/vw-desktop/src/app/views/task_board/modals_tests.rs` | 486 | 486 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/executor_selector.rs` | 14 | 14 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/executor_selector_ui_refresh_35_tests.rs` | 419 | 24 | 395 |
| `crates/vw-desktop/src/app/views/task_board/panel/logs_modal.rs` | 6 | 6 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/mod.rs` | 64 | 60 | 4 |
| `crates/vw-desktop/src/app/views/task_board/panel/model_selector.rs` | 29 | 29 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/reasoning_selector.rs` | 14 | 14 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/reasoning_selector_tests.rs` | 81 | 81 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/styles.rs` | 1 | 1 | 0 |
| `crates/vw-desktop/src/app/views/task_board/panel/tests.rs` | 217 | 217 | 0 |

## 静态验证与限制

只运行源码读取、Python pathlib 统计、独立 rustfmt 解析与本任务路径 git diff --check，检查 mod/cfg 注册及调用者。领域 host_module_11_tests 已在 lib.rs 注册；未使用 path/include 反向加载宿主。全工作区 diff --check 曾报告其它任务 vw-agent/session 两文件尾部空行，未越权修改；本任务路径检查无输出。

静态交付完成，未编译、未测试、未执行绘制或实机、未回归。类型/lifetime 与真实字体整形仍待另行授权验证，不声称运行通过。以后授权命令：`python3 scripts/cargo_slot.py cargo test -p desktop-task-board --lib host_module_11_tests -- --test-threads=1`；宿主 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`，本轮均未执行。
