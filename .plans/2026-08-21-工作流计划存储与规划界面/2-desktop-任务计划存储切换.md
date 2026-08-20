# 任务 2: desktop 任务计划存储切换

批次：【批次 2】 依赖批次 1

状态：未开始

目的：让桌面任务执行器的计划拆分产物和执行上下文统一使用用户级 `storage/plan/task-<id>/`，不再写入项目目录。

来源任务：用户提出“项目中生成很奇怪”。

预计会话范围：仅调整 `vw-desktop` 任务执行器 runner 的计划路径解析、错误传播和相邻测试；不改变任务拆分、执行或清理语义。

## 文件

- 修改：`crates/vw-desktop/src/app/task/executor/runner.rs`
- 修改：`crates/vw-desktop/src/app/task/executor/runner_tests.rs`

## 上下文

- runner 目前同时存在 `.plans/task-<id>` 的拆分目录，以及 `<project>/<HOME_CONFIG_DIR_NAME>/tasks/plan/<id>` 的执行上下文目录。
- `programs::user_home_dir()` 已按 `HOME`、`USERPROFILE` 顺序解析用户 home；批次 1 的共享 helper 是唯一目标根。
- `write_task_plan_files`、`read_plan_context`、`read_task_plan_prompt_md` 和 `execute_task_plan_blocking` 必须解析到同一任务目录。

## 测试计划

- 行为：任务拆分、prompt.md 读取、plan.md/子任务上下文写读都落到临时 home 的应用存储目录；项目内历史缓存即使存在也被忽略，不迁移、不读取、不回退。
- 失败验证测试：把 `task_plan_paths_use_project_plan_directory` 改为 `task_plan_paths_use_user_storage_directory`，新增 `task_plan_reads_ignore_legacy_project_cache`，并更新 prompt 读取与缺失 prompt 错误测试的目录断言。
- 失败验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期失败原因：测试先按 home 参数调用尚未改造的路径函数，产生签名/类型编译错误；现有实现仍依赖 project path。
- 通过验证命令：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 模拟策略：用临时 home 和项目目录验证真实文件系统写读；不修改进程级 HOME，避免并发测试污染。

## 步骤

1. 先改写路径测试，覆盖拆分目录、prompt.md、运行时 plan.md 和编号子任务文件的统一根。
2. 让 runner 在任务入口解析一次用户 home，并把明确路径传给所有计划读写 helper。
3. 把拆分提示中的“仓库相对路径”改为共享绝对计划目录，确保模型与运行时读取位置一致；不向模型提及旧路径。
4. home 不可用或新目录缺失时向上返回明确错误；禁止扫描、读取、迁移或回退到项目缓存。
5. 保留任务 ID 清洗、同任务旧 Markdown 清理和项目 root 下的代码执行行为。
6. 运行 clippy 并审查所有生产调用者已改用新路径契约。

## 验证

- 运行：`cargo clippy -p vw-desktop --all-targets --all-features -- -D warnings`
- 预期：退出码 0；runner 的全部调用者与测试编译通过，错误分支不产生项目级缓存。
- 所需证据：失败验证编译错误、通过验证退出码、临时 home 路径断言、冲突历史缓存被忽略且未复制的断言、调用者搜索结果与已审查差异。

## 完成

- `runner.rs` 中两套计划路径都统一到批次 1 的共享用户级根，项目 path 只用于代码执行。
- 旧缓存视为废弃数据，不兼容、不读取、不迁移；本任务不宽泛删除可能混有非缓存计划的项目目录。未实际运行测试时状态使用 `已完成，待回归`。
