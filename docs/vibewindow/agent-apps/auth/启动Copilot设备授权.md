# 启动 Copilot 设备授权

母任务 6／子任务 11。依赖批次 10 的[流程到期与轮询节流](流程到期与轮询节流.md)。启动入口为 `vw_agent_apps::auth::copilot::CopilotAuth::start`，复用 `AuthFlows` 注册作用域和到期清理，返回公开的 `AuthStartResponse` 和仅供执行主机持有的设备授权材料。对照 cc-switch 4.0.25（`ca58c031`）的 `commands/copilot.rs` 与 `proxy/providers/copilot_auth.rs`。

- 缺省使用 `github.com`。Enterprise 仅接受执行主机提供的许可列表中精确匹配的规范 authority。输入去除首尾空白并统一大小写，默认 HTTPS 端口规范为省略。非默认端口需单独许可。拒绝 URL、路径、用户信息、IP 字面量及未经许可的 authority。
- 以对应部署的 OAuth 客户端 ID 向同一 HTTPS authority 的 `/login/device/code` 发送表单，范围为 `read:user`。HTTP 客户端禁止重定向。仅发放匹配同一 authority 的 `/login/device` 验证页，保留上游轮询间隔，按单调时钟判定到期。
- 公开响应只携带流程 ID、用户码、验证页与时间字段。上游设备码随执行主机流程清理。规范 authority 保存在流程与授权材料中，供批次 12 兑换时按同域及 GitHub 用户 ID 判定身份。

## macOS 验证

macOS 27.0（26A428），arm64。回环 HTTP 替代公网 OAuth，检查实际 POST 路径、请求头与表单。验证公共域、许可 Enterprise 域和许可非默认端口，拒绝越界输入、无效时间、恶意验证地址及重定向。测试同时检查公开响应脱敏、跨主体查找拒绝，以及取消和到期清理。真实 GitHub 账号验收归批次 57。

红灯的两项行为断言均失败。公共域返回 `Unsupported`，未产生挑战。空 authority 返回 `Unsupported`，预期为 `InvalidInput`。

红灯和绿灯证据均来自动态领取的槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。首次红灯运行继承了仓库默认的中间产物目录，随后设置 `CARGO_BUILD_BUILD_DIR` 并在槽位内重跑。格式化在源码快照内运行，仅回写本批 Rust 文件。

| 命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo fmt --all` | 0 | 红灯和绿灯均通过 |
| `cargo test -p vw-agent-apps --lib auth6_copilot_start -- --nocapture`（红灯） | 101 | 0 通过、2 失败、729 过滤 |
| `cargo test -p vw-agent-apps --lib auth6_copilot_start -- --nocapture`（绿灯） | 0 | 2 通过、0 失败、729 过滤 |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 通过 |
| `git diff --check`、`git diff --cached --check` | 0 / 0 | 通过 |
