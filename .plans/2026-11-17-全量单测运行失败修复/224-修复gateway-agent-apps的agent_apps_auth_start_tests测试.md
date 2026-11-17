# 任务224：修复gateway-agent-apps的agent_apps_auth_start_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号426, 427。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_start_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据426：`host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_commits_expected_scoped_result_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_start_tests.rs:204`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_commits_expected_scoped_result_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_commits_expected_scoped_result_postgres

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 148 filtered out; finished in 0.00s

  stderr ───
    Error: VIBEWINDOW_TEST_POSTGRES_URL is not configured; provide the real local PostgreSQL test URL

    Caused by:
        environment variable not found

```

### 证据427：`host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_rejects_invalid_or_stale_input_postgres`

位置：`crates/libs/gateway-agent-apps/src/host_migration_3_tests/agent_apps_auth_start_tests.rs:276`；结果：FAIL。

```text

  stdout ───

    running 1 test
    test host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_rejects_invalid_or_stale_input_postgres ... FAILED

    failures:

    failures:
        host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_rejects_invalid_or_stale_input_postgres

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
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_commits_expected_scoped_result_postgres' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_3_tests::agent_apps_auth_start_tests::auth6_api_start_rejects_invalid_or_stale_input_postgres' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。


## 本次执行证据（2026-11-17）

- 原两条精确命令在一次 `python3 scripts/cargo_slot.py sh -c` 入口中顺序执行，各运行1项、passed=0、failed=1、退出101；均复现缺少 `VIBEWINDOW_TEST_POSTGRES_URL`，未进入业务断言。日志 `/tmp/shiju-task224/red-start.log`、`/tmp/shiju-task224/red-reject.log`。汇总入口退出0仅用于输出两条退出码，不表示验收通过。
- 槽位1；CARGO_TARGET_DIR、CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。入口和自有子进程已退出，槽位释放、缓存保留。未执行全量测试或额外check。
- 已阅读测试、直接调用者/父模块注册、相邻poll夹具、数据库夹具及底层执行上下文。PostgresTestDatabase创建隔离schema，缺配置直接跳过的宏不符合本任务真实执行标准，未采用。
- 共享依赖与任务219、221、223相同：`app-access-service/src/access.rs::authorize_scoped/execution_context` 忽略请求token、策略与target，固定使用desktop-local-user和gateway_host；`gateway_execution.rs` 从进程运行路径/PATH解析真实主机；`gateway-agent-apps/src/agent_apps_auth_read.rs::router_with_runtime` 忽略basic_auth。本测试Gateway::start写入临时policy中的owner/other/reader及host/remote/offline不再控制实际执行。
- 原断言包括OTHER粘贴返回404、READER启动返回403、remote返回501、offline返回503；现实现无法通过局部夹具调整同时保留上述安全断言。直接提供数据库并运行成功流还会解析本机Claude，而非临时脚本。未使用本机凭据或恢复已废弃策略来冒充隔离验收。
- 需要统一Gateway鉴权改造所有者明确新认证模型下principal/host/细粒度权限的验收标准与隔离执行入口；共享写入依赖上述三个文件，等待所有者处理，不与相邻任务争抢。其后配置真实测试数据库重跑原两条命令。
- 本测试文件及业务代码无本任务diff，只有本编号状态/证据更新，按公共规则不创建专用提交。保留任务文件，未声称完成。


## 2026-11-18 重试与决策边界

- 增加本文件四个入口共用的子进程隔离：HOME、VIBEWINDOW_TEST_ROOT与PATH在子进程初始化前设置，清除CLAUDE_CONFIG_DIR，数据库夹具仍调用from_environment，保留全部原断言。沿用相邻poll测试模式，不改共享授权实现。
- 使用独立真实postgres:17-alpine容器shiju-task224-postgres，仅回环动态端口52360，临时测试库shiju_test；本地显式sslmode=disable。通过一次cargo_slot.py sh -c顺序执行原两条精确命令，槽位1，双缓存均为/Users/xiangmin/worker/code/shiju/target/rust-slot-1。
- 日志/tmp/shiju-task224/retry-start.log和/tmp/shiju-task224/retry-reject.log。两条父测试和对应子进程各运行1项，均passed=0、failed=1、退出101；入口退出1。编译通过，无编译依赖错误；不额外check、不执行全量回归。
- 成功流已通过CREATED、fixture verification_url、pending等断言，证明调用的是临时Claude脚本；随后OTHER粘贴得到202而预期404。拒绝流READER启动得到201而预期403。环境问题已排除，剩余为当前统一身份实现与原安全验收的明确冲突。
- 已向用户集中询问：恢复细粒度授权并保留原断言，还是按统一Gateway身份重新定义验收。原任务明确要求保留安全/租户隔离/精确错误断言，不能单凭移除旧授权的实现自行改变此要求；此为实际产品授权决策，不是编译或环境阻塞。等待答复期间保留已阻塞，不标待回归。
- 自有Cargo入口与子进程均已结束；数据库残余vw_test_模式为0，临时容器已删除，缓存保留。git diff --check仅检查本文件通过；不碰其他任务暂存内容。
- 实际代码改动已提交：`762d15ffe`（test(gateway-agent-apps): 修复授权启动测试的进程环境隔离）。提交仅包含自身测试文件，使用共享提交锁串行执行，其他暂存内容保留；首次遇活动index.lock后等待其退出再成功提交，未删除他人锁。
