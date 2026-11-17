# 任务330：修复vw-agent宿主的t42_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号4, 32（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/plugins/compat/t42_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断4：E0432

位置：`crates/vw-agent/src/plugins/compat/t42_tests.rs:53`。

```text
error[E0432]: unresolved imports `compaction::compaction_tests::model_with_limits`, `compaction::compaction_tests::user_message`
  --> crates/vw-agent/src/plugins/compat/t42_tests.rs:53:40
   |
53 |     use compaction::compaction_tests::{model_with_limits, user_message};
   |                                        ^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^ no `user_message` in `session::compaction::compaction_tests`
   |                                        |
   |                                        no `model_with_limits` in `session::compaction::compaction_tests`
```

当前源码片段：

```rust
49:     session: &str,
50:     auto: bool,
51:     fail: bool,
52: ) -> Result<CompactionOutcome, crate::session::session::Error> {
53:     use compaction::compaction_tests::{model_with_limits, user_message};
54:     compaction::process_with_summarizer(
55:         ProcessInput {
56:             session_id: session.into(),
57:             parent_id: "boundary".into(),
58:             auto,
```

### 宿主诊断32：E0425

位置：`crates/vw-agent/src/plugins/compat/t42_tests.rs:54`。

```text
error[E0425]: cannot find function `process_with_summarizer` in module `compaction`
  --> crates/vw-agent/src/plugins/compat/t42_tests.rs:54:17
   |
54 |     compaction::process_with_summarizer(
   |                 ^^^^^^^^^^^^^^^^^^^^^^^ not found in `compaction`
```

当前源码片段：

```rust
50:     auto: bool,
51:     fail: bool,
52: ) -> Result<CompactionOutcome, crate::session::session::Error> {
53:     use compaction::compaction_tests::{model_with_limits, user_message};
54:     compaction::process_with_summarizer(
55:         ProcessInput {
56:             session_id: session.into(),
57:             parent_id: "boundary".into(),
58:             auto,
59:             messages: vec![
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib attempt -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
