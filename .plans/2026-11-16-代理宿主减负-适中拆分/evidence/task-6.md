# 任务6：计划成功与重试事件契约

实现完成。用户明确覆盖验证要求：未运行任何 Cargo 命令、单元测试或实机测试；运行退出码、通过数、失败数和 ignored 数无数据，全部延期统一验证。下述数量仅为源码迁移数量，不代表编译或运行清单。

## 断言映射

- `task_plan_run::runner_tests::run_plan_emits_full_success_event_sequence` → `host_split_6_tests::run_plan_emits_full_success_event_sequence`：保留真实计划文件生成、完整非 delta 事件顺序、PlanningStarted 的 task_id/路径/session_id、规划实时 delta、PlanCreated 子任务数、两批成功结果、子任务会话 ID、每个子任务 delta 先于结束、最终 completed 和 goal 会话、四次调用及 root 和提示词计划路径断言。
- `task_plan_run::runner_tests::run_plan_retries_failed_subtask_without_replanning` → `host_split_6_tests::run_plan_retries_failed_subtask_without_replanning`：保留无 Blocked、最终 completed、规划恰好一次、两次重试 delta、五次总调用、规划一次、失败子任务三次和后续批次继续断言。

宿主移出2条，领域新增2条，宿主保留本组断言0；原用例名保持。其余目标模式、多层子代理、ACP full access、网关 options 和实时存储接线测试留宿主，不修改。

## 最小夹具与接线

领域 lib.rs 独立注册 host_split_6_tests。ScriptedSessionRunner 仅保留脚本队列、一次性计划文件夹具、任务文件列表、调用记录和 delta 转发；删除目标模式开关及无计划文件构造入口。沿用真实 run_task_plan_in_home、文档加载、批次调度、重试和临时文件路径，无测试专用算法、宿主引用或反向依赖。脚本中的完成清单文字是原有合成测试数据，不是本次执行证据。

任务5和本组都需要全局宿主，故增加直接注册的 test_host.rs，任务5改为共用 Once 安装，避免 OnceLock 重复安装。最小宿主仅在内存预留 Info 并接受已预留会话的首次回复提交；不测试真实存储持久性。真实项目解析及临时目录沿用领域路径；未增加依赖。取消场景断言全部保留。

已读取任务5短证据，RustCodeGraph 节点行号与当前内容不一致，具名函数和直接夹具回退读取当前源码。真实宿主 runner.rs 原有 ensure_host → task_plan_runner 转发已有接线，无需改 mod.rs 或生产执行器。额外修改任务5测试和新增 test_host.rs 的理由是上述全局宿主唯一安装约束。

## 延期事项

原宿主 lib 注册清单、领域 host_split_6_tests 两条及共享宿主影响的 host_split_5_tests 一条，均留统一验证。公共检查同样未运行。plan.md/prompt.md 保持只读；按用户要求删除自身编号任务文件。
