# 任务5：计划取消事件契约

实现已完成；按用户覆盖指令未执行任何 Cargo、单元测试或实机验证，运行退出码、通过数均无数据，统一验证延期。

## 归属映射

`task_plan_run::runner_tests::task_plan_run_root_cancellation_publishes_cancelled_once` → `host_split_5_tests::task_plan_run_root_cancellation_publishes_cancelled_once`。

原用例全部领域断言迁入：规划确实启动；根取消在有界时间内停止规划并返回终止成功；Cancelled 恰好一次；最后事件携带对应 task_id 且 session_id=None。宿主保留断言0，移出用例1，领域新增用例1（源码计数，非运行清单）。删除专用 CancellationGatedSessionRunner 的宿主定义、Default 与 SessionRunner 实现。其余测试族不动。

领域 lib.rs 独立注册模块；无 path/include 宿主引用，无宿主反向依赖。生产执行器没有改动。真实宿主 runner.rs 原有 ensure_host → task_plan_runner::run_task_plan_with_cancellation 转发保持有效，无需新增装配。

## 夹具

仅复制取消门控 runner；临时目录通过 ACP 计划路径限定在工作区，复用现有目录生成、会话绑定和取消算法。最小 CancellationHost 替代宿主端口：提供租户、预留会话、时钟与提示策略；成功回复端口明确拒绝。项目解析仍使用真实 agent-project 路径。未复制 PlanFixture（原测试未使用）。新增已有生态的 tempfile dev dependency，Cargo.lock 手工同步依赖条目，未运行 Cargo。

## 边界及后续

任务1单独证据尚未生成；已读取 before-host-tests.txt 的原用例归属和 baseline.md。物理删除84行，迁移不代表已编译验收。领域测试进程安装全局 PlanRunHost；后续同库用例若也安装宿主，应统一已有夹具以避免 OnceLock 重复安装。当前仅本模块安装测试宿主。

延期验证：原宿主 lib 注册清单、领域 host_split_5_tests 定向用例及公共检查。plan.md/prompt.md 只读。按用户要求删除编号任务文件，不因延期验证保留。
