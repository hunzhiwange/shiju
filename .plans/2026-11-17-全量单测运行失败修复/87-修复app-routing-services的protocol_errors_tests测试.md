# 任务87：修复app-routing-services的protocol_errors_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号165, 166。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/app-routing-services/src/protocol_errors_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据165：`protocol_errors_tests::cc_route_024_sse_error_before_output_uses_http_and_done_ends_the_stream`

位置：`crates/libs/app-routing-services/src/protocol_errors_tests.rs:181`；结果：FAIL。

```text
crates/libs/app-routing-services/src/protocol_errors_tests.rs:206:5:
    assertion `left == right` failed
      left: 502
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据166：`protocol_errors_tests::cc_route_024_started_sse_ends_with_a_sanitized_error_frame`

位置：`crates/libs/app-routing-services/src/protocol_errors_tests.rs:121`；结果：FAIL。

```text
crates/libs/app-routing-services/src/protocol_errors_tests.rs:134:9:
    assertion `left == right` failed
      left: 502
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'protocol_errors_tests::cc_route_024_sse_error_before_output_uses_http_and_done_ends_the_stream' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'protocol_errors_tests::cc_route_024_started_sse_ends_with_a_sanitized_error_frame' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据

- 已读 dispatch::forward_json、timeouts 的首输出/终止处理、共享 stream_observer 及 sse_framing、timeouts_tests::Fixture。dispatch 在超时观察前调用 validated_body；Chat choice 缺少 index 会立即拒绝，且 [DONE] 要求 choice 已 finish。仅修本文件 OUTPUT 的 index=0、完成序列的 finish_reason=stop，保留错误脱敏、超时、截断和终止后错误不转发等断言；不削弱业务校验。
- 修改前逐条运行本文件两条精确命令：日志 /tmp/shiju-87-red1.log、/tmp/shiju-87-red2.log，均退出101、实际0项。槽位3，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-3。
- 修改后逐条原命令复验：日志 /tmp/shiju-87-after1.log、/tmp/shiju-87-after2.log，均退出101、实际0项。槽位4，双缓存路径均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-4。两阶段顺序命令各合并一次 cargo_slot.py sh -c 入口；未运行全量测试、桌面编译或额外 check。
- 当前共同阻塞为 rmcp 0.8.5 的13个 E0425/E0432/E0433 编译错误，缺少 base64/schemars/paste、handler::server 及 capabilities builder 类型。相邻任务81–86已有相同共享依赖证据，未修改共享依赖清单或注册表。不能将编译失败算作当前行为红灯或通过。
- cargo_slot 入口均已退出并释放自有槽位，保留缓存；git diff --check 对本测试文件通过。审查仅上述两处夹具修正，无业务文件修改。
- 待共享依赖恢复后重跑两条精确命令，每条须实际1项且 passed=1、failed=0、退出0；仍需确认完成帧与后续错误同一读取时的行为。尚无行为验收证据，保留任务文件。
