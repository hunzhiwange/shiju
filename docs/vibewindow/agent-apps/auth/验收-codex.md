# macOS Codex 账号验收记录

母任务 6，批次 58。当前缺少专用 ChatGPT 测试账号。用户豁免本批实号验收，并同意按批次完成处理。后续覆盖核对将本批记为**用户豁免，真实账号未验证**。

## 实际检查

在以 `1dd3761c5` 为 HEAD 的工作区执行。平台为 macOS 27.0（26A428），arm64。ShiJu 为 `0.6.0`，使用默认 features。`codex --version` 退出码为 0，报告 `codex-cli 0.167.0-alpha.3.2`。`codex login --help` 退出码为 0，列出浏览器登录入口和 `--device-auth` 选项。

批次 56 的网关记录覆盖 SDK 至执行主机的授权和登出。批次 57 有独立的实号验收豁免记录。桌面入口沿用 `crates/vw-desktop/src/app/message/agent_apps_auth/flow.rs`，经 `vw-gateway-client` 调用 `crates/vw-agent/src/gateway/api/handlers/agent_apps_auth_start.rs` 中的正式路由，设备及浏览器适配器位于 `crates/vw-agent-apps/src/auth/`。[批次 56 网关链路](macOS真实网关链路验收.md) · [批次 57 豁免](验收-copilot.md)

| 命令 | 槽位及绝对 `CARGO_TARGET_DIR` | 退出码 | 实际结果 |
| --- | --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_codex -- --nocapture` | 2 · `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 0 | 10 通过、0 失败、789 过滤。受控用例覆盖设备授权、浏览器回调、定向重登、刷新和轮换的成功及拒绝路径。 |
| `cargo run -p vw-desktop --bin shi-ju`（首次） | 2 · `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 1（监督脚本） | 临时用户目录使 rustup 开始下载工具链，随即中止。Cargo 退出码未采集，桌面未启动。 |
| `cargo run -p vw-desktop --bin shi-ju`（重试） | 1 · `/Users/xiangmin/worker/code/shiju/target/rust-slot-1` | 0 | 复用已安装的工具链和 Cargo 缓存。原生桌面进程使用临时用户目录、独立应用配置及无网关服务的回环端口启动，经进程定向退出请求正常退出。链接时出现既有 `__eh_frame` 大小提示。 |

各阶段均动态领取槽位，`CARGO_BUILD_BUILD_DIR` 与对应 `CARGO_TARGET_DIR` 相同。任务进程退出后释放锁、清理临时目录并保留编译缓存。

## 真实账号覆盖

0 项通过、0 项失败、8 项未验证。缺少获准的专用 ChatGPT 身份以及身份不符场景所需的第二个身份。本批未发起公网 OAuth，用户默认登录未被用于验收。桌面启动检查仅覆盖进程启动与退出。

| 场景 | 预期 | 实际 |
| --- | --- | --- |
| 设备登录 | 在专用账号确认后进入 `ready`，公开状态脱敏 | 未验证 |
| 浏览器登录 | 通过回环 PKCE 回调进入 `ready`，公开状态脱敏 | 未验证 |
| 同身份定向重登 | 更新目标账号凭据并保持账号 ID 和绑定 | 未验证 |
| 身份不符 | 拒绝回调并保留原账号凭据与绑定 | 未验证 |
| 原生 CLI 轮换 | 回读同身份的新一代凭据并清理私有启动目录 | 未验证 |
| 凭据失效 | 返回对应失效状态并引导重新授权 | 未验证 |
| 登出 | 清理目标受管凭据，保留其他账号状态 | 未验证 |
| 取消或失败后重试 | 原流程停留在终态，新流程完成授权 | 未验证 |
