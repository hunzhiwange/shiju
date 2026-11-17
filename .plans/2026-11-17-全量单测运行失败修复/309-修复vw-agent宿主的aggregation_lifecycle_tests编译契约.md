# 任务309：修复vw-agent宿主的aggregation_lifecycle_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号28, 29（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断28：E0425

位置：`crates/libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs:352`。

```text
error[E0425]: cannot find value `stop` in module `super::super::app_routing`
   --> /Users/xiangmin/worker/code/shiju/crates/vw-agent/../libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs:352:79
    |
352 |         .route("/stop/{host}", axum::routing::post(super::super::app_routing::stop))
    |                                                                               ^^^^ not found in `super::super::app_routing`
```

当前源码片段：

```rust
348:     let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
349:     policy["hosts"][0]["routing_startup"] = json!({"enabled":true,"listen_port":0});
350:     std::fs::write(&f.policy, policy.to_string()).unwrap();
351:     let router = Router::new()
352:         .route("/stop/{host}", axum::routing::post(super::super::app_routing::stop))
353:         .route("/start/{host}", axum::routing::post(super::super::app_routing::start))
354:         .layer(Extension(tenant()));
355:     let router = super::super::app_routing::configure_with_policy(
356:         router,
357:         &listener,
```

### 宿主诊断29：E0425

位置：`crates/libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs:353`。

```text
error[E0425]: cannot find value `start` in module `super::super::app_routing`
   --> /Users/xiangmin/worker/code/shiju/crates/vw-agent/../libs/gateway-aggregation/tests/host/aggregation_lifecycle_tests.rs:353:80
    |
353 |         .route("/start/{host}", axum::routing::post(super::super::app_routing::start))
    |                                                                                ^^^^^ not found in `super::super::app_routing`
    |
note: these functions exist but are inaccessible
   --> crates/vw-agent/src/tools/shell/session_tools_tests.rs:41:1
    |
 41 |   async fn start(tools: &[Box<dyn Tool>], ctx: Arc<ToolUseContext>) -> ToolCallResult {
    |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `crate::tools::shell::session_tools_tests::start`: not accessible
    |
   ::: crates/vw-agent/src/tools/shell/long_running_tests.rs:35:1
    |
 35 | / fn start<'a>(
 36 | |     root: &'a Path,
 37 | |     session: &'a str,
 38 | |     script: &str,
...   |
 55 | |     (cleanup, result)
 56 | | }
    | |_^ `crate::tools::shell::long_running_tests::start`: not accessible
help: consider importing one of these functions
    |
  1 + use crate::gateway::start;
    |
  1 + use crate::plugins::hook_session::start;
    |
  1 + use crate::session::fork_execution::start;
    |
help: if you import `start`, refer to it directly
    |
353 -         .route("/start/{host}", axum::routing::post(super::super::app_routing::start))
353 +         .route("/start/{host}", axum::routing::post(start))
    |
```

当前源码片段：

```rust
349:     policy["hosts"][0]["routing_startup"] = json!({"enabled":true,"listen_port":0});
350:     std::fs::write(&f.policy, policy.to_string()).unwrap();
351:     let router = Router::new()
352:         .route("/stop/{host}", axum::routing::post(super::super::app_routing::stop))
353:         .route("/start/{host}", axum::routing::post(super::super::app_routing::start))
354:         .layer(Extension(tenant()));
355:     let router = super::super::app_routing::configure_with_policy(
356:         router,
357:         &listener,
358:         f.policy.clone(),
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib aggregation_t19_host_stop_and_start_use_authorized_router -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
