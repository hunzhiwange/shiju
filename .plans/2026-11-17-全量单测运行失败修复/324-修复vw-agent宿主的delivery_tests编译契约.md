# 任务324：修复vw-agent宿主的delivery_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号9, 10, 40（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/cron/delivery_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断9：E0425

位置：`crates/vw-agent/src/cron/delivery_tests.rs:9`。

```text
error[E0425]: cannot find type `Config` in this scope
 --> crates/vw-agent/src/cron/delivery_tests.rs:9:40
  |
9 | fn fixture(tmp: &tempfile::TempDir) -> Config {
  |                                        ^^^^^^ not found in this scope
```

当前源码片段：

```rust
5: use serde_json::{Value, json};
6: use tokio::time::{Duration, timeout};
7: use vw_api_transport::id::TenantScope;
8:
9: fn fixture(tmp: &tempfile::TempDir) -> Config {
10:     let mut config = Config::default();
11:     config.workspace_dir = tmp.path().join("workspace");
12:     config.config_path = tmp.path().join("config.json");
13:     std::fs::create_dir_all(&config.workspace_dir).unwrap();
14:     config.channels_config.wecom =
```

### 宿主诊断10：E0425

位置：`crates/vw-agent/src/cron/delivery_tests.rs:24`。

```text
error[E0425]: cannot find type `Config` in this scope
  --> crates/vw-agent/src/cron/delivery_tests.rs:24:24
   |
24 | async fn save(config: &Config) {
   |                        ^^^^^^ not found in this scope
```

当前源码片段：

```rust
20: fn tenant() -> TenantScope {
21:     TenantScope::new("cron-wecom-test", uuid::Uuid::new_v4().to_string()).unwrap()
22: }
23:
24: async fn save(config: &Config) {
25:     crate::config::schema::save_config_for_tenant(
26:         config,
27:         &crate::session::ui_store::current_tenant_scope(),
28:     )
29:     .await
```

### 宿主诊断40：E0433

位置：`crates/vw-agent/src/cron/delivery_tests.rs:10`。

```text
error[E0433]: cannot find type `Config` in this scope
  --> crates/vw-agent/src/cron/delivery_tests.rs:10:22
   |
10 |     let mut config = Config::default();
   |                      ^^^^^^ use of undeclared type `Config`
```

当前源码片段：

```rust
6: use tokio::time::{Duration, timeout};
7: use vw_api_transport::id::TenantScope;
8:
9: fn fixture(tmp: &tempfile::TempDir) -> Config {
10:     let mut config = Config::default();
11:     config.workspace_dir = tmp.path().join("workspace");
12:     config.config_path = tmp.path().join("config.json");
13:     std::fs::create_dir_all(&config.workspace_dir).unwrap();
14:     config.channels_config.wecom =
15:         Some(serde_json::from_value(json!({"bot_id":"bot","secret":"secret"})).unwrap());
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib fixture -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib save -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
