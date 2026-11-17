# 任务82：修复app-routing-services的dispatch_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号170。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/app-routing-services/src/dispatch_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据170：`route_auth::dispatch_tests::cc_route_037_authorized_native_gemini_json_and_sse`

位置：`crates/libs/app-routing-services/src/dispatch_tests.rs:525`；结果：FAIL。

```text
crates/libs/app-routing-services/src/dispatch_tests.rs:608:13:
    assertion `left == right` failed: /v1beta/models/fixture:generateContent
      left: 401
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p app-routing-services --lib 'route_auth::dispatch_tests::cc_route_037_authorized_native_gemini_json_and_sse' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据

- 已核实底层 `dispatch.rs` 调用 `gemini_oauth_transport::apply_api_key`：静态 key 要求全为 ASCII graphic；原 Bearer 夹具包含空格，返回 401。仅将本用例的发布凭据改为原生 key，并校验 x-goog-api-key 与 Authorization 缺席，保留 JSON/SSE、路径与未授权断言。
- 修改前后均执行上述精确原生命令，退出 101、实际测试 0 项。rmcp 0.8.5 依赖编译失败（13 项 E0425/E0432/E0433，缺失 base64/schemars/paste 及生成类型），未取得运行红灯或绿灯；未扩大到依赖修复。
- 修改前槽位 3：CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；日志 `/tmp/shiju-task82-red.log`。
- 修改后槽位 4：双缓存均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`；日志 `/tmp/shiju-task82-green.log`。两次前台入口已退出 101，自有锁已释放。
- 审查 diff 仅修改本测试凭据与对应上游断言；git diff --check 通过。共享工作区其他差异不属于本任务。依赖恢复后须重跑上述精确用例，要求 1 passed、0 failed 才能完成。
