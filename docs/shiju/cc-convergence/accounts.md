# 账号凭据所有者收敛

2026-11-14，任务4。按用户要求本轮只实现，不运行 Cargo、单元测试、实机测试或最终回归。以下是静态核对结果，不代表当前行为已验证。

## 生产调用链与保留边界

- Iced 的 `crates/vw-desktop/src/app/components/system_settings_auth.rs` 经 `crates/vw-gateway-client/src/client/agent_apps_auth_accounts.rs` 的管理接口访问账号。SDK不持有原始刷新令牌；网关 `agent_apps_auth_read.rs`、`agent_apps_auth_start.rs` 及同目录账号变更处理器在主机授权后调用领域所有者。
- `crates/vw-agent/src/agent_apps/auth_accounts.rs::AuthAccountStore` 是受管账号入口。元数据使用 `agent_apps.auth_accounts.v2`，凭据由 `CredentialFileStore` 保存在私有主机文件；主体、租户、主机、服务和账号均参与作用域。不能把共享数据库元数据与私有凭据文件合成双写副本。
- Codex、xAI、Gemini 分别委托 `crates/vw-agent-apps/src/auth/{codex_refresh,xai_refresh,gemini_refresh}.rs`；Copilot委托 `copilot.rs`/`copilot_token.rs`。Claude的外部刷新吸收、AGY钥匙串账号与Kimi原生profile分别由 `claude_rotation.rs`、`agy_refresh.rs`、`kimi_account.rs` 处理。它们的服务协议与存储职责不同，不合并刷新实现。
- `auth_accounts/gemini.rs::gemini_access_token` 先确认同作用域受管元数据存在，再调用 `GeminiRefresh::access_token`，路由的 `app_routing/adapter_selection.rs` 消费该入口。它不是第二凭据写入器。
- `credential_lease.rs::read_for_refresh` 持有刷新锁并记录文件描述符及摘要；`save_refreshed` 在写入前重新检查代际。Gemini在网络等待时释放一般租约但保留刷新代际锁，发布时再领取租约，避免把网络等待与一般文件租约强合。其他服务保留已有并发和外部CLI轮换策略。
- `crates/vw-agent/src/auth/` 的平台认证与SDK供应商配置接口有不同的凭据、权限和生命周期，不能作为同账号刷新副本删除。

## 删除集合与来源

本域生产代码删除集合为空。正式包 `src/lib.rs` 不加载 `upstream`；旧 `upstream/src-tauri/src/services/{claude_account,agy_account,kimi_oauth}` 和 `store.rs` 以及 `upstream/src/lib/api/auth.ts` 是来源快照，不是上述Rust模块的第二运行路径。本任务不零散删除来源；整体归档/移除交给任务21、22。未发现本域需要切换的等价重复写入入口，不新增抽象或伪造失败测试。

前置证据沿用 `docs/vibewindow/agent-apps/auth/` 下的 `来源映射.md`、`串行化Codex令牌刷新.md`、`阻止撤销与刷新竞态.md`、`回归账号权限与主机隔离.md`、`回归凭据脱敏与失败诊断.md`、`切换与刷新Kimi账号凭据.md`。这些是历史交付，不能替代本轮验证。

## 核对与后续验证

本轮使用 RustCodeGraph `status`、`explore` 及精确文件 `node` 核对所有者/调用链，使用 `rg` 交叉查看SDK和模块边界；未初始化或更新索引，未执行构建。静态核对不证明并发刷新实际只产生一次结果，也不证明当前版本跨租户、失败回滚、日志脱敏行为通过。

保留现有 `auth/codex_refresh_tests.rs`、`auth/gemini_refresh_tests.rs`、`auth/xai_refresh_tests.rs`、`auth/logout_tests.rs`、`crates/vw-agent/src/agent_apps/auth_accounts_tests.rs` 与 `agent_apps_auth_security_tests.rs` 供以后统一定向验证。此次未新增测试，未产生业务夹具、前后落盘哈希或有效测试数。

回滚基线为任务1提交 `af965519d`；本任务只有本文，不改变运行行为或业务数据，无需恢复凭据。后续任何切换先记录独立脱敏夹具和具体切换提交，禁止整体重置工作区或启动新旧双写。
