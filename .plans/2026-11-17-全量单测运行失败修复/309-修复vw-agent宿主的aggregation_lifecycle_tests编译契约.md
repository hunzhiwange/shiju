# 任务309：修复vw-agent宿主的aggregation_lifecycle_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：待回归

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

## 本轮执行记录

- 已核对 `#[tokio::test]` 入口与宿主父模块；start/stop 实现已迁至 `gateway_app_routing::server::lifecycle_api`，测试直接引用真实处理器，configure_with_policy 仍复用宿主生命周期组装。
- 修复请求闭包遗漏的 `x-agent-apps-token`，保留未授权401、授权200、重复启动与原生配置恢复断言。
- 修复前后均执行 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib aggregation_t19_host_stop_and_start_use_authorized_router -- --nocapture`。槽位1，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 修复前编译失败67条（含本任务两条E0425）；修复后退出101、65条其他编译错误，无本文件诊断；有效运行测试0，不能视作行为通过。日志 `/tmp/shiju-task309-after.log`。阻塞示例：compat/t42_tests.rs 的 process_with_summarizer、llm transport_tests.rs 的 summary_output_budget、video_edit prepare_tests.rs 的 VideoContext 契约。
- git diff --check 定向通过；自有编译已退出。其余宿主任务解除阻塞后须重跑上述真实测试，成功前保留本文件。

## 公共产物回归记录

- 已确认提交 `93e8bb9c2` 包含两处处理器引用与请求头修复，并且是公共构建起始 HEAD `3382a35f8` 的祖先。本测试文件与该构建版本一致。
- 使用 `/tmp/shiju-host-common-304-LYrH28/receipt.md` 与 `test-binary.txt` 登记的槽位 1 产物 `target/rust-slot-1/debug/deps/vw_agent-e6b6e74ae78b3318`。构建结束 HEAD `17e53b009`，产物 SHA-256 `5548b4e2c673a564f9d12af91061c0e01e0334f68881fd8cf360a1cbf5933e78`。
- 直接以 `--exact gateway::api::handlers::aggregation_publish_tests::lifecycle_tests::aggregation_t19_host_stop_and_start_use_authorized_router --nocapture` 运行。独立进程使用隔离 HOME、CFFIXED_USER_HOME、CODEX_HOME、各 XDG 目录、TMPDIR、VIBEWINDOW_TEST_ROOT、VIBEWINDOW_TEST_HOME 和工作目录。已核对夹具数据库与原生配置写入临时根。未执行重编。
- 有效测试 1，0 passed、1 failed，退出 101。共享 `aggregation_publish_tests.rs:191` 的 `Fixture::preview` 先返回 HTTP 409 `revision_conflict`（预期 200），尚未运行启停与授权断言。日志：`/var/folders/rg/zr19lb_s5w32q6d2qkd6p0n80000gp/T/shiju-309-dygbuq6h/309-tests.log`。
- 需先排查共享发布夹具的预览失败，再用更新后的宿主产物重跑本用例。保留任务文件及既有提交。

## 配对契约迁移

- 查明预览 409 来自执行路径不一致。现行网关按租户解析隔离 home 和凭据根，旧夹具把事务写入另一个私有目录。精确用例现以独立进程启动，并在当前网关识别的租户凭据根初始化事务；数据库与原生 Claude 配置也指向相同隔离 home。
- 共用接线仅涉及父文件 `aggregation_publish_tests.rs` 的路径初始化。写入前核对该文件无其他编号写入归属和未提交差异，其他夹具仍使用原有默认路径。
- 启停使用正式应用路由及生产配对校验，发送有效或无效的 Bearer skey，保留拒绝后仍挂载、恢复原配置、重挂载与重复启动幂等断言。
- `git diff --check` 通过。按本轮指示未编译或运行更新后的源码。旧公共产物的 1 项失败仍是修复前证据。等待包含此次提交的新公共产物，按上一节精确用例在隔离环境中复验。

## 新公共产物回归

- 公共构建回执 `/tmp/shiju-host-common-310-s0GxHw/receipt.md`，槽位1，开始 HEAD `784724153`，结束 HEAD `e380dd943`。确认构建含 `905348a76`，两端任务源码一致。校验 `vw_agent-e6b6e74ae78b3318` 的 SHA-256 为 `0a610b9520d257082e0b53bfdd6cfaded6ebc56d086f4b3eb2e756d0365df7b2`。
- 使用隔离 HOME、XDG、CODEX_HOME、应用状态与工作目录，并由 macOS 沙盒拒绝真实 Codex 路径的读写。精确测试报告有效 1 项，失败 1 项，退出 101。日志 `/tmp/shiju-309-paired.srR4un/309-tests.log`，命令 `/tmp/shiju-309-paired.srR4un/command.json`。预览成功，发布进入异步执行后返回 503 `target_unavailable`。
- 发布的 `RouteAuth::issue_route` 要求监听器持有数据库以创建租户路由配置存储。确认父夹具文件无并行写入后，将默认监听器换成使用夹具数据库的监听器。此修改等待新公共产物复验，未重编。
