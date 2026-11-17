# 任务308：修复vw-agent宿主的execution_control_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号8（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/channel-runtime/src/manager/execution_control_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断8：E0422

位置：`crates/libs/channel-runtime/src/manager/execution_control_tests.rs:79`。

```text
error[E0422]: cannot find struct, variant or union type `ChannelTaskGuard` in this scope
  --> crates/vw-agent/src/channels/manager/../../../../libs/channel-runtime/src/manager/execution_control_tests.rs:79:21
   |
79 |         let guard = ChannelTaskGuard {
   |                     ^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
75:                 cancellation: CancellationToken::new(),
76:                 completion: completion.clone(),
77:             },
78:         );
79:         let guard = ChannelTaskGuard {
80:             ctx: ctx.clone(),
81:             scope,
82:             task_id: 1,
83:             completion: completion.clone(),
84:             gate: Some(gate),
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib channel_worker_abort_releases_busy_state_and_notifies_stop -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本次执行记录

- 根因：宿主 execution_control 只转导出 channel-runtime 公共接口，复用测试不能访问库内私有 ChannelTaskGuard；不扩大生产类型可见性。
- 改用真实宿主 process_channel_message 启动任务，以阻塞反馈夹具观察调用者 abort 后取消令牌、忙碌状态和门锁持续占用，反馈退出后验证完成通知、状态和门锁释放。保留八通道覆盖及停止断言。
- 已核对过滤入口为 #[tokio::test]。两次命令均为 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib channel_worker_abort_releases_busy_state_and_notifies_stop -- --nocapture`。
- 均使用槽位1，CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。修复前退出101，68条编译错误，复现 ChannelTaskGuard E0422（日志 /tmp/shiju-task308-red.log）；修复后退出101，67条其他编译错误，本文件无诊断（日志 /tmp/shiju-task308-after.log）。有效执行测试数0，不能记为通过。
- 其他阻塞包括 plugins/compat/t42_tests.rs 的 compaction 私有/缺失接口、acp 的 is_acp_session_changed_message 等，归兄弟任务处理。保留编号文件；待宿主阻塞解除后必须重跑上述定向测试，实际数>0且退出0才能完成。未运行额外 Cargo check。
- rustfmt 仅作用于修改文件；git diff --check 通过。两次槽位入口均已退出并释放自有锁；未清理他人进程或缓存。

- 提交未成功：反复执行仅本任务两文件的 git add / git commit --only，持续返回退出128：Unable to create .git/index.lock: File exists。通过 lsof + ps 确认锁属桌面应用父进程59757启动的 `git -C /Users/xiangmin/worker/code/shiju/. status --ignored --porcelain=v1 -z`（取证时PID97926），不是可擅自删除的遗留锁；已多轮有界等待及重试，未删除锁或终止他人进程。实际变更未提交，需锁竞争解除后使用中文信息仅提交本任务两个文件。
