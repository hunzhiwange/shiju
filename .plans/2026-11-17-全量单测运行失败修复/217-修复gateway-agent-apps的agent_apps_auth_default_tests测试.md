# 任务217：修复gateway-agent-apps的agent_apps_auth_default_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号410, 412。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_default_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据410：`host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_commits_expected_scoped_result_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_default_tests.rs:327`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_commits_expected_scoped_result_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_commits_expected_scoped_result_postgres

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据412：`host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_rejects_invalid_or_stale_input_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_default_tests.rs:335`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_rejects_invalid_or_stale_input_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_rejects_invalid_or_stale_input_postgres

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.00s

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
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_commits_expected_scoped_result_postgres' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_default_tests::auth6_api_default_rejects_invalid_or_stale_input_postgres' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 2026-11-17 执行证据与阻塞

- 原生两条精确命令均运行1项、失败1项、退出101；日志 `/tmp/shiju-217-red-commits.log`、`/tmp/shiju-217-red-rejects.log`。槽位9，双缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-9`。
- 使用已有 postgres:18 镜像创建自有临时容器 shiju-task217-postgres，端口仅绑定127.0.0.1:62734；测试URL显式指定 sslmode=disable。无缺环境跳过、无断言弱化。
- 配置真实数据库后的 commits 精确测试运行1项、失败1项、退出101：neighbor/foreign-company 返回404，预期204；槽位6，双缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`；日志 `/tmp/shiju-217-green-commits.log`（文件名不代表通过）。rejects 精确测试运行1项、失败1项、退出101：reader 请求返回404，预期403；槽位9；日志 `/tmp/shiju-217-green-rejects.log`。
- 根因：`app-access-service/src/access.rs` 的 authorize_scoped/execution_context 明确忽略 legacy app tokens/grants/access.json，使用 gateway_host 与固定 desktop-local-user，且 can_set_default=true；`gateway-agent-apps/src/agent_apps_auth_read.rs` 的 router_with_runtime 也忽略传入 basic_auth。旧测试的 owner/reader、主机及旧授权边界与当前业务实现冲突，不能恢复旧策略或删除隔离断言来取得通过。
- 需要明确新的配对认证下 owner/reader/host/tenant 权限验收标准，再协调上述共享授权文件所有者修复。本任务不越权修改共享认证链。测试源码临时诊断已还原，无实际代码变更，不创建空提交；保留本任务文件。

- SQLite 对照 commits 同样精确运行1项失败1项、退出101、404≠204，日志 `/tmp/shiju-217-sqlite.log`，证明不是 PostgreSQL 特有问题。所有自有验证进程已退出，槽位由入口释放，临时容器已停止并自动删除。

## 2026-11-18 当前工作区重试

- 已重读最新授权实现，确认并非暂时的共享变更：提交 25fb3fc22 明确将认证迁移至网关 skey，且注明旧权限测试尚未验证；后续 5cb8b43dc 明确删除重复连接表单和应用令牌前置校验。当前 authorize_scoped 继续忽略 headers/path/target，固定 desktop-local-user；路由忽略传入 basic_auth。
- 已使用真实独立 PostgreSQL 18 重试两项原生精确命令，均实际运行1项、0通过1失败、退出101，149项过滤；commits 仍404≠204，rejects仍404≠403。日志 `/tmp/shiju-217-retry-commits.log`、`/tmp/shiju-217-retry-rejects.log`。槽位6，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。编译成功，没有需要本任务修复的编译依赖错误。
- 自有容器 shiju-task217-retry 只绑定127.0.0.1:57373，已停止自动移除；测试模式残留检查为0。入口已完成，退出码汇总101,101，释放自有槽位。
- 已排查安全替代方案：恢复旧 grants 改变有意迁移的认证策略；按新网关契约重写测试需要替换 reader拒绝、离线/远程主机及旧basic认证断言，与本任务明确保留这些断言的要求冲突；仅改存储测试会漏掉网关鉴权边界。无安全的机械修复。
- 已向用户提出必须的契约选择：按当前网关配对认证迁移测试并保留适用隔离验证，或恢复旧owner/reader契约。选择尚未收到，保持已阻塞。无实际代码变更、无提交，仅更新此编号证据文件；不创建任务状态专用提交。
