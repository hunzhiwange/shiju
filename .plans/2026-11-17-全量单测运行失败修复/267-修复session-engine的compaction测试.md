# 任务267：修复session-engine的compaction测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号509, 510, 512, 513。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/session-engine/src/host_migration_23_tests/compaction.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据509：`compaction::host_migration_23_tests::internal_compaction_replaces_covered_history_with_a_model_summary`

位置：`crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:527`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:573:61:
    stored history: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'compaction::host_migration_23_tests::internal_compaction_replaces_covered_history_with_a_model_summary' (126078592) panicked at crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:20:5:
    isolated test failed: internal_compaction_replaces_covered_history_with_a_model_summary: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据510：`compaction::host_migration_23_tests::codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff`

位置：`crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:751`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:783:66:
    called `Result::unwrap()` on an `Err` value: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'compaction::host_migration_23_tests::codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff' (126078549) panicked at crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:20:5:
    isolated test failed: codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据512：`compaction::host_migration_23_tests::prune_skips_unfinished_tool_states`

位置：`crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:474`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:491:30:
    prune should ignore unfinished tools: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'compaction::host_migration_23_tests::prune_skips_unfinished_tool_states' (126078633) panicked at crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:20:5:
    isolated test failed: prune_skips_unfinished_tool_states: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### 证据513：`compaction::host_migration_23_tests::prune_marks_only_old_unprotected_outputs_after_thresholds`

位置：`crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:397`；结果：FAIL。

```text
crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:441:30:
    prune should complete: Storage(Json(Error("invalid type: map, expected f64", line: 0, column: 0)))
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

    thread 'compaction::host_migration_23_tests::prune_marks_only_old_unprotected_outputs_after_thresholds' (126078622) panicked at crates/libs/session-engine/src/host_migration_23_tests/compaction.rs:20:5:
    isolated test failed: prune_marks_only_old_unprotected_outputs_after_thresholds: exit status: 101
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'compaction::host_migration_23_tests::internal_compaction_replaces_covered_history_with_a_model_summary' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'compaction::host_migration_23_tests::codex_skills_replay_real_session_fork_preserves_sources_and_respects_cutoff' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'compaction::host_migration_23_tests::prune_skips_unfinished_tool_states' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p session-engine --lib 'compaction::host_migration_23_tests::prune_marks_only_old_unprotected_outputs_after_thresholds' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
