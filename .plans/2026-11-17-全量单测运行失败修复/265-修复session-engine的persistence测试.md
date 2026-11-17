# 任务265：修复session-engine的persistence测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号511, 514, 515, 517, 518。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/session-engine/src/host_migration_19_tests/persistence.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据511：`host_migration_19_tests::persistence::agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate`

位置：`crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:230`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:62:6:
    child session should load: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'host_migration_19_tests::persistence::agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate' (126078779) panicked at crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:27:5:
    isolated test failed: agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据514：`host_migration_19_tests::persistence::agcollab_stored_history_and_permissions_are_snapshotted`

位置：`crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:330`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:363:14:
    called `Result::unwrap()` on an `Err` value: invalid type: map, expected f64
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'host_migration_19_tests::persistence::agcollab_stored_history_and_permissions_are_snapshotted' (126078784) panicked at crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:27:5:
    isolated test failed: agcollab_stored_history_and_permissions_are_snapshotted: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据515：`host_migration_19_tests::persistence::child_agent_handoff_research_returns_sources_and_persists_full_stream`

位置：`crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:155`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:62:6:
    child session should load: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'host_migration_19_tests::persistence::child_agent_handoff_research_returns_sources_and_persists_full_stream' (126078802) panicked at crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:27:5:
    isolated test failed: child_agent_handoff_research_returns_sources_and_persists_full_stream: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据517：`host_migration_19_tests::persistence::child_agent_run_creates_persistent_child_and_waits`

位置：`crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:81`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:62:6:
    child session should load: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'host_migration_19_tests::persistence::child_agent_run_creates_persistent_child_and_waits' (126078821) panicked at crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:27:5:
    isolated test failed: child_agent_run_creates_persistent_child_and_waits: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据518：`host_migration_19_tests::persistence::child_agent_failure_persists_child_history`

位置：`crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:200`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:62:6:
    child session should load: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'host_migration_19_tests::persistence::child_agent_failure_persists_child_history' (126078786) panicked at crates/libs/session-engine/src/host_migration_19_tests/persistence.rs:27:5:
    isolated test failed: child_agent_failure_persists_child_history: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'host_migration_19_tests::persistence::agcollab_child_snapshot_is_immutable_and_none_keeps_task_separate' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'host_migration_19_tests::persistence::agcollab_stored_history_and_permissions_are_snapshotted' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'host_migration_19_tests::persistence::child_agent_handoff_research_returns_sources_and_persists_full_stream' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'host_migration_19_tests::persistence::child_agent_run_creates_persistent_child_and_waits' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'host_migration_19_tests::persistence::child_agent_failure_persists_child_history' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
