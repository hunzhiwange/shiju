# 任务333：修复vw-agent宿主的cloud_prepare_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号71（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断71：E0277

位置：`crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs:16`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs:16:145
   |
16 | ... root, &ctx).await;
   |           ^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
   |
help: the trait `VideoContext` is implemented for `NativeVideoContext`
  --> crates/vw-agent/src/workflow/video_edit/prepare.rs:17:1
   |
17 | impl VideoContext for NativeVideoContext {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: required for the cast from `&tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>` to `&dyn VideoContext`
```

当前源码片段：

```rust
12:     .await
13:     .unwrap();
14:     let hub = skill.location.unwrap().parent().unwrap().to_path_buf();
15:     let root = hub.join("cutvideo");
16:     workflow_video_edit::test_support::contracts::video_edit_prepare_native_receipts_keep_signed_tickets_in_memory(&provider, dir.path(), root, &ctx).await;
17: }
18: #[tokio::test]
19: async fn video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch() {
20:     let (_dir, ctx, skill) = super::cloud_tests::fixture("ok");
21:     assert_ne!(ctx.tool_use_context().tenant_scope().company_id().as_ref(), "10011");
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_prepare_native_receipts_keep_signed_tickets_in_memory -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本轮执行记录

- 根因：共享合同现接收 `&dyn VideoContext`，宿主测试仍传裸 `ToolRuntimeContext`；复用生产 cloud::discover 使用的 `prepare::NativeVideoContext` 包装原上下文，保留真实策略、执行路径及全部安全断言，仅修改本任务归属文件。
- 真实入口已确认是 `#[tokio::test] video_edit_prepare_native_receipts_keep_signed_tickets_in_memory`。
- 修改前执行任务指定精确命令：退出101，宿主15条编译错误，有效测试0。槽位1，双缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 修改后执行同命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_prepare_native_receipts_keep_signed_tickets_in_memory -- --nocapture`，退出101，宿主剩14条编译错误，有效测试0；完整日志 `/tmp/shiju-task333-after.log`。槽位3，双缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 本文件E0277已消失；剩余 controls_tests 的 ACP_CLIENT_CACHE 及 prepare_tests、command_tests、entry_test_transport、e2e_tests 的 VideoContext 错误归兄弟任务，不接管其文件。待其解除后须在本任务重跑上述真实测试，不能将编译诊断消失记为行为通过。
- 轻量 `git diff --check -- crates/vw-agent/src/workflow/video_edit/cloud_prepare_tests.rs` 通过。未运行重复check、桌面构建或全量测试。两次入口及自有子进程已退出，槽位自动释放，保留缓存。
