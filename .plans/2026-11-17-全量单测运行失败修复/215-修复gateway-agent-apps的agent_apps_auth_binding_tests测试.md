# 任务215：修复gateway-agent-apps的agent_apps_auth_binding_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号406, 407。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_binding_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据406：`host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_commits_expected_scoped_result_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_binding_tests.rs:301`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_commits_expected_scoped_result_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_commits_expected_scoped_result_postgres

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据407：`host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_rejects_invalid_or_stale_input_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_binding_tests.rs:307`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_rejects_invalid_or_stale_input_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_rejects_invalid_or_stale_input_postgres

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
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_commits_expected_scoped_result_postgres' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_binding_tests::auth6_api_binding_rejects_invalid_or_stale_input_postgres' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-17）

- 当前分支执行，未改业务代码或断言，无实际实现变更，不创建状态专用提交。
- 原生两条精确命令均执行 1 项，0 passed / 1 failed，退出 101；缺少测试 URL 的红灯日志：`/tmp/shiju-215-commits.log`、`/tmp/shiju-215-rejects.log`。
- 启动独立临时 PostgreSQL 18 容器 `shiju-task215-postgres`，仅绑定 127.0.0.1 随机端口；使用真实数据库 URL 并显式 sslmode=disable。两条原生精确命令复验仍各执行 1 项、0 passed / 1 failed、退出 101。日志：`/tmp/shiju-215-green-commits.log`、`/tmp/shiju-215-green-rejects.log`。
- commits 在第 317 行返回 404 而预期 204；rejects 在第 420 行返回 404 而预期 reauth_required / 409。
- 根因：已提交变更 `5cb8b43dc` 后，`app-access-service/src/access.rs:968-1004` 的 authorize_scoped / execution_context 忽略传入策略、目标、headers 和能力，改用 gateway_host 与固定 desktop-local-user。测试仍按 owner / host / 租户策略创建账户与供应商，真实请求作用域不同，因此资源不可见。夹具中的不同身份、主机、权限拒绝矩阵也与该共享层行为冲突。
- 不能仅改身份或将预期改成 404：会弱化任务要求保留的安全/隔离/精确错误断言。需要先确定当前本地授权设计与本计划旧安全断言的兼容边界，并由共享访问层所有者协调必要修复；本窄任务不擅自回退已提交安全设计。
- 全部命令复用槽位 9，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-9`；通过 scripts/cargo_slot.py 的 run(number=9) 复用红灯缓存。自有测试进程已退出、槽位已释放，临时数据库已清理。未运行全量检查。

## 继续执行证据（2026-11-18）

- 重读当前共享源码与已提交设计，实际用真实 PostgreSQL 18 再次运行两条原生精确命令；各运行 1 项，0 passed / 1 failed，退出均为 101。第一项仍为 404 / 预期 204；第二项仍为 404 / 预期 reauth_required 409。日志 `/tmp/shiju-215-retry-commits.log`、`/tmp/shiju-215-retry-rejects.log`。当前过滤外项数已变化，不能视作固定提交回归。
- 使用 scripts/cargo_slot.py 的 run(number=9)，两缓存均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-9`。自有入口已退出并释放槽位；自有临时容器 shiju-task215-retry 已删除。没有编译失败，无需修改依赖。
- 进一步核实：25fb3fc22 明确将独立 token 与权限清单改接 Gateway；5cb8b43dc 明确移除主机策略，固定本机上下文。因此该冲突是已提交产品契约与本任务旧权限断言不兼容，不只是环境缺失。上次将全部根因归于 5cb8b43dc 不够精确，身份与权限变更先发生于 25fb3fc22。
- 已排查替代路径：只改夹具账户/主机仍无法保留 EDITOR / READER / 缺少应用 token 的拒绝矩阵；测试专用授权注入会伪造已被移除的业务行为；恢复显式 policy 路径仍会重新引入已移除的重复授权；直接更新断言则违反本任务保留权限与精确错误的约束。
- 当前确实需要验收契约决策：沿用 Gateway skey / 本机身份并授权更新旧权限断言，或恢复应用层角色/身份/主机校验。已提出该二选一问题，未收到答复，不以等待超时视为授权。保持已阻塞；没有实际代码修改，不创建编号状态专用提交。
