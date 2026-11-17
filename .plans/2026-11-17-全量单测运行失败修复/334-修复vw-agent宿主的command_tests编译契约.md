# 任务334：修复vw-agent宿主的command_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号65, 66, 67, 68, 69, 70（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/workflow/video_edit/command_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断65：E0277

位置：`crates/vw-agent/src/workflow/video_edit/command_tests.rs:16`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/command_tests.rs:16:62
   |
16 |     assert!(prepare_video_edit_request("/shiju 剪辑 link.mp4", &ctx).is_err());
   |                                                                ^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
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
12:     let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()));
13:     let mut config = crate::app::agent::config::Config::default();
14:     config.autonomy.workspace_only = true;
15:     let ctx = ctx.with_test_config(config);
16:     assert!(prepare_video_edit_request("/shiju 剪辑 link.mp4", &ctx).is_err());
17:     let full = ctx.clone().with_tool_use_context(
18:         ctx.tool_use_context().as_ref().clone().with_full_access_enabled(true),
19:     );
20:     assert_eq!(
21:         prepare_video_edit_request("/shiju 剪辑 link.mp4", &full).unwrap().unwrap().video_path,
```

### 宿主诊断66：E0277

位置：`crates/vw-agent/src/workflow/video_edit/command_tests.rs:21`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/command_tests.rs:21:58
   |
21 |         prepare_video_edit_request("/shiju 剪辑 link.mp4", &full).unwrap().unwrap().video_path,
   |                                                            ^^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
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
17:     let full = ctx.clone().with_tool_use_context(
18:         ctx.tool_use_context().as_ref().clone().with_full_access_enabled(true),
19:     );
20:     assert_eq!(
21:         prepare_video_edit_request("/shiju 剪辑 link.mp4", &full).unwrap().unwrap().video_path,
22:         file.canonicalize().unwrap()
23:     );
24: }
25:
26: #[test]
```

### 宿主诊断67：E0277

位置：`crates/vw-agent/src/workflow/video_edit/command_tests.rs:39`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/command_tests.rs:39:48
   |
39 |     assert!(prepare_video_edit_request(&query, &ctx).unwrap().is_some());
   |                                                ^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
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
35:     config.autonomy.allowed_roots = vec![external.path().to_string_lossy().into_owned()];
36:     let ctx = ToolRuntimeContext::new("video", Some(root.path().to_string_lossy().into()))
37:         .with_test_config(config);
38:     let query = format!("/shiju 剪辑 '{}'", file.display());
39:     assert!(prepare_video_edit_request(&query, &ctx).unwrap().is_some());
40:     let full = ToolRuntimeContext::new("video", None).with_tool_use_context(
41:         crate::tools::ToolUseContext::new("video", None).with_full_access_enabled(true),
42:     );
43:     let req = prepare_video_edit_request(&query, &full).unwrap().unwrap();
44:     assert_eq!(req.project_root, None);
```

### 宿主诊断68：E0277

位置：`crates/vw-agent/src/workflow/video_edit/command_tests.rs:43`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/command_tests.rs:43:50
   |
43 |     let req = prepare_video_edit_request(&query, &full).unwrap().unwrap();
   |                                                  ^^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
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
39:     assert!(prepare_video_edit_request(&query, &ctx).unwrap().is_some());
40:     let full = ToolRuntimeContext::new("video", None).with_tool_use_context(
41:         crate::tools::ToolUseContext::new("video", None).with_full_access_enabled(true),
42:     );
43:     let req = prepare_video_edit_request(&query, &full).unwrap().unwrap();
44:     assert_eq!(req.project_root, None);
45:     assert!(prepare_video_edit_request("/shiju 剪辑 relative.mp4", &full).is_err());
46: }
47:
48: #[test]
```

### 宿主诊断69：E0277

位置：`crates/vw-agent/src/workflow/video_edit/command_tests.rs:45`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/command_tests.rs:45:66
   |
45 |     assert!(prepare_video_edit_request("/shiju 剪辑 relative.mp4", &full).is_err());
   |                                                                    ^^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
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
41:         crate::tools::ToolUseContext::new("video", None).with_full_access_enabled(true),
42:     );
43:     let req = prepare_video_edit_request(&query, &full).unwrap().unwrap();
44:     assert_eq!(req.project_root, None);
45:     assert!(prepare_video_edit_request("/shiju 剪辑 relative.mp4", &full).is_err());
46: }
47:
48: #[test]
49: fn video_edit_command_delegated_parent_remains_restricted_under_full_access() {
50:     use std::sync::Arc;
```

### 宿主诊断70：E0277

位置：`crates/vw-agent/src/workflow/video_edit/command_tests.rs:71`。

```text
error[E0277]: the trait bound `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>: VideoContext` is not satisfied
  --> crates/vw-agent/src/workflow/video_edit/command_tests.rs:71:80
   |
71 |         prepare_video_edit_request(&format!("/shiju 剪辑 '{}'", file.display()), &ctx).is_err()
   |                                                                                  ^^^^ the trait `VideoContext` is not implemented for `tool_assembly::runtime::ToolRuntimeContext<AgentContextHost>`
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
67:     let ctx = ToolRuntimeContext::new("child", Some(root.path().to_string_lossy().into()))
68:         .with_tool_use_context(base)
69:         .with_test_config(config);
70:     assert!(
71:         prepare_video_edit_request(&format!("/shiju 剪辑 '{}'", file.display()), &ctx).is_err()
72:     );
73: }
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_command_delegated_parent_remains_restricted_under_full_access -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_command_external_allowed_root_and_no_project -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_command_symlink_obeys_native_security_and_full_access -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 本次执行记录（2026-11-18）

- 仅修改本任务 command_tests.rs：六个调用显式包裹现有 NativeVideoContext，沿用生产 entry.rs 的适配边界，保留全部原生安全断言；未改业务接口。
- 已核对三个候选均为真实 #[test]。合并精确公共前缀运行：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib video_edit_command_ -- --nocapture`，覆盖本文件三个测试。
- 修复前：槽位1，退出101，15处编译错误，包含本文件六处E0277；日志 `/tmp/shiju-task334-before.log`。
- 修复后：槽位1，退出101，本文件诊断消失；其余8处错误属于 controls_tests.rs、prepare_tests.rs、entry_test_transport.rs 和 e2e_tests.rs。日志 `/tmp/shiju-task334-after.log`。有效执行0项，未宣称行为通过。
- 两次 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；入口已退出并释放自有槽位。只格式化修改文件，未运行桌面构建或额外check。
- 待兄弟任务解除宿主阻塞后，在本任务重跑上述过滤（预期3项），真实测试通过后方可完成删卡；全量验收仍归303。
