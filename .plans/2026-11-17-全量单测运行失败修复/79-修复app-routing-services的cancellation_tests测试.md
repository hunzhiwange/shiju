# 任务79：修复app-routing-services的cancellation_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号157, 160。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/app-routing-services/src/cancellation_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据157：`cancellation_tests::cc_route_022_sdk_cancel_and_credential_revocation_release_each_request_once`

位置：`crates/libs/app-routing-services/src/cancellation_tests.rs:182`；结果：FAIL。

```text
crates/libs/app-routing-services/src/cancellation_tests.rs:141:13:
    assertion `left != right` failed
      left: 0
     right: 0
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据160：`cancellation_tests::cc_route_022_cancellation_expected_behavior`

位置：`crates/libs/app-routing-services/src/cancellation_tests.rs:150`；结果：FAIL。

```text
crates/libs/app-routing-services/src/cancellation_tests.rs:141:13:
    assertion `left != right` failed
      left: 0
     right: 0
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'cancellation_tests::cc_route_022_sdk_cancel_and_credential_revocation_release_each_request_once' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'cancellation_tests::cc_route_022_cancellation_expected_behavior' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行记录

- 根因：测试 OUTPUT 的 Chat SSE choices 缺少 index；共享 stream_observer::chat 强制要求非负 index，因而在断连测试观察输出前拒绝事件。仅补齐 index:0，保留所有撤权、取消、许可计数与防重放断言，不修改业务校验。
- 原生命令修复前执行均被 rmcp 0.8.5 编译错误阻断（base64/schemars/paste 未解析、handler::server 被特性裁剪，13 个错误），实际测试数 0，不能认定运行红灯或通过。
- 槽位 10；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-10；原始日志 /tmp/shiju-task79/red-sdk.log、red-behavior.log。
- 无共享业务文件写入；保留现有工作区其他任务修改。后续需解决独立依赖编译问题后重跑两项精确命令，达到各 1 passed、0 failed 后再删除任务文件。
- 修复后仍使用同样两条精确过滤命令，槽位 6；双缓存路径均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-6；日志 /tmp/shiju-task79/post-sdk.log、post-behavior.log。依赖错误未消除，未取得测试通过证据。
- 轻量检查：git diff --check 通过；审查仅修改 OUTPUT 一行。未运行整包、全量、桌面编译或重复 check。
- 修复后两条命令退出码均 101，实际执行测试数 0；槽位 6、10 的自有锁均已释放。
