# 任务229：修复gateway-app-routing的agent_apps_high_availability_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号437, 438, 442, 445。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据437：`host_migration_14_tests::agent_apps_high_availability_tests::ha_api_health_and_policy_are_scope_bound`

位置：`crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:494`；结果：FAIL。

```text
crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:537:9:
    assertion `left == right` failed: health
      left: 404
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据438：`host_migration_14_tests::agent_apps_high_availability_tests::ha_api_queue_write_rejects_conflicts`

位置：`crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:135`；结果：FAIL。

```text
crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:230:5:
    assertion `left == right` failed
      left: 404
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据442：`host_migration_14_tests::agent_apps_high_availability_tests::ha_api_queue_write_rejects_conflicts_enabled`

位置：`crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:354`；结果：FAIL。

```text
crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:441:5:
    assertion `left == right` failed
      left: 503
     right: 409
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据445：`host_migration_14_tests::agent_apps_high_availability_tests::ha_api_queue_read_enforces_scope`

位置：`crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:20`；结果：FAIL。

```text
crates/libs/gateway-app-routing/src/host_migration_14_tests/agent_apps_high_availability_tests.rs:128:9:
    assertion `left == right` failed
      left: Object {"scope": Object {"project": Object {"platform_id": String("p"), "company_id": String("owner"), "project_id": String("project")}, "targetHostId": String("routing-mac"), "appId": String("codex")}, "version": Number(0), "enabled": Bool(false), "entries": Array []}
     right: Object {"scope": Object {"project": Object {"platform_id": String("p"), "company_id": String("owner"), "project_id": String("project")}, "targetHostId": String("routing-mac"), "appId": String("codex")}, "version": Number(1), "enabled": Bool(true), "entries": Array [Object {"providerId": String("queued-b"), "name": String("Custom")}, Object {"providerId": String("queued-a"), "name": String("Custom")}]}
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib 'host_migration_14_tests::agent_apps_high_availability_tests::ha_api_health_and_policy_are_scope_bound' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib 'host_migration_14_tests::agent_apps_high_availability_tests::ha_api_queue_write_rejects_conflicts' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib 'host_migration_14_tests::agent_apps_high_availability_tests::ha_api_queue_write_rejects_conflicts_enabled' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib 'host_migration_14_tests::agent_apps_high_availability_tests::ha_api_queue_read_enforces_scope' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 执行记录

- 共享依赖：任务229写入 `host_migration_7_tests/fixture.rs` 的数据库注入与 `server/agent_apps_high_availability.rs` 的策略来源；写入前两文件无外部差异。任务228的验收夹具独立，不覆盖其改动。
- 当前红灯：槽位1，四条精确命令均执行1项、失败1项、退出101；日志 `/tmp/shiju-task229/red-*.log`。双缓存均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 外部授权迁移并发移除了本测试认证头，当前访问层明确忽略旧令牌/grants；保留其外部修改，不削弱原认证与隔离断言。

- 已修复：共享夹具显式注入临时数据库；HA enabled 使用 RoutingControl 的策略路径；启用测试改用已注册的项目路由 mode 入口。按用户回复恢复本文件令牌头，保留原认证/隔离断言。
- 最终验证：原四条精确命令每条实际运行1项，passed=0、failed=1、退出101；日志 `/tmp/shiju-task229/final-<测试名>.log`，摘要 `/tmp/shiju-task229/final-summary.log`。沿用槽位1，两个缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；入口及子进程已退出，槽位释放。diff --check 通过。无参数化子进程用例。
- 未完成原因：并发授权迁移的 `app-access-service/src/access.rs:execution_context` 明确忽略旧 token/grants，并由 `gateway_execution.rs:gateway_host` 使用进程级 home/root，绕开本测试临时策略的执行目录。读取无令牌返回200（预期401）、写入无令牌返回200（预期401）、health跨host拒绝断言失败；enabled进入实际模式事务但因执行根不匹配返回409 revision_conflict（预期200）。恢复旧请求头后仍复现，非本次接线修复引入。不能削弱断言或恢复整个授权架构以冒充通过。
- 后续：待授权迁移所有者提供统一 pairing 中间件与临时执行目录注入方案，再按原四命令回归。本任务保持待回归且保留编号文件。
