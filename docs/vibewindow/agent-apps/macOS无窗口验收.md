# macOS 无窗口图标库验收

## 链路与范围

验收入口为 `vw-agent` 的独立测试模块 `gateway::api::handlers::agent_apps_acceptance_tests`，仅在 macOS 注册。真实 `build_router` 绑定 `127.0.0.1:0`，通过 `GatewayClient` 发送真实 HTTP 请求，使用临时 SQLite 或 `PostgresTestDatabase` 独立 schema。无窗口、无 `AppHandle`，不替代业务逻辑、数据库或 HTTP 服务。

认证在测试实例中注入随机 skey，使用生产 `extract_auth_skey` 与 `PairingGuard`；不修改进程全局凭证。`build_router` 原有 Basic 认证与租户中间件仍保留。因此应在未设置服务器 Basic 密码的隔离测试环境运行；该证据不等于成员授权或生产部署认证配置验收。

两种后端各有三个用例，共用前缀 `cc_base_icon_headless_`：

| 用例后缀（分别带 sqlite / postgres） | 断言 |
| --- | --- |
| `roundtrip` | 空库、新增中文名、内容去重、列表、改名保留 ID/图片/时间、删除与幂等删除 |
| `rejection_and_scope` | 无效认证的读写返回 401；缺失租户头返回 400；跨公司及跨平台读取为空、改名 404、删除不影响原作用域、新增内容独立；非法名称和图片不写入 |
| `disconnect_recovery` | 服务端完整接收正文后、进入业务处理前通知测试端；关闭原始 TCP 客户端，再放行真实改名；观测请求完成或因断连被取消，重建 SDK 连接，只允许完整旧值或完整新值，并与真实存储重读比对；后续删除成功证明无遗留事务锁 |

断开测试的门闩仅控制传输时序，正文继续交给真实 handler。完成通知由 RAII guard 同时覆盖响应完成与请求 future 被取消，避免把合法的提交前取消误判为超时。它不模拟存储错误，也不替代既有事务回滚测试。PostgreSQL 环境变量缺失直接报错，不使用跳过宏。

## 复现

先按编号任务规定动态领取 `target/rust-slot-locks/slot-N.lock`（1–10 顺序原子 mkdir），记录持有者与 PID，并设置绝对 `CARGO_TARGET_DIR=<仓库>/target/rust-slot-N` 及 `CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR`，后者覆盖仓库自定义中间目录。整个阶段持锁至 Cargo、服务器及相关子进程退出，清理自己的记录与锁，保留缓存。

使用独立本机 PostgreSQL 测试实例，连接串显式指定 `sslmode=disable`（仅本机临时实例）或 `sslmode=require`，将连接地址放入 `VIBEWINDOW_TEST_POSTGRES_URL`；不要使用生产数据库。随后运行：

```sh
cargo fmt --all
cargo test -p vw-agent --lib cc_base_icon_headless_ -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

PostgreSQL 夹具自动清理各自 schema；临时容器由验收调用方清理。每次必须确认命名用例实际运行，零测试或仅编译成功不算验收。

## 本次环境

2026-11-06：macOS 27.0（26A428），arm64；rustc 1.99.1。工作区 crate `vw-agent` / `vw-gateway-client` 0.6.0；Cargo.lock 中 axum 0.8.9、reqwest 0.13.4、rusqlite 0.37.0、postgres 0.19.14。PostgreSQL 18.6 在 Docker 的独立临时实例运行，服务与测试进程在 macOS；这不是 Linux 产品验收。

## 验证记录

本轮从槽位 1 顺序尝试后实际领取槽位 **2**，所有阶段均使用 `CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，并将 `CARGO_BUILD_BUILD_DIR` 指向同一绝对路径；不使用仓库默认中间目录。每阶段由持有进程写入 owner，退出时清理自己的锁，缓存保留。

- 红灯：`cargo fmt --all` 退出 0；`cargo test -p vw-agent --lib cc_base_icon_headless_ -- --nocapture` 退出 101，缺少新增 `Server` / `Gate` 夹具（E0433，其他诊断为未解析类型的派生错误）；没有用零测试当作失败证据。日志 `/tmp/shiju-task92-red.log`。
- 首次行为运行：同一测试命令退出 101，2 passed / 4 failed / 0 ignored；发现本机 PostgreSQL URL 未声明 sslmode，以及断开时请求取消导致完成通知缺失。日志 `/tmp/shiju-task92-green.log`。
- 第二次行为运行：同一测试命令退出 101，5 passed / 1 failed / 0 ignored；PostgreSQL 同步客户端在异步复核线程中启动运行时发生 panic。最终夹具将该复核移至 `spawn_blocking`。日志 `/tmp/shiju-task92-final.log`（此日志名不代表最终验收通过）。

实际用例名称如下，均位于 `gateway::api::handlers::agent_apps_acceptance_tests`：

1. `cc_base_icon_headless_sqlite_roundtrip`
2. `cc_base_icon_headless_postgres_roundtrip`
3. `cc_base_icon_headless_sqlite_rejection_and_scope`
4. `cc_base_icon_headless_postgres_rejection_and_scope`
5. `cc_base_icon_headless_sqlite_disconnect_recovery`
6. `cc_base_icon_headless_postgres_disconnect_recovery`

最终源码验证（`/tmp/shiju-task92-verify.log`）：

- `cargo fmt --all`：退出 **0**，无无关格式化差异。
- `cargo test -p vw-agent --lib cc_base_icon_headless_ -- --nocapture`：退出 **0**，上述 **6 passed / 0 failed / 0 ignored**，11435 filtered out；SQLite / PostgreSQL 各 3 项。断开实际观察到完整提交前与完整提交后状态，没有半写状态。日志 `/tmp/shiju-task92-test.log`。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出 **0**，无警告。日志 `/tmp/shiju-task92-clippy.log`。
- 测试链接成功，但链接器仍报告既有 `__eh_frame section too large` 提示；不据此宣称没有链接器提示。
- 全部 Cargo / 测试服务器进程已退出；自己的槽位 2 锁已释放，缓存保留。独立 PostgreSQL 容器随后停止并自动删除。未运行全量测试、wasm、Makefile 或跨平台验证。

本任务无窗口链路验收完成；批次 90/91 的真实窗口回归仍独立保留，不能由本证据替代。
