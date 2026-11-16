# 任务13实施检查点（整体未完成）

## 已落实入口

`message/git/mod.rs -> stage_commit::update -> desktop_git_ui::message_git::stage_commit::update`。
领域更新借用提交表单、过滤及选择字段，持有互斥检查、请求校验、开始/完成转换、成功清理和效果顺序。
返回 StageAll、Commit（携带 SelectedCommitRequest 和动画要求）、InputError、Error、Notify、NotifyRaw、Toast、RefreshAfterMutation、RefreshBranches。
宿主保留字段借用和网关执行、通知、Toast、分支/文件树刷新等跨域效果装配；网关仍使用原真实执行入口，没有替换仓库写入算法。
未改变 desktop-git 或网关依赖；没有领域对宿主的依赖、path/include 反向加载。

## 断言迁移

原 `message/git/stage_commit_tests.rs` 整组8个用例迁至领域 `host_module_13_tests.rs`，原名字和状态断言保留；通知断言改查真实返回的 Notify 效果，任务存在断言改查真实效果向量。
局部夹具只拥有该入口字段和输入路径，不构造App，不执行网关任务。错误字段仅用于保留原宿主反馈断言，依据真实 Error/InputError 效果投影。

- `commit_selected_rejects_blank_message_without_starting_commit` → `host_module_13_tests::commit_selected_rejects_blank_message_without_starting_commit`。
- `commit_all_reuses_commit_validation` → `host_module_13_tests::commit_all_reuses_commit_validation`。
- `stage_all_requires_project_context_and_reports_finished_states` → `host_module_13_tests::stage_all_requires_project_context_and_reports_finished_states`。
- `commit_selected_while_in_progress_preserves_existing_state` → `host_module_13_tests::commit_selected_while_in_progress_preserves_existing_state`。
- `commit_selected_without_project_path_restores_idle_state_and_reports_error` → `host_module_13_tests::commit_selected_without_project_path_restores_idle_state_and_reports_error`。
- `commit_selected_with_valid_context_enters_in_progress_state` → `host_module_13_tests::commit_selected_with_valid_context_enters_in_progress_state`。
- `commit_selected_finished_success_resets_form_and_reports_success` → `host_module_13_tests::commit_selected_finished_success_resets_form_and_reports_success`。
- `commit_selected_finished_error_keeps_form_and_reports_error` → `host_module_13_tests::commit_selected_finished_error_keeps_form_and_reports_error`。

## 当前静态证据

- 修改文件独立 rustfmt（skip_children=true），限定本任务路径的 git diff --check 已执行无输出；全仓检查发现其它并行任务文件的空白问题，未改动这些文件。
- baseline.json task=13 来源 5488 行，当前 5131 行，净减 357 行；其中生产候选 1971 → 1796，净减 175 行。无本任务新增宿主文件。
- 领域追加真实 mod/update 导出及 cfg(test) 测试注册，宿主移除旧测试挂载。
- 用户禁测：没有执行 Cargo、单元测试、实机绘制或最终回归；不声明编译或行为通过。
- 未来聚焦命令：`python3 scripts/cargo_slot.py cargo test -p desktop-git-ui --lib host_module_13_tests -- --test-threads=1`（本轮未运行）。

## 尚未完成，不能删除任务卡

本检查点只完成暂存提交控制器，不满足整项静态交付契约。
仍须迁移 diff、modal、refresh 的完整更新入口与全部领域测试；整合 git_panel 页面装配，迁移剩余领域夹具；将领域字段所有权收敛到领域状态，逐函数完成剩余源文件职责和断言审计。
本次借用输入尚未代替App中分散的领域字段。现有 repository/history 已有领域入口，但整组宿主测试归属仍待核对。
尚无客观外部阻塞，不能把这些未完成实现记为待回归，也不能宣称任务13完成。
