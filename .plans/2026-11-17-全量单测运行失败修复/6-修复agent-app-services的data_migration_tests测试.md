# 任务6：修复agent-app-services的data_migration_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号20。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/agent-app-services/src/mcp/data_migration_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据20：`mcp::data_migration_tests::cc_data_10_owner_add_skip_replace_and_repeat_without_duplicate_rows`

位置：`crates/libs/agent-app-services/src/mcp/data_migration_tests.rs:16`；结果：FAIL。

```text
crates/libs/agent-app-services/src/mcp/data_migration_tests.rs:39:88:
    called `Result::unwrap()` on an `Err` value: StorageUnavailable
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p agent-app-services --lib 'mcp::data_migration_tests::cc_data_10_owner_add_skip_replace_and_repeat_without_duplicate_rows' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 2026-11-17 执行记录

- 底层 `safety_snapshot_files::root` 使用 O_NOFOLLOW 逐段打开绝对路径，拒绝任何符号链接；相邻 safety_snapshot_tests 先 canonicalize 临时目录。macOS 临时目录含 /var 符号链接，本夹具原先直接传入导致 StorageUnavailable。仅将全部快照操作改用规范化后的私有临时根目录，保留 0700、快照、幂等、Skip/Replace、revision 和 stale CAS 断言，不修改业务安全策略。
- 修改前原生精确命令日志 `/tmp/shiju-task6-red.log`，修改后同命令日志 `/tmp/shiju-task6-green.log`；均退出101，实际执行0项。现有 rmcp 0.8.5 client-only 依赖组合发生13个 E0432/E0433/E0425（base64、schemars、paste、handler::server 等缺失），未进入测试；任务5也记录同一无关阻塞。不扩大到共享依赖配置修复。
- 两次均使用 cargo_slot.py 槽位3，CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`；入口已退出并由脚本释放锁，保留缓存。最终 stale CAS 调用也已统一传入规范化根目录，该修改仍需原生命令验证。
- diff 审查仅本测试夹具路径，无业务文件/父模块接线变更。未运行全量测试或重复 check。
- 恢复条件：集成 rmcp 依赖特性修复后，重跑本编号精确命令，取得执行1项 passed=1 failed=0 退出0再删除任务文件。
