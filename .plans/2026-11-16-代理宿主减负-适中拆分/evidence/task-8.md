# 任务8：历史裁剪停止边界

实现完成；用户明确覆盖验证及任务保留要求。本次未运行任何 Cargo 命令、单元测试、实机测试或工具链验证，全部验证延期；无退出码、通过数、失败数或 ignored 数证据。删除自身编号任务文件，plan.md/prompt.md 只读。

读取前置 evidence/baseline.md（任务1实际产物）和 evidence/task-7.md。当前目录当前分支实施。

## 断言映射

以下三个原宿主用例完整迁入 session-engine/src/host_split_8_tests.rs，名称不变：

| 用例 | 领域保留断言 | 宿主保留 |
| --- | --- | --- |
| prune_stops_at_summary_messages | 摘要前两条已完成工具输出的 compacted 均为 None | 无 |
| prune_keeps_candidate_when_prunable_tokens_equal_minimum | 80000 字符候选与120000字符保护预算下，恰好最小阈值不裁剪，工具 compacted 均为 None | 无 |
| prune_stops_at_previously_compacted_tool | 两条更旧工具保持 None；停止点 compacted 保持 Some(42) | 无 |

源码候选：领域新增3条、宿主移除3条、指定宿主保留0条。没有编译测试清单，不能当作实际测试发现数量或通过数。

## 接线与夹具

- lib.rs 以 cfg(all(test, not(target_arch = "wasm32"))) 独立注册 host_split_8_tests；真实临时文件存储仅支持原生目标，因此不宣称 wasm 覆盖。
- 直接调用 crate::compaction::prune 和 crate::message；宿主 compaction.rs 已 re-export 同一个 session_engine::compaction::prune，沿用真实调用接线，无需改生产实现。未复制算法，未引入宿主反向依赖。
- 只复制本组使用的消息/已完成工具构造、写入、清理夹具；没有模型、摘要生成或完整宿主环境。原宿主夹具仍供其它测试使用，保持不动。
- agent-storage 和 agent-paths 使用进程级缓存。每个测试在父进程持有 tempfile::TempDir，通过当前 lib-test 可执行文件的精确同名过滤在子进程运行，启动前设置 VIBEWINDOW_TEST_ROOT/HOME，退出后释放临时目录；不使用不安全的进程内环境修改。子进程直接调用真实持久化/裁剪入口。使用已有 tempfile、uuid、agent-storage 依赖，无清单变更。
- 对领域相邻测试的静态检索未发现这三个停止边界的等价独立用例；不删除其它覆盖。保留原循环断言与三个逐项映射断言。
- 未改摘要生成、溢出预算、checkpoint、其它裁剪测试族、plan.md 或 prompt.md。

## 后续验证

以后统一执行 session-engine --lib host_split_8_tests 定向测试（源码预期3条），确认各隔离子进程实际运行同名用例；编译宿主 lib-test 并取得注册清单，确认三个旧名不再归属宿主。迁移前基线及迁移后所有测试均未运行，不声称行为验收通过。
