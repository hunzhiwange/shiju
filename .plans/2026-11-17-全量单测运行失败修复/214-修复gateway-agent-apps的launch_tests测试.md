# 任务214：修复gateway-agent-apps的launch_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号402, 428。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据402：`host_migration_2_tests::launch_tests::cc_apps_launch_identity_list_sdk_http_tty_lifetime_and_account_rotation`

位置：`crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs:184`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs:408:5:
    assertion `left == right` failed
      left: String("fixture-old-refresh")
     right: "fixture-new-refresh"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据428：`host_migration_2_tests::launch_tests::cc_apps_launch_native_terminal_sdk_http_lifetime_and_rotation`

位置：`crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs:189`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/host_migration_2_tests/launch_tests.rs:408:5:
    assertion `left == right` failed
      left: String("fixture-old-refresh")
     right: "fixture-new-refresh"
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_2_tests::launch_tests::cc_apps_launch_identity_list_sdk_http_tty_lifetime_and_account_rotation' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'host_migration_2_tests::launch_tests::cc_apps_launch_native_terminal_sdk_http_lifetime_and_rotation' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行记录

- 根因：`app-auth-codex/src/rotation.rs::should_adopt_cli_rotation`要求同一 workspace/subject 的可解析 id_token，且 token 变化必须携带严格递增的 last_refresh。旧夹具使用 fixture-id 且无时间戳，因此拒绝写回；参照相邻 account_tests/auth_accounts_tests 补齐两代固定 JWT 和日期。仅修改本测试文件两处凭据，不改业务安全条件。
- 当前检出额外阻塞：`app-access-service/src/access.rs::execution_context`忽略 legacy policy/token/grants，转为全局 gateway_host；`gateway-app-settings/src/agent_apps_settings_api.rs`也不再发送旧应用 token。两个原生命令目前各运行1项、0 passed/1 failed、Cargo退出101，提前失败于264行401断言，未进入CLI轮换。日志：`/tmp/shiju-214-red-identity.log`、`/tmp/shiju-214-red-native.log`。
- 红灯入口槽位9，CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-9`。该轮进程已退出、锁已释放。
- 保留认证/身份权限/租户及子进程断言，不删除旧断言以取得通过。需先明确统一网关认证迁移后的测试契约与执行路径隔离，再回归本文件；本任务不反向恢复已移除的认证体系。
- 修改后两条精确测试分别1项、0 passed/1 failed、Cargo退出101，同样在264行401断言失败。日志：`/tmp/shiju-214-final-identity.log`、`/tmp/shiju-214-final-native.log`。使用cargo_slot.py的run(number=11)原生入口顺序复用缓存，双路径均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-11`；入口已退出，锁释放。未启动CLI子进程，未取得轮换行为通过证据。
- 审查diff：仅launch_tests.rs两行夹具变化，保留全部断言；git diff --check通过。未运行全量测试、桌面构建或额外Cargo check。代码与本记录一并提交，任务保留待回归。
- 提交未完成：git commit --only 指定本任务两文件，多次等待并重试仍退出128，错误为.git/index.lock已存在；未删除共享锁或触碰其他暂存内容。需锁释放后提交本任务差异。
