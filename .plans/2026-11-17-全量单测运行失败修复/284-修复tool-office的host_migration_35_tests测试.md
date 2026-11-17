# 任务284：修复tool-office的host_migration_35_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号556, 557, 558, 559, 560, 561, 562, 568, 569。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/tool-office/src/host_migration_35_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据556：`host_migration_35_tests::cc_control_16_foreign_executor_owner_cannot_discover_or_execute`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:246`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:267:6:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Word control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据557：`host_migration_35_tests::cc_control_16_duplicate_titles_require_exact_document_and_generation`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:162`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:166:68:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Word control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据558：`host_migration_35_tests::cc_control_16_document_approval_is_scoped_and_explicit_deny_is_preserved`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:356`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:371:10:
    called `Result::unwrap()` on an `Err` value: Word control denied: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据559：`host_migration_35_tests::cc_control_16_malformed_readback_is_failure_and_closes_connection`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:330`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:339:5:
    assertion failed: f.call(request).await.unwrap_err().to_string().contains("invalid Word readback")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据560：`host_migration_35_tests::cc_control_16_no_session_discovers_empty_and_refuses_execution`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:156`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:158:62:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Word control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据561：`host_migration_35_tests::cc_control_16_replace_returns_actual_changed_range`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:212`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:223:48:
    called `Result::unwrap()` on an `Err` value: Denied { message: "Word control denied: PermissionDenied", permission_request: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据562：`host_migration_35_tests::cc_control_16_revocation_during_command_discards_result`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:271`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:283:5:
    assertion failed: f.call(request).await.unwrap_err().to_string().contains("StalePermission")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据568：`host_migration_35_tests::cc_control_16_abort_invalidates_connection_and_discards_late_result`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:313`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:146:10:
    called `Result::unwrap()` on an `Err` value: Elapsed(())
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据569：`host_migration_35_tests::cc_control_16_range_conflict_is_failure_and_invalidates_session`

位置：`crates/libs/tool-office/src/host_migration_35_tests.rs:298`；结果：FAIL。

```text
crates/libs/tool-office/src/host_migration_35_tests.rs:146:10:
    called `Result::unwrap()` on an `Err` value: Elapsed(())
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'host_migration_35_tests::cc_control_16_range_conflict_is_failure_and_invalidates_session' (126105318) panicked at crates/libs/tool-office/src/host_migration_35_tests.rs:308:21:
    called `Result::unwrap()` on an `Err` value: JoinError::Panic(Id(3), "called `Result::unwrap()` on an `Err` value: Elapsed(())", ...)

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_foreign_executor_owner_cannot_discover_or_execute' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_duplicate_titles_require_exact_document_and_generation' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_document_approval_is_scoped_and_explicit_deny_is_preserved' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_malformed_readback_is_failure_and_closes_connection' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_no_session_discovers_empty_and_refuses_execution' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_replace_returns_actual_changed_range' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_revocation_during_command_discards_result' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_abort_invalidates_connection_and_discards_late_result' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p tool-office --lib 'host_migration_35_tests::cc_control_16_range_conflict_is_failure_and_invalidates_session' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
