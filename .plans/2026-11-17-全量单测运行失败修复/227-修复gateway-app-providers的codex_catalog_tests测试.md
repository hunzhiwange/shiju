# 任务227：修复gateway-app-providers的codex_catalog_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号431, 432, 433, 434, 435, 436。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据431：`host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_missing_write_grant_and_managed_rows_are_rejected`

位置：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:315`；结果：FAIL。

```text
crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:336:10:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Null }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据432：`host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_auth_stash_preservation_and_failure`

位置：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:221`；结果：FAIL。

```text
crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:229:68:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Null }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据433：`host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_commits_saved_directory_and_scoped_pointer`

位置：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:105`；结果：FAIL。

```text
crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:111:10:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Null }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据434：`host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_recovers_marked_catalog_and_pointer_before_replanning`

位置：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:258`；结果：FAIL。

```text
crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:266:10:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Null }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据435：`host_migration_6_tests::codex_catalog_tests::cc04_ui_codex_sdk_edits_mapping_and_preserves_inactive_fields_through_native_transaction`

位置：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:354`；结果：FAIL。

```text
crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:363:61:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Null }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据436：`host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_denial_and_profile_failure_keep_native_files`

位置：`crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:144`；结果：FAIL。

```text
crates/libs/gateway-app-providers/src/host_migration_6_tests/codex_catalog_tests.rs:162:10:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Null }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib 'host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_missing_write_grant_and_managed_rows_are_rejected' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib 'host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_auth_stash_preservation_and_failure' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib 'host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_commits_saved_directory_and_scoped_pointer' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib 'host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_recovers_marked_catalog_and_pointer_before_replanning' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib 'host_migration_6_tests::codex_catalog_tests::cc04_ui_codex_sdk_edits_mapping_and_preserves_inactive_fields_through_native_transaction' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-providers --lib 'host_migration_6_tests::codex_catalog_tests::cc04_codex_catalog_sdk_api_denial_and_profile_failure_keep_native_files' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
