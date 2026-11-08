# 管理 DSH 共享凭据记录

母任务 6／批次 32。核对前置批次 1–31 的交付提交，直接前置为 [完成 DSH 浏览器授权](完成DSH浏览器授权.md)（`3f013396f`）。沿用已交付的执行主机和供应商事务合同。来源为 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/dsh_account.rs`、`commands/dsh.rs` 和 `dsh_config.rs`。

执行主机确认可信作用域拥有原生槽后，通过 `auth::dsh_account::DshAccount` 启动、查询及本地登出。Web 和 Desktop 共用 `DSH_HOME/.credentials.yaml` 的 `records["deepseek-account-platform/default"]`。路径解析、原生 `.credentials.yaml.lock`、进程内应用锁和原子发布复用 `providers::dsh` 与 live 写入适配。遇到待处理的 DSH 供应商事务时返回冲突。

登录复用有效设备 ID。首次登录的设备 ID 与 grant 在兑换成功后一起原子写入。提交时重查该 grant 与设备 ID，拒绝覆盖启动后变化的记录，并以锁内读到的最新 YAML 为基础保留其他记录、API key 引用和扩展字段。成功写入沿用来源的 YAML 序列化方式，保留字段值，注释会丢失，排版可能变化。登出只移除该 grant，重复登出保持文件字节。无效文档、无效 grant、原生锁冲突与不安全的文件路径均返回稳定错误。

状态使用现有脱敏账号 DTO，只向固定 DeepSeek 平台的 `GET /auth-api/v0/users/current` 发送原生 `x-dsh-auth-token` 请求头。HTTP 401 或业务码 40003 标记需重新登录。禁用代理与跳转，外部 issuer 的凭据可以本地清除。余额归母任务 7，远端撤销和统一登出代际归批次 36–37，真实账号及应用消费归批次 63。

## 验证

macOS 27.0（26A428），arm64。使用真实临时 YAML、文件权限、锁、符号链接、回环浏览器回调和 HTTP；仅替换公网 DeepSeek 响应。验证共享文件内授权替换、设备 ID 创建和复用、状态请求与脱敏、登出、独立 profile 字节、外部新增引用保留、非法兑换及过期 grant 的旧字节、锁冲突、异机作用域、跳转拒绝和外部新 grant 不被覆盖。

红灯分别断言「缺少 grant 时已登出」和「未知 issuer 需要重新授权」，均收到 `Unsupported`。首轮实现的测试和 Clippy 编译发现当前 UUID 未启用 Serde，改为沿用来源的 UUID 字符串后重新运行。

各阶段均动态领取槽位 1。`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。在源码快照执行格式化，只将本批文件写回。验证进程结束后释放本批锁，保留缓存。

| 阶段 | 命令 | 退出码 | 结果 |
| --- | --- | ---: | --- |
| 红灯、首轮实现、最终验证 | `cargo fmt --all` | 0／0／0 | 格式化成功 |
| 红灯 | `cargo test -p vw-agent-apps --lib auth6_dsh_account -- --nocapture` | 101 | 0 通过、2 失败、777 过滤 |
| 最终验证 | `cargo test -p vw-agent-apps --lib auth6_dsh_account -- --nocapture` | 0 | 2 通过、0 失败、777 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib auth6_dsh_login -- --nocapture` | 0 | 2 通过、0 失败、777 过滤 |
| 相邻回归 | `cargo test -p vw-agent-apps --lib cc04_dsh -- --nocapture` | 0 | 6 通过、0 失败、773 过滤 |
| 最终验证 | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| 差异检查 | `git diff --check`、`git diff --cached --check` | 0／0 | 通过 |
