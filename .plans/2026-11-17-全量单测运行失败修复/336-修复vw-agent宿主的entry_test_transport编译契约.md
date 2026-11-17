# 任务336：修复vw-agent宿主的entry_test_transport编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：已完成

来源：继续执行宿主单测；诊断编号76（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/workflow/video_edit/entry_test_transport.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断76：E0277

位置：`crates/vw-agent/src/workflow/video_edit/entry_test_transport.rs:35`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/entry_test_transport.rs:35:63
   |
35 |         workflow_video_edit::test_support::receipts::external(Arc::new(context.clone()))
   |                                                               ^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
   |
help: the trait `VideoContext` is implemented for `NativeVideoContext`
  --> crates/vw-agent/src/workflow/video_edit/prepare.rs:17:1
   |
17 | impl VideoContext for NativeVideoContext {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: required for the cast from `std::sync::Arc<tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>>` to `std::sync::Arc<(dyn VideoContext + 'static)>`
```

当前源码片段：

```rust
31:     if let Some(factory) = EXTERNALS.lock().unwrap().get(session) {
32:         return Some(factory(context));
33:     }
34:     SESSIONS.lock().unwrap().contains(session).then(|| {
35:         workflow_video_edit::test_support::receipts::external(Arc::new(context.clone()))
36:     })
37: }
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib get -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 执行结果

- 默认 receipts 外部传输使用现有 NativeVideoContext 包装宿主上下文，与生产 entry 边界一致；仅修改本任务源码文件。
- 修复前运行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list`，退出101：复现本文件 E0277，另有 prepare_tests 的5处错误。get 是夹具而非测试入口。
- 首次修复后定向执行仍退出101，仅余任务337的5处错误；等待其修复并结束编译后重试。
- 精确验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence -- --nocapture`。
- 最终退出0，实际1 passed、0 failed、1900 filtered out；完整名称 gateway::api::handlers::session::stream::stream_tests::video_edit_entry_gateway_unbound_projects_template_edit_menu_and_persistence。
- 全部命令槽位1，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-1；自有命令退出并释放槽位。日志 /tmp/shiju-336-before.log、/tmp/shiju-336-after.log、/tmp/shiju-336-final.log。
- `git diff --check` 通过；未重复check，最终全量验收仍归任务303。
