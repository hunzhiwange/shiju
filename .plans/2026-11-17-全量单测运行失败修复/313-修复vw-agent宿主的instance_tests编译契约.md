# 任务313：修复vw-agent宿主的instance_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号12, 13, 14, 15, 16, 17, 18（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/libs/gateway-instance/src/instance_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断12：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:177`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:177:9
    |
177 |     let Json(info) = path_get::<Host>(Query(query_for(temp.path())), HeaderMap::new())
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
173: 
174: #[tokio::test]
175: async fn path_get_reports_global_and_instance_paths() {
176:     let temp = tempfile::tempdir().expect("tempdir");
177:     let Json(info) = path_get::<Host>(Query(query_for(temp.path())), HeaderMap::new())
178:         .await
179:         .expect("path_get should succeed");
180: 
181:     assert!(!info.home.is_empty());
182:     let home = directories::UserDirs::new().unwrap();
```

### 宿主诊断13：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:196`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:196:9
    |
196 |     let Json(info) = path_get::<Host>(Query(InstanceQuery { directory: None }), headers_for(temp.path()))
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
192: 
193: #[tokio::test]
194: async fn path_get_uses_directory_header_when_query_missing() {
195:     let temp = tempfile::tempdir().expect("tempdir");
196:     let Json(info) = path_get::<Host>(Query(InstanceQuery { directory: None }), headers_for(temp.path()))
197:         .await
198:         .expect("path_get should succeed");
199: 
200:     assert_eq!(info.directory, temp.path().to_string_lossy());
201: }
```

### 宿主诊断14：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:219`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:219:9
    |
219 |     let Json(info) = vcs_get::<Host>(Query(query_for(temp.path())), HeaderMap::new())
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
215: 
216: #[tokio::test]
217: async fn vcs_get_returns_empty_branch_for_non_git_directory() {
218:     let temp = tempfile::tempdir().expect("tempdir");
219:     let Json(info) = vcs_get::<Host>(Query(query_for(temp.path())), HeaderMap::new())
220:         .await
221:         .expect("vcs_get should succeed");
222: 
223:     assert_eq!(info.branch, "");
224: }
```

### 宿主诊断15：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:248`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:248:9
    |
248 |     let Json(info) = vcs_get::<Host>(Query(query_for(temp.path())), HeaderMap::new())
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
244:     repo.set_head("refs/heads/unit-test-branch").expect("checkout branch ref");
245:     repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
246:         .expect("checkout worktree");
247: 
248:     let Json(info) = vcs_get::<Host>(Query(query_for(temp.path())), HeaderMap::new())
249:         .await
250:         .expect("vcs_get should succeed");
251: 
252:     assert_eq!(info.branch, "unit-test-branch");
253: }
```

### 宿主诊断16：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:372`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:372:9
    |
372 |     let Json(disposed) = instance_dispose::<Host>(Query(query_for(temp.path())), HeaderMap::new())
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
368: #[tokio::test]
369: async fn instance_dispose_returns_true_and_clears_cached_instance() {
370:     let temp = tempfile::tempdir().expect("tempdir");
371: 
372:     let Json(disposed) = instance_dispose::<Host>(Query(query_for(temp.path())), HeaderMap::new())
373:         .await
374:         .expect("dispose should succeed");
375: 
376:     assert!(disposed);
377: }
```

### 宿主诊断17：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:396`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:396:9
    |
396 |     let Json(disposed) = instance_dispose::<Host>(Query(query_for(temp.path())), HeaderMap::new())
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
392:     .await
393:     .expect("instance context should be provided")
394:     .expect("approval manager should load");
395: 
396:     let Json(disposed) = instance_dispose::<Host>(Query(query_for(temp.path())), HeaderMap::new())
397:         .await
398:         .expect("dispose should succeed");
399:     assert!(disposed);
400: 
401:     let after = crate::app::agent::project::instance::provide(temp.path(), None, || {
```

### 宿主诊断18：E0531

位置：`crates/libs/gateway-instance/src/instance_tests.rs:431`。

```text
error[E0531]: cannot find tuple struct or tuple variant `Json` in this scope
   --> crates/vw-agent/src/gateway/api/handlers/../../../../../libs/gateway-instance/src/instance_tests.rs:431:9
    |
431 |     let Json(disposed) = instance_dispose::<Host>(Query(query_for(temp.path())), HeaderMap::new())
    |         ^^^^ not found in this scope
```

当前源码片段：

```rust
427:             let _ = tx.send(evt);
428:         }
429:     });
430: 
431:     let Json(disposed) = instance_dispose::<Host>(Query(query_for(temp.path())), HeaderMap::new())
432:         .await
433:         .expect("dispose should succeed");
434:     assert!(disposed);
435: 
436:     let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib instance_dispose_clears_directory_approval_manager -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib instance_dispose_publishes_disposed_event_for_directory -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib instance_dispose_returns_true_and_clears_cached_instance -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib path_get_reports_global_and_instance_paths -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib path_get_uses_directory_header_when_query_missing -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib vcs_get_reports_current_git_branch -- --nocapture
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib vcs_get_returns_empty_branch_for_non_git_directory -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
