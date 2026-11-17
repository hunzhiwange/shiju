# 任务137：修复computer-control的lifecycle_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号265, 266, 267, 268, 269, 270, 271, 272。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/computer-control/src/lifecycle_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据265：`host::lifecycle_tests::cc_control_20_repeated_stop_is_idempotent`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:38`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据266：`host::lifecycle_tests::cc_control_20_revoked_permission_still_allows_authenticated_stop`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:56`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据267：`host::lifecycle_tests::cc_control_20_stop_cancels_attached_session`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:30`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据268：`host::lifecycle_tests::cc_control_20_stop_invalidates_ticket`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:23`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据269：`host::lifecycle_tests::cc_control_20_stop_preserves_other_session`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:46`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据270：`host::lifecycle_tests::cc_control_20_stop_rejects_replayed_sequence`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:78`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据271：`host::lifecycle_tests::cc_control_20_stop_rejects_cross_session_identity`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:64`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据272：`host::lifecycle_tests::cc_control_20_stopped_session_rejects_queued_action`

位置：`crates/libs/computer-control/src/lifecycle_tests.rs:71`；结果：FAIL。

```text
crates/libs/computer-control/src/lifecycle_tests.rs:13:70:
    called `Result::unwrap()` on an `Err` value: PermissionDenied
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_repeated_stop_is_idempotent' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_revoked_permission_still_allows_authenticated_stop' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_stop_cancels_attached_session' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_stop_invalidates_ticket' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_stop_preserves_other_session' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_stop_rejects_replayed_sequence' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_stop_rejects_cross_session_identity' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p computer-control --lib 'host::lifecycle_tests::cc_control_20_stopped_session_rejects_queued_action' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
