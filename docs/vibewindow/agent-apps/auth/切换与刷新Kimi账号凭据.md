# 切换与刷新 Kimi 账号凭据

母任务 6／批次 22，接续[完成 Kimi 分地区设备授权](完成Kimi分地区设备授权.md)。对照 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `services/kimi_oauth/{account,lock,mod}.rs` 和 `kimi_config.rs`。

`AuthAccountStore::use_kimi_account` 读取调用者、租户与执行主机作用域内的账号元数据，再由 `KimiDeviceAuth::use_account` 取得账号凭据租约。宿主传入已授权的原生主目录、配置事务和目标供应商。选择要求该供应商已有原生模型别名及文件型 OAuth 凭据。登录只建立托管账号，选用现有原生供应商时才通过母任务 4 的 `KimiFiles::apply` 切换 `default_model`。

供应商的 API 和 OAuth 端点必须与托管地区相符，身份由同地区 `/me` 的地区与用户 ID 哈希验证。原生槽位使用 Kimi 的 `oauth/<slot>.lock` 目录协议。已有锁返回 `AccountInUse`，锁禁用或配置使用钥匙串返回 `Unsupported`。刷新前后读取当前代际，锁有效且文件仍匹配时才替换，其他进程已写入的新代际须仍有效且通过身份核验。确定的刷新拒绝只为原代际写入撤销标记。独立 CLI 凭据必须在可验证有效身份时使用，独立刷新采用 CLI 设备 ID。只有起始代际与托管凭据一致的轮换才写回托管凭据。配置事务在发布前复核已验证的凭据代际和配置快照。

公开 API 装配和真实账号验收由后续批次覆盖。本批在 macOS 使用隔离的文件与回环 HTTP，没有写入用户的默认 Kimi 主目录。

## 验证

macOS 27.0（26A428），arm64。红灯测试走到两项业务断言。地区匹配的切换预期成功，地区不符预期 `IdentityMismatch`，最小接线在两处均返回 `Unsupported`。

绿灯使用真实临时文件、回环 OAuth 与独立文件操作进程。断言默认模型切换保留配置注释、刷新提交原生文件和同源托管凭据、拒绝地区和身份不符、占锁不发送网络请求、过期独立凭据拒绝续期、确定拒绝写入撤销标记，以及同一槽位出现新代际时保留并验证该文件。格式化在工作区快照运行，仅回写本批文件。测试未新增数据库后端写入。

| 命令 | 槽位 | 退出码 | 结果 |
| --- | ---: | ---: | --- |
| `cargo fmt --all`（红灯／最终绿灯） | 2／1 | 0／0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_kimi_account -- --nocapture`（红灯） | 2 | 101 | 0 通过、2 失败、751 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_kimi_account -- --nocapture`（最终绿灯） | 1 | 0 | 2 通过、0 失败、751 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 1 | 0 | 通过 |
| `git diff --check` 与 `git diff --cached --check` | — | 0／0 | 通过 |

各阶段均令 `CARGO_BUILD_BUILD_DIR` 等于 `CARGO_TARGET_DIR`。本批占用的锁均已释放，保留构建缓存。

- 槽位 1 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 槽位 2 的 `CARGO_TARGET_DIR` 为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
