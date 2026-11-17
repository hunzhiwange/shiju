# 任务314：修复vw-agent宿主的tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号3（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/llm-runtime/tests/host_suites/tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断3：E0432

位置：`crates/libs/llm-runtime/tests/host_suites/tests.rs:9`。

```text
error[E0432]: unresolved import `super::acp::is_acp_session_changed_message`
 --> crates/vw-agent/src/session/llm/../../../../libs/llm-runtime/tests/host_suites/tests.rs:9:41
  |
9 |     extract_tool_call_from_acp_message, is_acp_session_changed_message, lookup_acp_command,
  |                                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `is_acp_session_changed_message` in `session::llm::acp`
```

当前源码片段：

```rust
5:
6: use super::acp::{
7:     AcpReplayStrategy, ParsedAcpOptions, acp_session_name, build_acp_command_line,
8:     build_replay_prompt, extract_delta_from_acp_message, extract_reasoning_delta_from_acp_message,
9:     extract_tool_call_from_acp_message, is_acp_session_changed_message, lookup_acp_command,
10:     missing_session_error, normalize_acp_agent_config, parse_acp_options, parse_replay_strategy,
11: };
12: use serde_json::json;
13: use acp_types::{AcpSessionOptions, AuthPolicy, NonInteractivePermissionPolicy, PermissionMode};
14:
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib -- --list
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本次执行记录（2026-11-18）

- 宿主 ACP 聚合模块未导出匹配函数；测试改从真实归属 `llm_runtime::acp` 导入，不修改生产接口、不弱化断言。
- 修复前执行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp_session_changed_matcher_accepts_nested_acp_prefixes -- --exact`，退出101，53条编译错误；未执行测试（此未限定模块的 exact 过滤不作为验收命令）。
- 已核对源码 `#[test]` 与宿主 `session::llm::tests` 注册。正确验收命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib session::llm::tests::acp_session_changed_matcher_accepts_nested_acp_prefixes -- --exact`。
- 修复后上述正确命令退出101，52条其它编译错误；日志 `/tmp/shiju-task314-after.log` 不再包含本文件或该导入错误。有效执行数0，不能宣称行为通过。阻塞包含 ACP_CLIENT_CACHE、上下文治理函数、VideoContext 等其它归属错误。
- 两次均槽位1，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。入口已退出并自动清理自有进程与锁；不删除后续任务重新领取的锁。
- `git diff --check -- crates/libs/llm-runtime/tests/host_suites/tests.rs` 通过。其余宿主任务解除编译阻塞后须运行上述精确测试，ran>0、failed=0 才可完成并删卡。
