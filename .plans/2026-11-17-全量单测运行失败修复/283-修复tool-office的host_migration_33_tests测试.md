# 任务283：修复tool-office的host_migration_33_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号549, 550, 551, 552, 553, 554, 555, 567。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/tool-office/src/host_migration_33_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据549：`host_migration_33_tests::cc_control_14_foreign_executor_owner_cannot_discover_or_execute`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:253`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:274:6:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Excel control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据550：`host_migration_33_tests::cc_control_14_no_session_discovers_empty_and_refuses_execution`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:156`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:158:62:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Excel control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据551：`host_migration_33_tests::cc_control_14_document_approval_is_scoped_and_explicit_deny_is_preserved`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:365`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:380:10:
    called `Result::unwrap()` on an `Err` value: Excel control denied: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据552：`host_migration_33_tests::cc_control_14_malformed_readback_is_failure_and_closes_connection`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:339`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:348:5:
    assertion failed: f.call(request).await.unwrap_err().to_string().contains("invalid Excel readback")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据553：`host_migration_33_tests::cc_control_14_revocation_during_command_discards_result`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:278`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:290:5:
    assertion failed: f.call(request).await.unwrap_err().to_string().contains("StalePermission")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据554：`host_migration_33_tests::cc_control_14_duplicate_titles_require_exact_document_and_generation`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:162`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:166:68:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Excel control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据555：`host_migration_33_tests::cc_control_14_write_returns_actual_readback_to_model`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:212`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:228:14:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Excel control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据567：`host_migration_33_tests::cc_control_14_abort_invalidates_connection_and_discards_late_result`

位置：`crates/libs/tool-office/src/host_migration_33_tests.rs:320`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_33_tests.rs:146:10:
    called `Result::unwrap()` on an `Err` value: Elapsed(())
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_foreign_executor_owner_cannot_discover_or_execute' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_no_session_discovers_empty_and_refuses_execution' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_document_approval_is_scoped_and_explicit_deny_is_preserved' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_malformed_readback_is_failure_and_closes_connection' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_revocation_during_command_discards_result' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_duplicate_titles_require_exact_document_and_generation' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_write_returns_actual_readback_to_model' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_33_tests::cc_control_14_abort_invalidates_connection_and_discards_late_result' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
