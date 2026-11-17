# 任务83：修复app-routing-services的agent_apps_failover_timeouts_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号162, 167。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_timeouts_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据162：`host_migration_13_tests::agent_apps_failover_timeouts_tests::ha_first_byte_timeout_stops_silent_upstream`

位置：`crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_timeouts_tests.rs:33`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_13_tests::agent_apps_failover_timeouts_tests::ha_first_byte_timeout_stops_silent_upstream ... FAILED

    failures:

    failures:
        host_migration_13_tests::agent_apps_failover_timeouts_tests::ha_first_byte_timeout_stops_silent_upstream

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 99 filtered out; finished in 10.25s

  stderr ───
    Error: deadline has elapsed

```

### 证据167：`host_migration_13_tests::agent_apps_failover_timeouts_tests::ha_idle_and_total_deadlines_bound_stream`

位置：`crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_timeouts_tests.rs:38`；结果：FAIL。

```text
crates/libs/app-routing-services/src/host_migration_13_tests/agent_apps_failover_timeouts_tests.rs:285:26:
    stream must terminate within idle or original total deadline: Elapsed(())
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'host_migration_13_tests::agent_apps_failover_timeouts_tests::ha_first_byte_timeout_stops_silent_upstream' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'host_migration_13_tests::agent_apps_failover_timeouts_tests::ha_idle_and_total_deadlines_bound_stream' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据

- 根因：夹具先保存显式 HA policy（默认首字节/idle 超时），后仅修改普通 AppTimeouts。`failover_runtime::policy` 优先返回显式保存策略，因此预期的 1 秒超时未生效。将同一个 AppTimeouts 写入 HA retry_timeout 和普通配置；保留全部真实 HTTP、无重放、队列、熔断和关闭断言，业务实现未修改。
- 修改前两条原生精确命令均因 rmcp 0.8.5 编译失败，退出 101，实际测试数 0；日志 `/tmp/task83-first-red.log`、`/tmp/task83-idle-red.log`。未取得当前运行红灯，不将历史失败冒充本次执行。
- 修改后按原命令复验，日志 `/tmp/task83-first-after.log`、`/tmp/task83-idle-after.log`；相同 rmcp 缺失 base64/schemars/paste 和 capabilities builder 类型错误阻断，未能验收。此共享依赖问题超出本测试文件归属，未修改包清单、Cargo.lock 或 registry。
- 两阶段均使用 cargo_slot.py，实际槽位 3；CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。日志在 /tmp 保存，缓存保留。
- 已审查本文件 diff；git diff --check 通过。共享工作区存在其他任务改动，本任务只提交自身测试夹具与本编号记录。
- 下一步：共享 rmcp 编译问题修复后，重新执行本文件两条精确命令，要求每条 1 passed、0 failed 后才能完成删除编号文件。
