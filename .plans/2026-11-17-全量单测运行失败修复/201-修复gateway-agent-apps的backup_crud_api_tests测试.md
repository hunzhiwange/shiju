# 任务201：修复gateway-agent-apps的backup_crud_api_tests测试

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次1】 依赖：无

状态：已完成

来源：原任务19全量实际执行；证据编号405, 408。
预计会话范围：仅本测试定义文件中以下失败族及证据证明必要的底层修复，不接管整包。

## 文件与真实诊断

- 修改/测试：`crates/libs/gateway-agent-apps/src/agent_apps_data/backup_crud_api_tests.rs`。
- 读取：该测试调用的业务模块、相邻夹具、父模块注册和直接调用者；额外共享写入遵守公共所有权规则。

### 证据405：`agent_apps_data::backup_crud_api_tests::cc_data_34_backup_api_maintains_isolated_backups`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_data/backup_crud_api_tests.rs:83`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_data/backup_crud_api_tests.rs:78:9:
    assertion `left == right` failed
      left: 503
     right: 200
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

### 证据408：`agent_apps_data::backup_crud_api_tests::cc_data_34_backup_api_rejects_cross_tenant_id`

位置：`crates/libs/gateway-agent-apps/src/agent_apps_data/backup_crud_api_tests.rs:128`；结果：FAIL。

```text
crates/libs/gateway-agent-apps/src/agent_apps_data/backup_crud_api_tests.rs:78:9:
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
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_data::backup_crud_api_tests::cc_data_34_backup_api_maintains_isolated_backups' -- --exact --nocapture
python3 scripts/cargo_slot.py cargo test -p gateway-agent-apps --lib 'agent_apps_data::backup_crud_api_tests::cc_data_34_backup_api_rejects_cross_tenant_id' -- --exact --nocapture
```

完成证据：各条精确过滤执行1项且passed=1、failed=0、退出0；参数化子进程用例同时记录子进程证据。记录槽位、双缓存路径、日志、代码状态和审查diff。不运行全量测试；最后统一验收任务负责跨任务复验。

## 执行证据

- 根因：执行上下文已由 Gateway 本地主机提供，策略夹具不再参与路由授权；旧 reader 身份和目录与真实执行上下文不一致。仅迁移本文件夹具，使用 desktop-local-user/local host，并以子进程 VIBEWINDOW_TEST_ROOT 隔离缓存路径，保留全部业务及租户隔离断言；无共享业务文件写入。
- 初始两条原命令经单个 cargo_slot.py sh -c 入口执行。槽位3，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-3。日志 /tmp/shiju-201-red-1.log、/tmp/shiju-201-red-2.log。首条遇 tool-computer 既有 BrowserPermission/browser_permissions/transfer_permission 编译错误退出101，未取得当前运行红灯；第二条编译时已读到修复后夹具，父/子各1 passed、0 failed，退出0。
- 最终按本文件两条精确 Cargo 命令顺序执行（同一槽位入口，前条成功再执行后条），槽位4；双缓存路径均为 /Users/xiangmin/worker/code/shiju/target/rust-slot-4。日志 /tmp/shiju-201-green-1.log、/tmp/shiju-201-green-2.log。每条父/子进程均执行1项，1 passed、0 failed，入口退出0；未执行全量测试或额外 check。
- 已审查本文件与测试 diff；rustfmt 仅修改测试文件，git diff --check 通过。两个入口均已退出并释放自有槽位锁，保留缓存。工作区另有大量他人变更与暂存内容，不纳入本次提交。
