# 任务318：修复vw-agent宿主的tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号59, 60, 61, 62（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/tool-agents/src/session_tool/tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断59：E0599

位置：`crates/libs/tool-agents/src/session_tool/tests.rs:339`。

```text
error[E0599]: no method named `mark_succeeded` found for struct `session_engine::task_session::TaskSessionCoordinator` in the current scope
   --> crates/vw-agent/src/tools/session_tool/../../../../libs/tool-agents/src/session_tool/tests.rs:339:17
    |
339 |     coordinator.mark_succeeded(&parent.info.id, &task.info.id).expect("finish");
    |                 ^^^^^^^^^^^^^^ method not found in `session_engine::task_session::TaskSessionCoordinator`
```

当前源码片段：

```rust
335:     let task = create_task(project.path(), &parent.info.id).await;
336:     let coordinator = TaskSessionCoordinator::new();
337:     coordinator.register(&parent.info.id, &task.info.id, None).expect("register");
338:     coordinator.mark_running(&parent.info.id, &task.info.id).expect("start");
339:     coordinator.mark_succeeded(&parent.info.id, &task.info.id).expect("finish");
340:     let (request_tx, request_rx) = oneshot::channel();
341:     let request_tx = std::sync::Mutex::new(Some(request_tx));
342:     let executor = Arc::new(
343:         move |_directory: String, request: vw_api_chat::chat::GatewayChatStreamRequest| {
344:             request_tx
```

### 宿主诊断60：E0624

位置：`crates/libs/tool-agents/src/session_tool/tests.rs:407`。

```text
error[E0624]: method `spawn_background_turn` is private
   --> crates/vw-agent/src/tools/session_tool/../../../../libs/tool-agents/src/session_tool/tests.rs:407:10
    |
407 |           .spawn_background_turn(parent.info.id, task.info, "Continue".into())
    |            ^^^^^^^^^^^^^^^^^^^^^ private method
    |
   ::: crates/libs/tool-agents/src/session_tool/mod.rs:177:5
    |
177 | /     fn spawn_background_turn(
178 | |         &self,
179 | |         parent_session_id: String,
180 | |         task: session::Info,
181 | |         prompt: String,
182 | |     ) -> anyhow::Result<()> {
    | |___________________________- private method defined here
```

当前源码片段：

```rust
403:         .with_coordinator(TaskSessionCoordinator::new())
404:         .with_stream_executor(executor);
405:
406:     let error = tool
407:         .spawn_background_turn(parent.info.id, task.info, "Continue".into())
408:         .expect_err("unregistered task startup must fail");
409:
410:     assert!(error.to_string().contains("unknown task session"));
411:     assert!(!executor_called.load(std::sync::atomic::Ordering::SeqCst));
412: }
```

### 宿主诊断61：E0599

位置：`crates/libs/tool-agents/src/session_tool/tests.rs:434`。

```text
error[E0599]: no method named `mark_succeeded` found for struct `session_engine::task_session::TaskSessionCoordinator` in the current scope
   --> crates/vw-agent/src/tools/session_tool/../../../../libs/tool-agents/src/session_tool/tests.rs:434:17
    |
434 |     coordinator.mark_succeeded(&parent.info.id, &task.info.id).expect("finish");
    |                 ^^^^^^^^^^^^^^ method not found in `session_engine::task_session::TaskSessionCoordinator`
```

当前源码片段：

```rust
430:         .expect("internal child session");
431:     let coordinator = TaskSessionCoordinator::new();
432:     coordinator.register(&parent.info.id, &task.info.id, None).expect("register");
433:     coordinator.mark_running(&parent.info.id, &task.info.id).expect("start");
434:     coordinator.mark_succeeded(&parent.info.id, &task.info.id).expect("finish");
435:     let tool = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
436:         .with_coordinator(coordinator);
437:
438:     let listed = tool.execute(json!({"action":"list"})).await.expect("list");
439:     let listed: serde_json::Value = serde_json::from_str(&listed.output).expect("list JSON");
```

### 宿主诊断62：E0599

位置：`crates/libs/tool-agents/src/session_tool/tests.rs:540`。

```text
error[E0599]: no method named `mark_succeeded` found for struct `session_engine::task_session::TaskSessionCoordinator` in the current scope
   --> crates/vw-agent/src/tools/session_tool/../../../../libs/tool-agents/src/session_tool/tests.rs:540:21
    |
540 |         coordinator.mark_succeeded(&parent_id, &first_id).expect("finish first");
    |                     ^^^^^^^^^^^^^^ method not found in `session_engine::task_session::TaskSessionCoordinator`
```

当前源码片段：

```rust
536:     let first_id = first.info.id.clone();
537:     let parent_id = parent.info.id.clone();
538:     let transition = tokio::spawn(async move {
539:         tokio::task::yield_now().await;
540:         coordinator.mark_succeeded(&parent_id, &first_id).expect("finish first");
541:     });
542:     let result = tool.execute(json!({"action":"wait","session_ids":[first.info.id, second.info.id],"condition":"any","timeout_ms":1000})).await.expect("wait");
543:     transition.await.expect("transition");
544:     let output: serde_json::Value = serde_json::from_str(&result.output).expect("JSON");
545:     assert_eq!(output["timed_out"], false);
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib background_turn_reports_startup_failure_instead_of_succeeding_silently -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session_tool_lists_statuses_reads_and_rejects_non_task_targets -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session_tool_send_continues_existing_visible_task -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session_tool_wait_returns_first_changed_target -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 执行记录

- 共享接口写入归属：本任务需在 `crates/libs/tool-agents/src/session_tool/mod.rs` 将后台启动入口按相邻 `validated_target` 模式标记 doc(hidden) public，供宿主注册的现有测试保留启动失败断言；当前该文件无未提交改动。协调器测试便捷方法仅在依赖自身 cfg(test) 下存在，夹具改用真实公开 finish_turn 接口。
- 当前已有其他 Cargo/rustc 编译运行，等待重型编译退出后串行验证。

### 验证结果（2026-11-18）

- 四个候选均为真实 tokio 测试。第一次使用单槽位 sh -c 入口按原命令顺序验证，第一条编译退出101，其余未执行；实际运行0项。
- 重试命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib background_turn_reports_startup_failure_instead_of_succeeding_silently -- --nocapture`，退出101，实际运行0项。日志 `/tmp/shiju-task318-validation.log`。
- 两次均槽位1；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。自有入口已退出并释放锁。
- 本任务四条诊断已消失；宿主仍有44条其他编译错误，包括 thread_tool/tests.rs:437 的 mark_succeeded、tool-computer 私有方法和 VideoContext 契约。保留所有断言，未接管兄弟文件。
- 两个源码文件的 git diff --check 通过；代码已提交为 95d3ae7b9。仍须待兄弟编译阻塞解除后运行原列四个测试，各要求实际数>0、failed=0、退出0。当前不能宣称行为通过，保留任务卡。
