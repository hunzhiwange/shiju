# 任务271：修复session-store的postgres_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号525, 526, 527, 528, 529, 530。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/session-store/src/store/postgres_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据525：`store::postgres::postgres_tests::dynamic_tools_persistence_failed_write_keeps_running_catalog`

位置：`crates/libs/session-store/src/store/postgres_tests.rs:150`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test store::postgres::postgres_tests::dynamic_tools_persistence_failed_write_keeps_running_catalog ... FAILED

    failures:

    failures:
        store::postgres::postgres_tests::dynamic_tools_persistence_failed_write_keeps_running_catalog

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据526：`store::postgres::postgres_tests::dynamic_tools_persistence_concurrent_replacements_follow_commit_order`

位置：`crates/libs/session-store/src/store/postgres_tests.rs:180`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test store::postgres::postgres_tests::dynamic_tools_persistence_concurrent_replacements_follow_commit_order ... FAILED

    failures:

    failures:
        store::postgres::postgres_tests::dynamic_tools_persistence_concurrent_replacements_follow_commit_order

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据527：`store::postgres::postgres_tests::dynamic_tools_persistence_native_index_preserves_existing_ui`

位置：`crates/libs/session-store/src/store/postgres_tests.rs:187`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test store::postgres::postgres_tests::dynamic_tools_persistence_native_index_preserves_existing_ui ... FAILED

    failures:

    failures:
        store::postgres::postgres_tests::dynamic_tools_persistence_native_index_preserves_existing_ui

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据528：`store::postgres::postgres_tests::dynamic_tools_persistence_rejects_untrusted_and_invalid`

位置：`crates/libs/session-store/src/store/postgres_tests.rs:164`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test store::postgres::postgres_tests::dynamic_tools_persistence_rejects_untrusted_and_invalid ... FAILED

    failures:

    failures:
        store::postgres::postgres_tests::dynamic_tools_persistence_rejects_untrusted_and_invalid

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据529：`store::postgres::postgres_tests::dynamic_tools_persistence_scopes_and_stale_versions_are_isolated`

位置：`crates/libs/session-store/src/store/postgres_tests.rs:173`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test store::postgres::postgres_tests::dynamic_tools_persistence_scopes_and_stale_versions_are_isolated ... FAILED

    failures:

    failures:
        store::postgres::postgres_tests::dynamic_tools_persistence_scopes_and_stale_versions_are_isolated

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据530：`store::postgres::postgres_tests::dynamic_tools_persistence_success_and_explicit_empty`

位置：`crates/libs/session-store/src/store/postgres_tests.rs:157`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test store::postgres::postgres_tests::dynamic_tools_persistence_success_and_explicit_empty ... FAILED

    failures:

    failures:
        store::postgres::postgres_tests::dynamic_tools_persistence_success_and_explicit_empty

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 87 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p session-store --lib 'store::postgres::postgres_tests::dynamic_tools_persistence_failed_write_keeps_running_catalog' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-store --lib 'store::postgres::postgres_tests::dynamic_tools_persistence_concurrent_replacements_follow_commit_order' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-store --lib 'store::postgres::postgres_tests::dynamic_tools_persistence_native_index_preserves_existing_ui' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-store --lib 'store::postgres::postgres_tests::dynamic_tools_persistence_rejects_untrusted_and_invalid' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-store --lib 'store::postgres::postgres_tests::dynamic_tools_persistence_scopes_and_stale_versions_are_isolated' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-store --lib 'store::postgres::postgres_tests::dynamic_tools_persistence_success_and_explicit_empty' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
