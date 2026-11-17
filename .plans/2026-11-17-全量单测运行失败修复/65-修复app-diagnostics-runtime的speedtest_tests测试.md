# 任务65：修复app-diagnostics-runtime的speedtest_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号136。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/app-diagnostics-runtime/src/agent_apps_diagnostic_executors/speedtest_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据136：`agent_apps_diagnostic_executors::speedtest::tests::diagnostic_speedtest_reports_each_endpoint`

位置：`crates/libs/app-diagnostics-runtime/src/agent_apps_diagnostic_executors/speedtest_tests.rs:88`；结果：FAIL。

```text
crates/libs/app-diagnostics-runtime/src/agent_apps_diagnostic_executors/speedtest_tests.rs:193:9:
    assertion `left == right` failed
      left: Succeeded { evidence: SpeedEvidence { http_status: 502, latency_ms: 6 } }
     right: Failed { error: DiagnosticFailure { phase: ResponseHeaders, code: Transport }, http_status: None }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p app-diagnostics-runtime --lib 'agent_apps_diagnostic_executors::speedtest::tests::diagnostic_speedtest_reports_each_endpoint' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行记录

- 代码审查：测速共用 upstream_http_client_builder，默认采用宿主代理；测试通过直接断开测量连接断言 Transport。代理可能将该断连转换为 HTTP 502，与证据136一致。相邻 reachability 夹具使用 no_proxy。未取得当前行为红灯，原因见编译阻塞。
- 修复仅限本任务测试文件：以同一精确测试名启动独立子进程，设置 NO_PROXY/no_proxy=*，避免更改父进程全局环境；保留真实 TCP 夹具、业务提交入口和所有断言。子进程超时30秒后 kill 并等待回收。
- 修复前后均运行原定精确 cargo_slot.py 命令。日志：/tmp/shiju-task65-red.log、/tmp/shiju-task65-green.log。均在 rmcp 0.8.5 编译阶段报13个 E0432/E0433/E0425（base64、schemars、paste、server handler 等不可用），两次退出101，未执行测试，实际测试数0；不能记为通过，未获得子进程通过证据。
- 两次实际槽位均为3；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-3。两次入口均已退出并释放自有锁；槽位后续已由其他任务领取，未触碰新所有者。保留缓存。
- git diff --check 针对修改文件通过；代码差异仅增加测试进程代理隔离。未修改公共依赖清单、Cargo.lock、生产代理行为或其他任务文件。
- 下一步：公共依赖编译问题解决后，重跑上方原定命令，确认父、子进程各1项通过；验证前保留本任务文件。
