# 任务327：修复vw-agent宿主的turn_fork_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号19, 20, 21, 22, 23, 24, 25（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断19：E0433

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:18`。

```text
error[E0433]: cannot find module or crate `ui_store` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:18:26
   |
18 |         .layer(Extension(ui_store::current_tenant_scope()))
   |                          ^^^^^^^^ use of unresolved module or unlinked crate `ui_store`
   |
   = help: if you wanted to use a crate named `ui_store`, use `cargo add ui_store` to add it to your `Cargo.toml`
help: consider importing this module
   |
 1 + use crate::session::ui_store;
   |
```

当前源码片段：

```rust
14: 
15: async fn request(method: Method, path: &str, directory: &str, body: Value) -> (StatusCode, Value) {
16:     let response = super::super::super::router()
17:         .with_state(state())
18:         .layer(Extension(ui_store::current_tenant_scope()))
19:         .oneshot(
20:             Request::builder()
21:                 .method(method)
22:                 .uri(path)
23:                 .header("content-type", "application/json")
```

### 宿主诊断20：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:90`。

```text
error[E0425]: cannot find function `with_tenant_instance` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:90:5
   |
90 |     with_tenant_instance(ui_store::current_tenant_scope(), source.directory.clone(), move || {
   |     ^^^^^^^^^^^^^^^^^^^^ not found in this scope
   |
note: function `crate::gateway::router::router_tests::handlers::session::stream::stream_tests::with_tenant_instance` exists but is inaccessible
  --> crates/vw-agent/src/gateway/api/handlers/session/shared.rs:16:1
   |
16 | / pub(super) async fn with_tenant_instance<T>(
17 | |     tenant: TenantScope,
18 | |     directory: String,
19 | |     f: impl FnOnce() -> crate::app::agent::project::BoxFuture<Result<T, ApiError>> + Send + 'static,
...  |
24 | |     ui_store::with_tenant_scope(tenant, with_instance(directory, f)).await
25 | | }
   | |_^ not accessible
```

当前源码片段：

```rust
86:     directory
87: }
88: 
89: async fn worktree_cleanup(source: &agent_session::session::Info, target: String) {
90:     with_tenant_instance(ui_store::current_tenant_scope(), source.directory.clone(), move || {
91:         Box::pin(async move {
92:             crate::worktree::remove_fork_checkout(target)
93:                 .await
94:                 .map_err(|error| ApiError::internal(error.to_string()))
95:         })
```

### 宿主诊断21：E0433

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:90`。

```text
error[E0433]: cannot find module or crate `ui_store` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:90:26
   |
90 |     with_tenant_instance(ui_store::current_tenant_scope(), source.directory.clone(), move || {
   |                          ^^^^^^^^ use of unresolved module or unlinked crate `ui_store`
   |
   = help: if you wanted to use a crate named `ui_store`, use `cargo add ui_store` to add it to your `Cargo.toml`
help: consider importing this module
   |
 1 + use crate::session::ui_store;
   |
```

当前源码片段：

```rust
86:     directory
87: }
88: 
89: async fn worktree_cleanup(source: &agent_session::session::Info, target: String) {
90:     with_tenant_instance(ui_store::current_tenant_scope(), source.directory.clone(), move || {
91:         Box::pin(async move {
92:             crate::worktree::remove_fork_checkout(target)
93:                 .await
94:                 .map_err(|error| ApiError::internal(error.to_string()))
95:         })
```

### 宿主诊断22：E0433

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:103`。

```text
error[E0433]: cannot find module or crate `ui_store` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:103:26
    |
103 |         .layer(Extension(ui_store::current_tenant_scope()))
    |                          ^^^^^^^^ use of unresolved module or unlinked crate `ui_store`
    |
    = help: if you wanted to use a crate named `ui_store`, use `cargo add ui_store` to add it to your `Cargo.toml`
help: consider importing this module
    |
  1 + use crate::session::ui_store;
    |
```

当前源码片段：

```rust
99: }
100: 
101: fn acceptance_app() -> axum::Router {
102:     super::super::super::router().with_state(state())
103:         .layer(Extension(ui_store::current_tenant_scope()))
104: }
105: #[tokio::test]
106: async fn session_fork_before_turn_keeps_only_complete_earlier_turns() {
107:     gateway_session::fork_acceptance::session_fork_before_turn_keeps_only_complete_earlier_turns::<super::super::super::message_ops::Host, super::super::super::stream::Host>(acceptance_app).await;
108: }
```

### 宿主诊断23：E0433

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:187`。

```text
error[E0433]: cannot find module or crate `ui_store` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:187:5
    |
187 |     ui_store::with_tenant_scope(tenant.clone(), async {
    |     ^^^^^^^^ use of unresolved module or unlinked crate `ui_store`
    |
    = help: if you wanted to use a crate named `ui_store`, use `cargo add ui_store` to add it to your `Cargo.toml`
help: consider importing this module
    |
  1 + use crate::session::ui_store;
    |
```

当前源码片段：

```rust
183: async fn worktree_environment_fork_waits_for_setup_and_rejects_failure() {
184:     let _guard = SESSION_OPS_TEST_LOCK.lock().await;
185:     ensure_test_home();
186:     let tenant = TenantScope::new("environment-fork", Uuid::new_v4().to_string()).unwrap();
187:     ui_store::with_tenant_scope(tenant.clone(), async {
188:         let repo = repository();
189:         crate::config::update_global_for_tenant(&tenant, json!({"worktree_settings": {"root": repo.path().join("managed")}})).await.unwrap();
190:         let source = create_session_in(repo.path(), "Source").await;
191:         let (_, boundary) = persist_turn(&source.id, "prompt", "answer").await;
192:         let scope = vw_api_transport::ProjectScope::new(tenant, vw_api_transport::ProjectId::from(source.project_id.as_str())).unwrap();
```

### 宿主诊断24：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:286`。

```text
error[E0425]: cannot find function `with_tenant_instance` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:286:9
    |
286 |         with_tenant_instance(ui_store::current_tenant_scope(), workspace.clone(), move || {
    |         ^^^^^^^^^^^^^^^^^^^^ not found in this scope
    |
note: function `crate::gateway::router::router_tests::handlers::session::stream::stream_tests::with_tenant_instance` exists but is inaccessible
   --> crates/vw-agent/src/gateway/api/handlers/session/shared.rs:16:1
    |
 16 | / pub(super) async fn with_tenant_instance<T>(
 17 | |     tenant: TenantScope,
 18 | |     directory: String,
 19 | |     f: impl FnOnce() -> crate::app::agent::project::BoxFuture<Result<T, ApiError>> + Send + 'static,
...   |
 24 | |     ui_store::with_tenant_scope(tenant, with_instance(directory, f)).await
 25 | | }
    | |_^ not accessible
```

当前源码片段：

```rust
282:         }
283:         assert!(forked.is_user_navigable());
284:         let id = forked.id.clone();
285:         let workspace = forked.directory.clone();
286:         with_tenant_instance(ui_store::current_tenant_scope(), workspace.clone(), move || {
287:             Box::pin(async move {
288:                 for (agent, backend) in
289:                     [("main", None), ("acp-claude", Some("claude")), ("acp-codex", Some("codex"))]
290:                 {
291:                     let (binding, _) = agent_binding::prepare(
```

### 宿主诊断25：E0433

位置：`crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:286`。

```text
error[E0433]: cannot find module or crate `ui_store` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/session/turn_fork_tests.rs:286:30
    |
286 |         with_tenant_instance(ui_store::current_tenant_scope(), workspace.clone(), move || {
    |                              ^^^^^^^^ use of unresolved module or unlinked crate `ui_store`
    |
    = help: if you wanted to use a crate named `ui_store`, use `cargo add ui_store` to add it to your `Cargo.toml`
help: consider importing this module
    |
  1 + use crate::session::ui_store;
    |
```

当前源码片段：

```rust
282:         }
283:         assert!(forked.is_user_navigable());
284:         let id = forked.id.clone();
285:         let workspace = forked.directory.clone();
286:         with_tenant_instance(ui_store::current_tenant_scope(), workspace.clone(), move || {
287:             Box::pin(async move {
288:                 for (agent, backend) in
289:                     [("main", None), ("acp-claude", Some("claude")), ("acp-codex", Some("codex"))]
290:                 {
291:                     let (binding, _) = agent_binding::prepare(
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acceptance_app -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib fork_selection_before_start_survives_reload -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib request -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib worktree_cleanup -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib worktree_environment_fork_waits_for_setup_and_rejects_failure -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
