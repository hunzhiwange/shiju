# 任务116：修复channel-runtime的approval_commands_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：未开始

来源：原任务19全量实际执行；证据编号229, 230, 231, 232。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据229：`runtime_command::approval_commands::approval_commands_tests::approve_tool_persists_config_and_reports_no_exclusion_note`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:523`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:543:5:
    assertion failed: auto_approve.iter().any(|entry| entry == "shell")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据230：`runtime_command::approval_commands::approval_commands_tests::confirm_tool_approval_persists_config_when_runtime_path_exists`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:304`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:326:5:
    assertion failed: auto_approve.iter().any(|entry| entry == "shell")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据231：`runtime_command::approval_commands::approval_commands_tests::request_tool_approval_handles_usage_unknown_already_approved_and_pending`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:195`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:215:5:
    assertion failed: pending.contains("Approval request created.")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据232：`runtime_command::approval_commands::approval_commands_tests::unapprove_tool_removes_persisted_config_and_runtime_state`

位置：`crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:586`；结果：FAIL。

```text
crates/libs/channel-runtime/src/runtime_command/approval_commands_tests.rs:611:5:
    assertion failed: !auto_approve.iter().any(|entry| entry == "shell")
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_commands::approval_commands_tests::approve_tool_persists_config_and_reports_no_exclusion_note' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_commands::approval_commands_tests::confirm_tool_approval_persists_config_when_runtime_path_exists' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_commands::approval_commands_tests::request_tool_approval_handles_usage_unknown_already_approved_and_pending' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p channel-runtime --lib 'runtime_command::approval_commands::approval_commands_tests::unapprove_tool_removes_persisted_config_and_runtime_state' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。
