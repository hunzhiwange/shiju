# 任务209：修复gateway-agent-apps的apps_usage_acceptance_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号395, 396。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/apps_usage_acceptance_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据395：`apps_usage_refresh_tests::acceptance::usage_macos_api_rejects_invalid_input`

位置：`crates/libs/gateway-agent-apps/src/apps_usage_acceptance_tests.rs:199`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/apps_usage_acceptance_tests.rs:236:129:
    called `Result::unwrap()` on an `Err` value: Decode
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据396：`apps_usage_refresh_tests::acceptance::usage_macos_api_returns_expected_result`

位置：`crates/libs/gateway-agent-apps/src/apps_usage_acceptance_tests.rs:129`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/apps_usage_acceptance_tests.rs:155:45:
    called `Result::unwrap()` on an `Err` value: Decode
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'apps_usage_refresh_tests::acceptance::usage_macos_api_rejects_invalid_input' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'apps_usage_refresh_tests::acceptance::usage_macos_api_returns_expected_result' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-17）

- 当前源码已包含共享提交 `5cb8b43dc`：app-access-service 的 execution_context 忽略策略、目标及旧令牌，固定 principal=desktop-local-user；gateway-app-usage SDK 也忽略 principal_token。此变更与本任务必须保留的 principal/授权隔离断言冲突，不能通过恢复旧认证或删改隔离断言自行解决。
- 已逐条运行任务所列两条精确命令（同阶段通过一次 cargo_slot.py sh -c 入口顺序执行）。槽位3；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- invalid_input：实际1项，passed=0、failed=1，退出101；日志 `/tmp/task209-red-invalid.log`。当前在218行发起刷新返回403。
- expected_result：实际1项，passed=0、failed=1，退出101；日志 `/tmp/task209-red-result.log`。当前在121行发起刷新返回403。原 Decode 位置尚未到达，不能声称已修复。
- 已检查测试、父夹具、SDK读写接口、usage_read 与底层 execution_context。仅设置SDK认证也无法恢复隔离，因为底层明确忽略旧令牌；需统一协调新的身份/隔离契约及相关底层任务归属后继续。未修改业务或测试源码，未运行全量测试，未创建状态专用提交。
- 槽位入口已正常退出（results:101,101），自有Cargo运行已结束，缓存保留；后续槽位复用不属于本任务。

## 2026-11-18 续执行

额外必要底层依赖（已发现其他任务正在修改，未抢写）：`api-apps-usage/src/apps_usage.rs` 的 UsageNumber 解码。网关依赖统一启用 arbitrary_precision，internally tagged enum 中小数使用特殊 Number 映射，需要支持该实际传输格式；不改变线协议或有限数约束。新增网关特性下的独立回归断言。认证契约冲突已向用户提出选择，未擅自恢复旧认证或削弱隔离断言。


续执行验证：
- cargo_slot.py 槽位3，双缓存均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，invalid_input 实际1项失败、退出101，仍在218行403。日志 `/tmp/task209-retry-invalid.log`。第二条Cargo因共享源码漂移重新触发依赖编译，已通过原句柄中断，入口退出130，未声称Cargo验收完成。
- 随后以槽位入口领取槽位1，双缓存均 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，执行该槽位已包含新增用例的原生测试二进制 `debug/deps/gateway_agent_apps-bf5715e2a385139d`，精确过滤 `apps_usage_refresh_tests::acceptance::usage_snapshot_decodes_fractional_numbers_with_gateway_features`：实际1项、passed=1、failed=0、退出0，日志 `/tmp/task209-number.log`。
- 同一二进制精确运行 expected_result：实际1项、passed=0、failed=1、退出101，日志 `/tmp/task209-native-result.log`，121行刷新403。两入口均已结束，未遗留自有构建；保留缓存。
- 安全替代方案已核实：仅重命名夹具 principal 无法满足 OTHER_TOKEN 隔离；SDK发送 skey 也无法改变底层固定 principal；恢复旧鉴权会改变当前已明确移除应用令牌的产品契约。已向用户提出“保持网关认证改测租户/操作归属隔离”或“为usage恢复旧策略隔离”的契约选择。此处确需决策，故保留已阻塞；原安全断言未改动。
- 仅新增数值解码回归用例；底层 UsageNumber 修复由共享工作区其他任务持有。diff 已审查，无扩大权限、无全量测试；原两项行为验收仍未通过。
