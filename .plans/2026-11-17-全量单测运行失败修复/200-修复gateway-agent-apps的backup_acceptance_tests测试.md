# 任务200：修复gateway-agent-apps的backup_acceptance_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：待回归

来源：原任务19全量实际执行；证据编号403, 404。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/agent_apps_data/backup_acceptance_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据403：`agent_apps_data::backup_acceptance_tests::cc_data_46_macos_backup_restore_retention_round_trip`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_data/backup_acceptance_tests.rs:87`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_data/backup_acceptance_tests.rs:79:9:
    assertion `left == right` failed
      left: 503
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据404：`agent_apps_data::backup_acceptance_tests::cc_data_46_macos_corrupt_backup_preserves_target`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_data/backup_acceptance_tests.rs:165`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_data/backup_acceptance_tests.rs:79:9:
    assertion `left == right` failed
      left: 503
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

## 步骤与验证

1. 重读最新源码和diff，核实所有者与失败条件。逐条运行下面原生精确命令保存当前红灯；死锁用例先检查重复锁并使用外部有界超时保留现场，不能无限等待。
2. 比较真实接口、业务预期及现有隔离夹具，先检查底层；只修证据证明的问题。保留安全/租户隔离/精确错误/主题与交互断言。
3. 最小修复后运行全部下列命令，若共享底层受影响，再选择直接调用测试补充。修复编译期间的签名/trait/宏变化须立即原生验证，不积压到最终回归。

```sh
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_data::backup_acceptance_tests::cc_data_46_macos_backup_restore_retention_round_trip' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_data::backup_acceptance_tests::cc_data_46_macos_corrupt_backup_preserves_target' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 本次执行证据（2026-11-17）

- 原命令分别运行1项，均 failed=1、退出101；当前实际红灯为 create 返回403，日志 `/tmp/task200-red1.log`、`/tmp/task200-red2.log`。旧503不可直接沿用。
- 当前访问层 execution_context 已采用网关本机身份并忽略旧 policy；夹具改用 desktop-local-user / 本机HOST_ID，以独立子进程设置 VIBEWINDOW_TEST_ROOT，备份目录跟随真实租户路径。保留恢复、保留策略、损坏保护及外租户哨兵断言。不改共享业务文件。
- 红灯槽位4，修复后槽位6；每次 CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 都为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-N`（N为对应槽位）。入口为 scripts/cargo_slot.py sh -c，同阶段两条原精确命令顺序运行。
- 修复后日志 `/tmp/task200-green1.log`、`/tmp/task200-green2.log`；受共享工作区 tool-computer 编译失败影响，尚无本任务通过证据。错误 E0432/E0433 BrowserPermission、E0609 browser_permissions、E0599 transfer_permission，位于 computer_control/chrome.rs:106/107/170。未接管无关并发变更。需依赖修复后重跑两条精确命令，确认父子进程各1项通过。
- 第一条修复后命令已报告上述编译错误；等待其它编译作业收尾后停止自有入口（退出130），自有进程组已退出、槽位6已释放，第二条修复后命令未执行。
- 仅审查自身diff并做 git diff --check；不运行全量测试或桌面编译。保留本任务文件供定向回归。
