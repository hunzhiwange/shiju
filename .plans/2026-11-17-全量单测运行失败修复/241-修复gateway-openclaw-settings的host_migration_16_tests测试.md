# 任务241：修复gateway-openclaw-settings的host_migration_16_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号470。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-openclaw-settings/src/host_migration_16_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据470：`authorization::host_migration_16_tests::openclaw_api_protects_sensitive_env`

位置：`crates/libs/gateway-openclaw-settings/src/host_migration_16_tests.rs:32`；结果：FAIL。

```text
crates/libs/gateway-openclaw-settings/src/host_migration_16_tests.rs:51:5:
    assertion `left == right` failed
      left: 401
     right: 403
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-openclaw-settings --lib 'authorization::host_migration_16_tests::openclaw_api_protects_sensitive_env' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-18）

- 当前源码已将应用认证归属 Gateway；`execution_context` 忽略旧应用令牌及 grants。旧 policy.json 不能再控制临时 home 和敏感权限。
- 仅修改本任务测试定义文件：通过现有 Authorization 注入边界验证真实路由、文件读取及投影；保留 403、拒绝前无 IO、健康信息脱敏、原文件不变和获准读取 secret 断言，增加租户/项目/主机参数断言。未修改业务安全策略。
- 修前与修后均运行本文件指定精确命令；均退出 101，实际执行 0 项。阻塞为第三方 rmcp 0.8.5 编译缺少 base64、schemars、paste，以及 server/Builder 类型解析错误（13 errors），未得到测试红绿结果。
- 两次均使用槽位 1；CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。日志：`/tmp/shiju-task241-red.log`、`/tmp/shiju-task241-green.log`。入口及子进程已正常结束，锁由入口释放，缓存保留。
- 本任务代码 diff 已审查，限定单一测试文件，`git diff --check` 通过。依赖错误不在本任务写入范围；待依赖恢复后重跑指定精确命令，须确认执行 1 项并通过才能删除本任务卡。
