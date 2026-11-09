# macOS xAI 账号验收记录

母任务 6，批次 59。用户确认当前没有专用 xAI 测试身份，并同意按完成处理。后续覆盖核对将本批记为**用户豁免，真实账号未验证**。批次 58 已按同样口径交接。[批次 58 记录](验收-codex.md)

## 实际检查

在以 `f9069fd76` 为 HEAD 的共享工作区执行。平台为 macOS 27.0（26A428），arm64，ShiJu 0.6.0 使用默认 features。`grok --version` 退出码为 0，报告 `grok 1.0.77 (50ca579fa996) [stable]`。桌面设备授权经 `vw-gateway-client` 进入正式 Gateway 授权路由，执行主机使用 `XaiDeviceAuth`。现有 `AuthAccountStore::xai_access_token` 接入 `XaiRefresh`，调用方目前均为测试，生产 Gateway 路由未接入受管 xAI 账号消费。来源对照 cc-switch 4.0.25（`ca58c031`）。[设备授权](完成xAI设备授权.md) · [令牌刷新](刷新xAI凭据与重登状态.md) · [网关链路](macOS真实网关链路验收.md)

| 命令 | 槽位及绝对 `CARGO_TARGET_DIR` | 退出码 | 实际结果 |
| --- | --- | ---: | --- |
| `cargo test -p vw-agent-apps --lib auth6_xai_ -- --nocapture` | 3 · `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` | 0 | 4 通过、0 失败、795 过滤。回环 HTTP 与临时凭据覆盖设备授权、刷新轮换、暂时故障及重登标记。 |
| `cargo run -p vw-desktop --bin shi-ju` | 4 · `/Users/xiangmin/worker/code/shiju/target/rust-slot-4` | 0 | 原生桌面使用临时用户目录、应用配置和无监听服务的回环端口启动。进程定向退出请求被接受，桌面正常退出。链接器报告既有的 `__eh_frame` 大小提示。 |

两阶段均动态领取槽位，`CARGO_BUILD_BUILD_DIR` 与对应 `CARGO_TARGET_DIR` 相同。进程退出后释放各自槽位锁，保留编译缓存。桌面临时目录已清理。

## 真实账号覆盖

0 项通过、0 项失败、6 项未验证。用户默认会话未用于验收，未发起公网 OAuth。复验需要两个获准的专用身份、隔离的浏览器与执行主机，以及能在正式链路触发受管凭据刷新的生产调用。

| 场景 | 预期 | 实际 |
| --- | --- | --- |
| 设备登录 | 专用账号完成授权后显示 `ready`，公开响应及界面脱敏 | 未验证 |
| 令牌刷新 | 临期令牌经执行主机刷新，轮换后的凭据可继续使用 | 未验证 |
| 撤销后重登 | 上游撤销后拒绝旧凭据，账号显示需重登 | 未验证 |
| 默认切换 | 默认引用切换到另一个专用账号，两份凭据各自保留 | 未验证 |
| 登出 | 清除目标账号的受管凭据和默认引用，另一账号保持原状态 | 未验证 |
| 取消或失败后重试 | 原流程保持终态，新流程完成设备登录 | 未验证 |
