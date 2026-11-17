# 任务232：修复gateway-app-routing的outbound_probe_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号449。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_probe_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据449：`host_migration_7_tests::outbound_probe_tests::cc_route_029_outbound_probe_expected_behavior`

位置：`crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_probe_tests.rs:91`；结果：FAIL。

```text
crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_probe_tests.rs:147:135:
    called `Result::unwrap()` on an `Err` value: Http { status: 503, body: Some(ApiErrorBody { error: ApiErrorDetail { code: "unavailable", message: "Application routing control is unavailable", details: None } }) }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib 'host_migration_7_tests::outbound_probe_tests::cc_route_029_outbound_probe_expected_behavior' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-18）

- 原生精确命令已执行，日志 `/tmp/shiju-task232-red.log`；退出101，实际1项，0 passed、1 failed、32 filtered out。当前失败提前到第109行：首次请求预期403、实际503，不再是原证据第147行。
- 槽位6；`CARGO_TARGET_DIR`、`CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-6`。包装入口已退出，保留缓存；后续槽位复用不属于本任务。
- 阅读父模块、共享fixture及其现有diff、调用的outbound_probe/lifecycle_api、diagnostic_jobs及底层access/gateway_execution。fixture未准备 `.local/state/shiju`，诊断注册器只创建其私有子目录，原503存在目录准备缺口；但当前授权先于该路径失败，单独补目录不足以恢复用例。
- 当前 `app-access-service::execution_context` 明确忽略传入policy、target及旧应用令牌/grants，使用全局agent_paths和固定 `desktop-local-user`。本用例的首个项目授权403、READER/异租户/异主机403和PEER任务归属404均依赖旧授权模型。不得通过删除断言、将预期改为503、恢复旧令牌授权或更改全局HOME绕过。
- 阻塞依赖：Gateway配对授权迁移需明确并提供可隔离的主体、租户、主机和项目授权测试上下文，以及任务所有者隔离契约，然后才能保留同等安全断言完成本用例。未修改业务或测试文件；共享fixture及其他会话暂存内容未触碰，无本任务代码提交。未运行全量测试、桌面构建或额外check。
- 审查本任务测试文件diff为空；仅更新此编号文件状态，因此按公共规则不创建任务文件专用提交。验证未通过，保留本文件。

## 继续执行证据（2026-11-18）

- 最新工作区重新执行原命令：`/tmp/shiju-task232-retry-red.log`，退出101，实际1项、0 passed/1 failed，首断言503≠403。
- 已修复原503根因：仅本测试创建临时状态根目录，并新增断言确认真实诊断注册器锁文件存在；未改共享fixture、业务授权、原安全断言或全局HOME。
- 修复后同一精确命令：`/tmp/shiju-task232-retry-fixed.log`，退出101，实际1项、0 passed/1 failed。注册器断言通过，第113行首个项目授权断言实际202、预期403。因当前旧grants被忽略，原请求已被接受；这不是编译或外部服务故障。
- 两轮均使用槽位1；双缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，包装入口已退出；未运行全量测试。
- 已排查目录准备、隔离路径及现有泛型RoutingProbe接口：改HOME不能恢复忽略的grants，伪造RoutingProbe授权不能验证真实产品安全行为，恢复旧授权或改403/404断言属于授权契约选择。已请求明确“迁移到当前Gateway配对授权并允许改旧断言”或“保留断言并恢复相应产品授权”。在决定前保留已阻塞；完整用例仍失败，不能标待回归或完成。
- diff仅本用例新增4行和本编号记录；使用git-commit技能提交，保留其它暂存内容。
