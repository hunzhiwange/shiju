# macOS 钥匙串存储

母任务 6／子任务 7。核对批次 1–6 的交付提交 `cd0dfb217`、`14acbba54`、`c7d8474f1`、`f2e90ecc8`、`ba4883457` 和 `ceb96d943`。沿用已验收的原生包、网关认证与双后端存储底座，以及相邻 Claude 供应商切换事务。来源为 cc-switch 4.0.25 的 `ca58c031`，核对了 Claude 快照钥匙串和 AGY 登录存储的读写及测试。[底座交付](../底座交付映射.md) · [主机凭据存储](执行主机凭据文件存储.md) · [安全边界](安全边界.md)

## 执行边界

- `vw-agent-apps/src/auth/keychain.rs` 实现主机侧 generic password 条目操作。现有 Claude 适配继续用其服务名、账号规则、HOME 授权检查和 JSON 解码，通过相同的 `/usr/bin/security` 执行身份访问。网关已拒绝无执行通道的远端主机。AGY 登录和切换由后续批次接入。
- 写入前解析匹配条目所在的钥匙串。缺失条目在当前默认钥匙串创建，若并发创建导致冲突则返回错误。更新前读取原条目确认访问权，在同一个钥匙串原地更新并回读。固定写入位置避免更新失败后改写默认钥匙串。路径解析失败或路径无法安全放入交互命令时拒绝写入。[Apple SecurityTool 更新源码](https://github.com/apple-oss-distributions/Security/blob/main/SecurityTool/macOS/keychain_add.c)
- 凭据通过标准输入传入，交互命令上限为 4032 字节，其中包含条目标识、钥匙串路径和十六进制载荷。超限时显式失败。交互命令报告失败、回读不匹配和进程超时都返回脱敏错误。适配器没有文件存储入口。Claude 事务原有的缺项兼容逻辑只在钥匙串报告未找到条目时适用。

## macOS 验证

macOS 27.0（26A428），arm64。格式化在源码快照运行，回写仅限本任务 Rust 文件。各阶段动态领取槽位，`CARGO_BUILD_BUILD_DIR` 与 `CARGO_TARGET_DIR` 一致。所有本任务锁已释放，构建缓存已保留。

| 阶段 | 槽位 | CARGO_TARGET_DIR |
| --- | ---: | --- |
| 红灯 | 1 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` |
| 绿灯、Claude 回归和全工作区 Clippy | 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` |
| 最终手工 ACL 探针 | 3 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` |
| 早期 ACL 探针 | 2、4 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-4` |

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo fmt --all`（红灯、初次及最终绿灯） | 0 / 0 / 0 | 格式化成功 |
| `cargo test -p vw-agent-apps --lib auth6_keychain -- --nocapture`（红灯） | 101 | 0 通过，2 失败，721 过滤 |
| 同一命令（最终绿灯） | 0 | 2 通过，0 失败，721 过滤 |
| `cargo test -p vw-agent-apps --lib claude_login -- --nocapture` | 0 | 10 通过，0 失败，713 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 全工作区通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 无空白错误 |

红灯的成功用例在读取新命名空间时收到 `Unsupported`。拒绝用例发现带引号和换行的账号仍被接受。绿灯在真实钥匙串中验证同一服务下两个账号隔离，创建、覆盖、回读 Unicode 值、删除及重复删除均符合预期。超长写入被拒绝，原值可继续读回。测试条目删除后查找为空。

手工 ACL 探针用 `security -q -i` 和 `-T ""` 在独立 service/account 写入合成数据。macOS 授权弹窗无法通过自动化操作，因而在独立 Swift 探针进程中禁止交互，再调用 `SecKeychainFindGenericPassword`。系统返回 `errSecAuthFailed`（−25293），没有返回数据。另用 `rustc` 编译引用生产适配器的临时探针，读取和写前检查均返回 `TimedOut`。夹具删除退出码为 0，随后查找退出码为 44。最终探针脚本退出码为 0。早期探针预期了另一种 OSStatus，经复跑确认实际为 −25293。验收仅使用合成数据和测试条目。
