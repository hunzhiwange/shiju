# 任务117：修复channel-runtime的approval_config_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号233, 234, 235, 236, 237。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据233：`runtime_command::approval_config::approval_config_tests::persist_adds_auto_approve_and_removes_always_ask_entries`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:148`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:167:5:
    assertion failed: auto_approve.iter().any(|entry| entry == "shell")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据234：`runtime_command::approval_config::approval_config_tests::persist_and_remove_accept_toml_config_payloads`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:230`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:106:37:
    called `Result::unwrap()` on an `Err` value: Error("expected value", line: 2, column: 1)
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据235：`runtime_command::approval_config::approval_config_tests::persist_removes_always_ask_when_tool_is_already_auto_approved`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:181`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:201:5:
    assertion `left == right` failed
      left: Array [String("shell"), String("other")]
     right: Array [String("other")]
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据236：`runtime_command::approval_config::approval_config_tests::persist_supports_leading_whitespace_json_config`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:205`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:225:5:
    assertion failed: auto_approve.iter().any(|entry| entry == "shell")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据237：`runtime_command::approval_config::approval_config_tests::remove_deletes_auto_approve_only_when_present`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:281`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_config_tests.rs:298:5:
    assertion failed: !auto_approve.iter().any(|entry| entry == "shell")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_config::approval_config_tests::persist_adds_auto_approve_and_removes_always_ask_entries' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_config::approval_config_tests::persist_and_remove_accept_toml_config_payloads' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_config::approval_config_tests::persist_removes_always_ask_when_tool_is_already_auto_approved' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_config::approval_config_tests::persist_supports_leading_whitespace_json_config' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_config::approval_config_tests::remove_deletes_auto_approve_only_when_present' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
