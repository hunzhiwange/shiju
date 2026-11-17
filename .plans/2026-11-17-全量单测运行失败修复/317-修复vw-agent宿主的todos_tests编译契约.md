# 任务317：修复vw-agent宿主的todos_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号5（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/session-engine/tests/host_suites/todos_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断5：E0433

位置：`crates/libs/session-engine/tests/host_suites/todos_tests.rs:92`。

```text
error[E0433]: cannot find `runner_todos` in the crate root
  --> crates/vw-agent/src/session/processor/../../../../libs/session-engine/tests/host_suites/todos_tests.rs:92:20
   |
92 |     assert!(crate::runner_todos::read_todos_or_empty(&ctx.session).is_empty());
   |                    ^^^^^^^^^^^^ could not find `runner_todos` in the crate root
```

当前源码片段：

```rust
88:     let mut tool_state = super::super::ToolSessionState::default();
89:
90:     super::maybe_mark_todo_in_progress(&mut session, &ctx, &mut tool_state);
91:
92:     assert!(crate::runner_todos::read_todos_or_empty(&ctx.session).is_empty());
93:     assert!(session.messages.is_empty());
94: }
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib maybe_mark_todo_in_progress_noops_for_empty_list_without_side_effects -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本轮执行记录

- 确认目标函数为 `#[test]`，宿主 `processor/todos.rs` 通过 path 注册该共享测试；宿主无根级 runner_todos，实际调用 session_engine 接口。
- 最小修复：使用现有 `read(&ctx)` 读取真实 todo 存储并解析。保留空列表及无消息副作用断言；读取/解析失败会直接失败，不以默认空列表掩盖。
- 修复前执行指定定向命令，退出101：51条宿主编译错误，包含本任务E0433。日志 `/tmp/shiju-317-red.log`。
- 修复后执行同一命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib maybe_mark_todo_in_progress_noops_for_empty_list_without_side_effects -- --nocapture`。日志 `/tmp/shiju-317-green.log`；退出101，45条其余宿主编译错误，本文件诊断消失；有效测试数0，行为验证尚未运行。两轮入口均已退出并完成自有进程收尾。
- 两轮均槽位1；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，入口负责进程清理及释放自有锁，缓存保留。
- `git diff --check` 通过。未执行整包测试、桌面构建或重复check；有效测试数0，不能记为行为通过。
- 等待兄弟任务解除 core 历史接口、compaction测试夹具、cron Config、gateway宿主导入及VideoContext契约等错误后，重跑上述同宿主定向命令，要求运行数>0、failed=0、退出0后方可完成并删除本文件。
