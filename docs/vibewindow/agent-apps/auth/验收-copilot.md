# macOS Copilot 账号验收记录

母任务 6，批次 57。因缺少获准使用的 Copilot 专用测试账号，用户豁免本批真实账号验收，并同意将批次 57 视为计划前置完成，继续批次 58。后续覆盖核对将本批记为**用户豁免，真实账号未验证**。

## 实际检查

在以 `b562eeeb0` 为 HEAD 的工作区执行。平台为 macOS 27.0（26A428），arm64。ShiJu 为 `0.6.0`，使用默认 features。`copilot --version` 退出码为 0，报告 CLI `1.0.105`。

两次 Cargo 检查均动态领取槽位 1。`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同路径。相关进程退出后释放锁，保留构建缓存。

| 命令 | 退出码 | 实际结果 |
| --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_copilot -- --nocapture` | 0 | 6 通过、0 失败、793 过滤。回环 HTTP 夹具覆盖设备码申请、轮询和会话令牌处理。 |
| `cargo run -p vw-desktop --bin shi-ju` | 0 | 使用临时用户目录和独立应用配置启动原生桌面进程，随后正常退出并清理临时目录。桌面指向未运行网关服务的独立回环端口。 |

前序材料提供了网关生命周期和设备码弹窗的验证，公网 OAuth 及弹窗挑战使用测试夹具。[批次 56 网关链路](macOS真实网关链路验收.md) · [设备码弹窗](展示桌面设备码登录弹窗.md)

## 真实账号覆盖

0 项通过、0 项失败、6 项未验证。本批未启动公网授权流程。

| 场景 | 结果 |
| --- | --- |
| 公共 GitHub 设备授权 | 未验证 |
| 公共 GitHub Copilot 短期令牌兑换 | 未验证 |
| 取消后重新发起并成功授权 | 未验证 |
| 两个账号之间切换默认账号 | 未验证 |
| 登出目标账号并确认其他账号仍可用 | 未验证 |
| 获准的 GitHub Enterprise 域授权 | 未验证 |
