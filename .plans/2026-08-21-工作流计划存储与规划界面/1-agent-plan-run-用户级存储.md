# 任务 1: agent plan-run 用户级存储

批次：【批次 1】 无

状态：已完成，待回归

目的：让网关 plan-run 只在用户级 `<HOME_CONFIG_DIR_NAME>/storage/plan/task-<id>/` 生成计划，同时保持项目目录为代码执行 root。

来源任务：用户提出“工作流执行缓存应生成到 `~/.vibewindowdev/storage/plan/` 或 `~/.vibewindow/storage/plan/`”。

预计会话范围：集中修改共享路径契约和 `vw-agent::task_plan_run` 的路径、提示词与相邻测试；不触及桌面 UI 或流式刷新。

## 文件

- 修改：`crates/vw-config-types/src/paths.rs`
- 修改：`crates/vw-config-types/src/paths_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/prompts.rs`
- 修改：`crates/vw-agent/src/task_plan_run/runner.rs`
- 修改：`crates/vw-agent/src/task_plan_run/prompts_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/runner_tests.rs`
- 修改：`crates/vw-agent/src/task_plan_run/session_bind_tests.rs`

## 上下文

- `plan_dir_abs(root, task_id)` 当前拼接项目 `.plans/task-<id>`；`run_task_plan` 创建该目录并把相对路径写进规划提示。
- `crate::global::paths().home` 是 agent 已有用户 home 来源；`vw_config_types::paths::home_config_dir` 已处理 debug/release 目录名。
- `SessionRunRequest.root` 必须继续使用项目 root，不能改成计划缓存目录。

## 测试计划

- 行为：debug/release 构建均通过 `HOME_CONFIG_DIR_NAME` 得到用户级 `storage/plan/task-<安全 ID>`，规划提示使用该绝对目录；项目 root 下的历史 `.plans` 即使存在也不迁移、不读取、不作为回退。
- 失败验证测试：新增 `task_plan_storage_dir_uses_build_specific_home_directory` 和 `run_plan_ignores_legacy_project_cache`，更新 `build_planning_prompt_requires_fixed_plan_dir_and_tool_writes` 与 `run_plan_emits_full_success_event_sequence` 的路径断言。
- 失败验证命令：`cargo clippy -p vw-config-types -p vw-agent --all-targets --all-features -- -D warnings`
- 预期失败原因：测试先引用尚未存在的共享计划存储路径 helper，或现有提示仍包含 `.plans/task-<id>`，产生编译失败或待回归断言不满足。
- 通过验证命令：`cargo clippy -p vw-config-types -p vw-agent --all-targets --all-features -- -D warnings`
- 模拟策略：runner 测试使用临时 home 与临时项目 root；只模拟现有 `SessionRunner` 外部边界，不修改进程真实 home。

## 步骤

1. 在 `vw-config-types::paths` 为 `<home>/<HOME_CONFIG_DIR_NAME>/storage/plan` 编写测试与最小 helper。
2. 让 `task_plan_run` 路径 helper 接收 home，并把安全任务子目录拼到共享根目录；移除仓库相对 `.plans` 约束。
3. 让规划提示明确要求写入运行时给出的绝对目录，同时继续把项目 root 传给规划和执行会话。
4. 调整 runner 测试 fixture，使计划文件写到临时 home；在项目 root 预置冲突历史缓存，证明运行时忽略它且不会复制到新目录。
5. 对 home/目录创建失败返回包含目标路径的明确错误；不扫描、不读取、不迁移或回退到项目目录。
6. 运行 clippy，并审查没有历史兼容分支、自动迁移、宽泛删除或新增配置开关。

## 验证

- 运行：`cargo clippy -p vw-config-types -p vw-agent --all-targets --all-features -- -D warnings`
- 预期：退出码 0；全部新增/更新测试可编译，生产路径不再把 `request.root` 用作计划目录根。
- 所需证据：失败验证的编译错误或旧路径差异、通过验证退出码、共享 helper 的路径断言、runner 忽略冲突历史缓存且未复制数据的断言、已审查差异。
- 验证结果：失败验证退出码 101，新增测试因 `task_plan_storage_dir` 尚不存在而编译失败；实现后通过验证退出码 0。
- 待回归：仓库禁令要求不执行 `cargo test`，新增行为测试仅由 all-targets clippy 证明可编译，尚未实际运行。

## 完成

- `paths.rs` 提供唯一用户级计划存储根；`prompts.rs` 和 `runner.rs` 使用该根生成、读取并上报计划路径。
- 历史 `.plans` 不进入任何兼容、读取或迁移路径；为避免误删非缓存计划，本任务不递归删除整个目录。因仓库禁止执行测试，只有 clippy 和差异证据时将状态更新为 `已完成，待回归`。
