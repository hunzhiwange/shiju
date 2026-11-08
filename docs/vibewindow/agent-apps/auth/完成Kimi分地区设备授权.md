# 完成 Kimi 分地区设备授权

母任务 6／批次 21。批次 1–20 的合同及宿主适配已交付，直接前置见[刷新 xAI 凭据与重登状态](刷新xAI凭据与重登状态.md)。协议对照 cc-switch 4.0.25（`ca58c0313360eae7c851fbd7553c1614c7af03e7`）的 `commands/kimi_oauth.rs`、`services/kimi_oauth/mod.rs` 和 `services/kimi_oauth/account.rs`。

`KimiDeviceAuth` 使用显式地区选择固定端点。大陆区使用 `auth.kimi.com` 和 `api.kimi.com/coding/v1`，全球区使用 `auth.kimi.ai` 和 `api.kimi.ai/coding/v1`。设备挑战、令牌兑换和 `/me` 请求共用同一地区和设备 ID。新登录由宿主生成 UUID，定向重登使用账号已保存的地区和设备 ID。显式地区冲突在网络请求前被拒绝。浏览器验证链接须使用同地区认可域名的 HTTPS，宿主请求禁用重定向。

同一平台、公司、成员和执行主机只保留一轮待完成的 Kimi 授权。通过输入校验的新一轮在请求挑战前取消上一轮，旧请求即使已经在途也无法提交。其他作用域的流程独立，轮询沿用宿主状态机的间隔、降频、取消和到期规则。

`AuthAccountStore::start_kimi_device` 与 `poll_kimi_device` 连接宿主业务与现有凭据租约、账号元数据发布事务。账号 ID 绑定作用域、地区和 `/me` 验证的用户身份。身份无法验证时登录失败，避免建立无法确认归属的托管账号。定向重登只能更新原账号。元数据保存地区、身份哈希、标签及授权时间，私有文件保存访问令牌、刷新令牌、设备 ID、地区及毫秒级令牌到期时间。

原生配置写入和刷新轮换交由批次 22。公开 API 接线归批次 39–41，真实 Kimi 账号验收归批次 60。

## 验证

macOS 27.0 arm64。红灯接线返回 `Unsupported`。有效地区挑战预期启动，实际返回 `Unsupported`。缺少地区的新登录预期返回 `InvalidInput`，实际返回 `Unsupported`。两项都运行到行为断言。

绿灯使用真实回环 HTTP 和受控单调时钟。测试核对两个地区的授权、兑换与身份查询端点、表单和设备头，验证同一上游用户 ID 按地区得到不同账号，定向重登复用原设备，以及跨地区、跨主体和身份不符的拒绝路径。延迟真实 HTTP 身份响应验证重启后旧流程无法写入，并验证挑战失败的重启也会取消旧流程。

格式化在源码快照执行，只回写本批文件。`CARGO_BUILD_BUILD_DIR` 与所用的 `CARGO_TARGET_DIR` 一致。

| 命令 | 槽位 | 退出码 | 结果 |
| --- | ---: | ---: | --- |
| `cargo fmt --all`（红灯／最终绿灯） | 1、3 | 0／0 | 通过 |
| `cargo test -p vw-agent-apps --lib auth6_kimi_device -- --nocapture`（红灯） | 1 | 101 | 0 通过、2 失败、749 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_kimi_device -- --nocapture`（绿灯） | 3 | 0 | 2 通过、0 失败、749 过滤 |
| `cargo test -p vw-agent --lib auth6_kimi_device_store -- --nocapture` | 2 | 0 | SQLite、PostgreSQL 各 1 通过、0 失败、11750 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 4 | 0 | 通过 |
| `git diff --check` 和 `git diff --cached --check` | — | 0／0 | 通过 |

存储测试重开真实 SQLite 文件，并在本机 Lima PostgreSQL 实例中使用独立 schema。两库都验证了重登目标读回、身份与旧代拒绝、跨成员隔离及令牌和设备 ID 脱敏。macOS 测试链接出现既有的 `__eh_frame` 大小警告。PostgreSQL 已停止。

实际 `CARGO_TARGET_DIR` 如下，本任务的槽位锁已释放，缓存保留。

- 槽位 1 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。
- 槽位 2 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- 槽位 3 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。
- 槽位 4 使用 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`。
