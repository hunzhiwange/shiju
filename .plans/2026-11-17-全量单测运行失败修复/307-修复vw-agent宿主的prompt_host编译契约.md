# 任务307：修复vw-agent宿主的prompt_host编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：继续执行宿主单测；诊断编号6, 7（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/agent-loop/src/tests/prompt_host.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断6：E0433

位置：`crates/libs/agent-loop/src/tests/prompt_host.rs:133`。

```text
error[E0433]: cannot find `ToolRuntimeContext` in `tool_types`
   --> crates/vw-agent/src/agent/prompt/../../../../libs/agent-loop/src/tests/prompt_host.rs:133:40
    |
133 |     let workflow_context = tool_types::ToolRuntimeContext::new(
    |                                        ^^^^^^^^^^^^^^^^^^ could not find `ToolRuntimeContext` in `tool_types`
    |
help: consider importing this type alias through its public re-export
    |
  2 + use crate::tools::ToolRuntimeContext;
    |
help: if you import `ToolRuntimeContext`, refer to it directly
    |
133 -     let workflow_context = tool_types::ToolRuntimeContext::new(
133 +     let workflow_context = ToolRuntimeContext::new(
    |
```

当前源码片段：

```rust
129:             "missing execution boundary '{expected}':\n{execution}"
130:         );
131:     }
132:
133:     let workflow_context = tool_types::ToolRuntimeContext::new(
134:         "workflow-subtask-session",
135:         Some(temp.path().to_string_lossy().to_string()),
136:     );
137:     let agent_tool = tool_types::tool_specs_for_context(&workflow_context)
138:         .into_iter()
```

### 宿主诊断7：E0425

位置：`crates/libs/agent-loop/src/tests/prompt_host.rs:137`。

```text
error[E0425]: cannot find function `tool_specs_for_context` in crate `tool_types`
   --> crates/vw-agent/src/agent/prompt/../../../../libs/agent-loop/src/tests/prompt_host.rs:137:34
    |
137 |     let agent_tool = tool_types::tool_specs_for_context(&workflow_context)
    |                                  ^^^^^^^^^^^^^^^^^^^^^^ not found in `tool_types`
```

当前源码片段：

```rust
133:     let workflow_context = tool_types::ToolRuntimeContext::new(
134:         "workflow-subtask-session",
135:         Some(temp.path().to_string_lossy().to_string()),
136:     );
137:     let agent_tool = tool_types::tool_specs_for_context(&workflow_context)
138:         .into_iter()
139:         .find(|spec| spec.id == "AgentTool")
140:         .expect("default workflow/goal session toolset should include AgentTool");
141:     for action in ["launch", "run", "status", "wait", "stop"] {
142:         assert!(
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib base_and_task_plan_prompts_share_recursive_orchestration_policy -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。

## 执行记录

- 2026-11-18：核对真实宿主注册及 tools/toolset：ToolRuntimeContext 是绑定 AgentContextHost 的别名，tool_specs_for_context 由宿主公开导出。仅将测试中两处 tool_types 路径迁移为 crate::tools，保留全部编排与动作枚举断言。
- 原始诊断6/7为修改前编译红灯；当前其他 Cargo 正在运行，先等待重型编译资源，再执行本任务定向测试。
- 验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib base_and_task_plan_prompts_share_recursive_orchestration_policy -- --nocapture`；已由源码第64行确认是 `#[test]` 入口。
- 实际使用槽位1，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。日志 `/tmp/shiju-307-before.log`（名称虽为 before，因排队期间先完成最小修复，实际记录修改后编译）。原诊断6/7作为修改前红灯；不声称取得新的修改前测试结果。
- Cargo 编译失败（退出101；外层等待脚本未透传退出码而返回0），68条其余宿主错误，实际执行测试数0，不能认定行为通过。未再出现 prompt_host.rs 诊断。首项阻塞为 core::build_native_assistant_history_from_parsed_calls 导入不存在，另有 runner_todos、ChannelTaskGuard 等其他编号归属错误。
- `git diff --check -- crates/libs/agent-loop/src/tests/prompt_host.rs` 通过。槽位入口与自有子进程已退出，未清除其他任务缓存或锁。待其余宿主编译任务修复后，必须重跑以上命令并取得有效数>0、failed=0、退出0，再清理本任务文件。
