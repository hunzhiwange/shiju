# 任务219：修复gateway-agent-apps的agent_apps_auth_logout_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号415, 416, 418。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_logout_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据415：`host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_logout_tests.rs:583`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result_postgres

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据416：`host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_rejects_invalid_or_stale_input_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_logout_tests.rs:592`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_rejects_invalid_or_stale_input_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_rejects_invalid_or_stale_input_postgres

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据418：`host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_logout_tests.rs:569`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_logout_tests.rs:264:5:
    assertion `left == right` failed: expected reauth_required
      left: 400
     right: 409
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result_postgres' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_rejects_invalid_or_stale_input_postgres' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_logout_tests::auth6_api_logout_commits_expected_scoped_result' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-17）

- 三条精确命令在 cargo_slot.py 同一 sh 入口顺序执行；槽位1，双缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。每条实际执行1项、failed=1、退出101。
- PostgreSQL 两条仍缺少测试URL；日志 `/tmp/shiju-219-red-auth6_api_logout_commits_expected_scoped_result_postgres.log`、`/tmp/shiju-219-red-auth6_api_logout_rejects_invalid_or_stale_input_postgres.log`。
- SQLite 当前失败前移至286行：设置默认账户404而非204；日志 `/tmp/shiju-219-red-auth6_api_logout_commits_expected_scoped_result.log`。补充原生诊断亦执行1项、失败1项、退出101，正文 code=not_found（`/tmp/shiju-219-diagnostic.log`）。诊断改动已撤回。
- 阻塞依赖：`app-access-service/src/access.rs` 的 authorize_scoped/execution_context 当前忽略策略路径、headers与目标主机，固定 gateway_host、desktop-local-user；`agent_apps_auth_read.rs` 同样忽略 Basic Auth 参数。原夹具 owner/host/credential_root 不再参与解析，当前隔离权限断言无法由旧夹具表达。需共享鉴权变更所有者明确兼容接线或隔离测试入口；不能弱化权限断言或擅自恢复旧鉴权。邻近任务217亦有同类404。
- 已启动自有 postgres:18 容器 shiju-task219-postgres（仅127.0.0.1动态端口），随后清理。由于当前鉴权会解析真实本机环境，未继续执行可能写入原生凭据的注销请求。验收未完成，历史DSH 400尚未触达。
- 自有Cargo入口均已结束，槽位已释放，缓存保留。测试文件无最终改动，仅编号状态及证据更新，不创建专用提交。未运行全量测试。
