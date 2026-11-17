# 任务328：修复vw-agent宿主的task_plan_run_tests编译契约

先阅读并遵守 [公共执行规则](plan.md)及 [宿主补充范围](宿主补充范围.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：继续执行宿主单测；诊断编号42, 43, 44, 45, 46（宿主编译错误证据.json）。
预计会话范围：修复一个源码文件中的宿主接口或测试夹具编译契约；不接管整包。

## 文件与上下文

- 修改/测试：`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs`。
- 宿主编译包：`vw-agent`；库测试通过不能代替该宿主路径编译。
- 阅读当前父模块注册、trait或DTO定义与直接调用者；错误文件已归一化，原始包含路径保存在诊断中。

### 宿主诊断42：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:38`。

```text
error[E0425]: cannot find function `task_plan_run_status` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:38:24
   |
38 |     let Json(status) = task_plan_run_status::<Host>(Extension(owner.clone()), Path(task_id.clone()))
   |                        ^^^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
34:             },
35:         )
36:         .expect("observe planning");
37:
38:     let Json(status) = task_plan_run_status::<Host>(Extension(owner.clone()), Path(task_id.clone()))
39:         .await
40:         .expect("owner status");
41:     assert_eq!(status.status, TaskPlanRunStatus::Running);
42:     assert_eq!(status.current_session_id.as_deref(), Some(session_id.as_str()));
43:
```

### 宿主诊断43：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:45`。

```text
error[E0425]: cannot find function `task_plan_run_status` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:45:9
   |
45 |         task_plan_run_status::<Host>(Extension(outsider.clone()), Path(task_id.clone()))
   |         ^^^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
41:     assert_eq!(status.status, TaskPlanRunStatus::Running);
42:     assert_eq!(status.current_session_id.as_deref(), Some(session_id.as_str()));
43:
44:     let cross_tenant_status =
45:         task_plan_run_status::<Host>(Extension(outsider.clone()), Path(task_id.clone()))
46:             .await
47:             .expect_err("cross-tenant status must be hidden");
48:     assert_eq!(cross_tenant_status.status, StatusCode::NOT_FOUND);
49:     let cross_tenant_stop = task_plan_run_stop::<Host>(Extension(outsider), Path(task_id.clone()))
50:         .await
```

### 宿主诊断44：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:49`。

```text
error[E0425]: cannot find function `task_plan_run_stop` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:49:29
   |
49 |     let cross_tenant_stop = task_plan_run_stop::<Host>(Extension(outsider), Path(task_id.clone()))
   |                             ^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
45:         task_plan_run_status::<Host>(Extension(outsider.clone()), Path(task_id.clone()))
46:             .await
47:             .expect_err("cross-tenant status must be hidden");
48:     assert_eq!(cross_tenant_status.status, StatusCode::NOT_FOUND);
49:     let cross_tenant_stop = task_plan_run_stop::<Host>(Extension(outsider), Path(task_id.clone()))
50:         .await
51:         .expect_err("cross-tenant stop must be hidden");
52:     assert_eq!(cross_tenant_stop.status, StatusCode::NOT_FOUND);
53:     assert!(!cancellation.is_cancelled());
54:
```

### 宿主诊断45：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:55`。

```text
error[E0425]: cannot find function `task_plan_run_stop` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:55:25
   |
55 |     let Json(stopped) = task_plan_run_stop::<Host>(Extension(owner.clone()), Path(task_id.clone()))
   |                         ^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
51:         .expect_err("cross-tenant stop must be hidden");
52:     assert_eq!(cross_tenant_stop.status, StatusCode::NOT_FOUND);
53:     assert!(!cancellation.is_cancelled());
54:
55:     let Json(stopped) = task_plan_run_stop::<Host>(Extension(owner.clone()), Path(task_id.clone()))
56:         .await
57:         .expect("owner stop");
58:     assert_eq!(stopped.status, TaskPlanRunStatus::Cancelled);
59:     assert!(cancellation.is_cancelled());
60:
```

### 宿主诊断46：E0425

位置：`crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:62`。

```text
error[E0425]: cannot find function `task_plan_run_stop` in this scope
  --> crates/vw-agent/src/gateway/api/handlers/task_plan_run_tests.rs:62:9
   |
62 |         task_plan_run_stop::<Host>(Extension(owner), Path(task_id)).await.expect("idempotent stop");
   |         ^^^^^^^^^^^^^^^^^^ not found in this scope
```

当前源码片段：

```rust
58:     assert_eq!(stopped.status, TaskPlanRunStatus::Cancelled);
59:     assert!(cancellation.is_cancelled());
60:
61:     let Json(stopped_again) =
62:         task_plan_run_stop::<Host>(Extension(owner), Path(task_id)).await.expect("idempotent stop");
63:     assert_eq!(stopped_again.status, TaskPlanRunStatus::Cancelled);
64: }
```

## 步骤和验证

1. 重读最新diff和相关接口，先确认其它会话是否已修复；原日志是当前编译红灯，禁止删除测试或弱化安全断言。类型导入或签名迁移沿用现有边界；涉及行为缺陷则先在独立测试文件建立失败断言。
2. 实施最小修复，修改共享业务接口前核查真实调用者；额外公共写入先记录归属并排队。MobileMine分支必须检查现有移动端导航逻辑并接入真实行为，不能用通配分支吞掉消息。
3. 以下名称来自诊断位置最近的真实函数，只作定位过滤候选。运行前用rg核对其是否#[test]/#[tokio::test]入口；若是业务函数/fixture，则用同宿主`cargo test -p vw-agent --lib -- --list`（外层槽位入口）确定真实调用测试并将精确命令记入本编号文件，不能将零项算作通过。

```sh
python3 scripts/cargo_slot.py cargo test -p vw-agent --lib task_plan_run_control_routes_are_tenant_scoped_and_stop_is_idempotent -- --nocapture
```

必要验证：在任务内等待同宿主其余编译阻塞解除后运行与当前修改直接相关的真实测试，要求有效数>0、failed=0、退出0；不将只有no-run或编译成功记为行为通过。生产模块改动同时使用受影响目标check（定向测试已编译同一目标则不重复）；新增必要测试放独立文件。兄弟错误未解除时保留诊断与任务状态，不提前删除编号文件。

编译取证命令（不是测试通过证据）：`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib --no-run`。同阶段顺序命令合并一次入口复用缓存，不默认逐任务全量测试。

最终需由原任务303执行两阶段全量验收；这组修复仅覆盖已发现编译阻塞，尚不能保证宿主全部运行失败已发现。
