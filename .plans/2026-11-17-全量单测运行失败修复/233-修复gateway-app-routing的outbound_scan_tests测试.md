# 任务233：修复gateway-app-routing的outbound_scan_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已阻塞

来源：原任务19全量实际执行；证据编号450。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据450：`host_migration_7_tests::outbound_scan_tests::cc_route_030_outbound_scan_expected_behavior`

位置：`crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs:24`；结果：FAIL。

```text
crates/libs/gateway-app-routing/src/host_migration_7_tests/outbound_scan_tests.rs:79:5:
    assertion `left == right` failed
      left: 503
     right: 202
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-app-routing --lib 'host_migration_7_tests::outbound_scan_tests::cc_route_030_outbound_scan_expected_behavior' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-18）

- 已补齐本用例私有 home 权限（0700）及 `.local/state/shiju`，诊断存储 `open_on_execution_host` 不递归创建父目录；生产启动负责准备状态根目录。未修改共享 fixture 或业务安全逻辑。
- 原精确命令执行 1 项，0 passed / 1 failed，退出 101：首个权限断言实际 503，预期 403；日志 `/tmp/shiju-task233-red.log`。
- 修复后同一精确命令执行 1 项，0 passed / 1 failed，退出 101：首个权限断言实际 202，预期 403；日志 `/tmp/shiju-task233-green.log`（文件名不代表通过）。
- 两次均使用槽位 1，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`；入口已退出，保留缓存。
- 当前共享授权迁移使 `app-access-service::authorize_scoped` 调用 `execution_context`，忽略旧 policy/token/grants，身份固定 `desktop-local-user`；因此旧 READER、租户和 host 授权断言无法依靠旧策略夹具验证。未删除或弱化任何断言，未回退共享授权实现；需要授权迁移负责方提供 Gateway 配对授权的替代隔离测试入口后定向回归。
- 已审查本任务 diff，仅测试初始化 4 行；其他任务的共享 fixture、Cargo.lock 和暂存内容保持原样。不运行全量测试。

- 本任务提交：`f85871d60`，仅包含 outbound_scan_tests.rs；编号文件保留待回归。

## 授权契约复核（2026-11-18）

- 生产 `client_router` 使用 `skey_auth_middleware`。开启配对鉴权时，无效或缺失 skey 返回 401，有效 skey 只放行请求，不传递逐凭据主体或读写角色。`5cb8b43dc` 移除了旧项目和主机授权门槛，`execution_context` 仍忽略旧令牌及 grants，并产生固定主体 `desktop-local-user`。因此接入真实中间件也无法保留原单测的项目未授权、READER、异租户和异主机的 403 契约。
- 诊断注册器仍按主体及完整 `DiagnosticScope` 限制状态与取消。单测中的探针读取扫描任务 404 是任务类型断言，应保留。生产集成用例另有旧 PEER 主体断言，有效 skey 之间无法制造该主体差异。替换授权或取消旧断言会改变所验证的产品契约。
- 独立目录修复已提交 `f85871d60`。本轮仅静态复核，遵守调度未启动 Cargo。最近实际定向验证仍为 1 项失败（202 ≠ 403），生产集成用例未运行。与任务 232 等待同一决策：按配对契约替换旧断言并保留任务范围隔离覆盖，或恢复产品层角色、主机、项目和主体授权，再进行定向及集成回归。仅编号状态更新，无新代码提交。
