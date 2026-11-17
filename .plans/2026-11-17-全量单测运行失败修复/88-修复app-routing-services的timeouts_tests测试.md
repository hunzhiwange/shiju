# 任务88：修复app-routing-services的timeouts_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号171, 172。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/app-routing-services/src/timeouts_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据171：`timeouts_tests::cc_route_021_stream_heartbeats_do_not_extend_idle_after_output`

位置：`crates/libs/app-routing-services/src/timeouts_tests.rs:242`；结果：FAIL。

```text
crates/libs/app-routing-services/src/timeouts_tests.rs:257:5:
    assertion `left == right` failed
      left: 502
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据172：`timeouts_tests::cc_route_021_stream_idle_zero_disables_idle_without_a_total_timeout`

位置：`crates/libs/app-routing-services/src/timeouts_tests.rs:288`；结果：FAIL。

```text
crates/libs/app-routing-services/src/timeouts_tests.rs:301:5:
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
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'timeouts_tests::cc_route_021_stream_heartbeats_do_not_extend_idle_after_output' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'timeouts_tests::cc_route_021_stream_idle_zero_disables_idle_without_a_total_timeout' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 执行记录

- 根因：dispatch.rs 的 validated_body 使用 StreamObserver::chat 严格要求 choices[].index；三个输出夹具缺失此字段，在输出开始前被拒绝为 502。只补 index:0，保留分片、心跳、空闲禁用、超时错误帧及关闭断言；无共享业务文件改动。
- 原生红灯和修改后均逐条执行上列两个精确命令，均退出101，实际测试数0（编译阶段受阻），不宣称验收通过。
- 两阶段均由 scripts/cargo_slot.py 持有槽位4；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-4。句柄60737、53724已退出，槽位锁已释放。
- 红灯日志：/tmp/task88-red-heartbeats.log、/tmp/task88-red-zero.log；修改后日志：/tmp/task88-fixed-heartbeats.log、/tmp/task88-fixed-zero.log。
- 外部阻塞：rmcp 0.8.5 client 特性组合编译失败（base64/schemars/paste 未解析、server 模块被 cfg 禁用，13项错误），与本次测试夹具改动无关。未扩大到共享依赖清单修复；依赖恢复后重跑本任务原生精确命令。
- 已审查diff：仅三个夹具字段；git diff --check 通过。未运行全量测试或桌面构建。保留任务文件供回归。
