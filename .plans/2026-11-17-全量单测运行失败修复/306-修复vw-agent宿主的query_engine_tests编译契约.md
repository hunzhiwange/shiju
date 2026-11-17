# 任务306：修复vw-agent宿主的query_engine_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号37, 38（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/agent-loop/src/query_engine_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断37：E0308

位置：`crates/libs/agent-loop/src/query_engine_tests.rs:412`。

```text
error[E0308]: mismatched types
   --> crates/vw-agent/src/agent/loop_/../../../../libs/agent-loop/src/query_engine_tests.rs:412:32
    |
412 |     engine.hook_runtime = Some(runtime.clone());
    |                           ---- ^^^^^^^^^^^^^^^ expected `agent_hooks::HookRuntime`, found `hooks::runtime::HookRuntime`
    |                           |
    |                           arguments to this enum variant are incorrect
    |
    = note: `hooks::runtime::HookRuntime` and `agent_hooks::HookRuntime` have similar names, but are actually distinct types
note: `hooks::runtime::HookRuntime` is defined in the current crate
   --> crates/vw-agent/src/hooks/runtime.rs:9:1
    |
  9 | pub struct HookRuntime(agent_hooks::HookRuntime);
    | ^^^^^^^^^^^^^^^^^^^^^^
note: `agent_hooks::HookRuntime` is defined in crate `agent_hooks`
   --> crates/libs/agent-hooks/src/runtime.rs:21:1
    |
 21 | pub struct HookRuntime {
    | ^^^^^^^^^^^^^^^^^^^^^^
help: the type constructed contains `hooks::runtime::HookRuntime` due to the type of the argument passed
   --> crates/vw-agent/src/agent/loop_/../../../../libs/agent-loop/src/query_engine_tests.rs:412:27
    |
412 |     engine.hook_runtime = Some(runtime.clone());
    |                           ^^^^^---------------^
    |                                |
    |                                this argument influences the type of `Some`
note: tuple variant defined here
   --> /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library/core/src/option.rs:606:4
```

当前源码片段：

```rust
408:     engine.tools_registry = vec![Box::new(HookProbeTool)];
409:     engine.compact_context = true;
410:     engine.history =
411:         vec![ChatMessage::user("previous task"), ChatMessage::assistant("previous result")];
412:     engine.hook_runtime = Some(runtime.clone());
413:
414:     let response = engine.submit_message("exercise primary and retry").await.unwrap();
415:
416:     assert_eq!(response, "recovered");
417:     assert_eq!(runtime.audit_store().entries().len(), 2);
```

### 宿主诊断38：E0308

位置：`crates/libs/agent-loop/src/query_engine_tests.rs:432`。

```text
error[E0308]: mismatched types
   --> crates/vw-agent/src/agent/loop_/../../../../libs/agent-loop/src/query_engine_tests.rs:432:32
    |
432 |     engine.hook_runtime = Some(disabled.clone());
    |                           ---- ^^^^^^^^^^^^^^^^ expected `agent_hooks::HookRuntime`, found `hooks::runtime::HookRuntime`
    |                           |
    |                           arguments to this enum variant are incorrect
    |
    = note: `hooks::runtime::HookRuntime` and `agent_hooks::HookRuntime` have similar names, but are actually distinct types
note: `hooks::runtime::HookRuntime` is defined in the current crate
   --> crates/vw-agent/src/hooks/runtime.rs:9:1
    |
  9 | pub struct HookRuntime(agent_hooks::HookRuntime);
    | ^^^^^^^^^^^^^^^^^^^^^^
note: `agent_hooks::HookRuntime` is defined in crate `agent_hooks`
   --> crates/libs/agent-hooks/src/runtime.rs:21:1
    |
 21 | pub struct HookRuntime {
    | ^^^^^^^^^^^^^^^^^^^^^^
help: the type constructed contains `hooks::runtime::HookRuntime` due to the type of the argument passed
   --> crates/vw-agent/src/agent/loop_/../../../../libs/agent-loop/src/query_engine_tests.rs:432:27
    |
432 |     engine.hook_runtime = Some(disabled.clone());
    |                           ^^^^^----------------^
    |                                |
    |                                this argument influences the type of `Some`
note: tuple variant defined here
   --> /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library/core/src/option.rs:606:4
```

当前源码片段：

```rust
428:         ProviderStep::Response(chat_text("disabled", None)),
429:     ]);
430:     let mut engine = test_engine(provider, Arc::new(FakeMemory::default()));
431:     engine.tools_registry = vec![Box::new(HookProbeTool)];
432:     engine.hook_runtime = Some(disabled.clone());
433:
434:     assert_eq!(engine.submit_message("hooks disabled").await.unwrap(), "disabled");
435:     assert!(disabled.audit_store().entries().is_empty());
436: }
437:
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib query_engine_uses_hook_runtime_for_primary_and_retry_turns -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本次执行记录（2026-11-18）

- 已提交 `54207264c`：两处运行时赋值沿用网关生产调用的 `HookRuntime::shared()`，保留原有审计及禁用断言；仅修改本任务测试文件。
- 原红灯：宿主诊断37、38的E0308。等待已有宿主编译退出后执行定向命令，等待期间完成最小修复，因此本次日志是修复后尝试，不是修复前复现。
- 验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib query_engine_uses_hook_runtime_for_primary_and_retry_turns -- --nocapture`。
- 槽位1；`CARGO_TARGET_DIR`、`CARGO_BUILD_BUILD_DIR`均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 退出101；宿主lib test仍有70条其它编译错误（包括TaskSessionCoordinator旧方法、私有spawn_background_turn及VideoContext契约）。本文件两处E0308已不在此次输出中；有效执行测试0，不能记为行为通过。日志：`/tmp/shiju-306-red.log`。
- 自有验证入口已退出并释放槽位；`git diff --check`通过。保留编号文件，待同宿主其余编译阻塞解除后按上述命令运行真实用例，要求有效数>0、failed=0、退出0，再收尾。
