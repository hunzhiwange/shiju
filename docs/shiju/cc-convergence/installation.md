# 安装更新唯一执行路径

2026-11-14，静态核对基线 `e6ba984b13bffb2eb59d056e086b11845680cb82`。本轮按用户覆盖要求不运行 Cargo、构建、单元测试、实机或最终回归；本文不是行为通过声明。

## 所有者与调用链

- 应用生命周期：`crates/vw-desktop/src/app/message/settings/apps_lifecycle.rs` 通过 `agent_apps_lifecycle_api.rs` 的 `agent_apps_action_preview_authorized`、`agent_apps_action_submit_authorized` 调用网关。SDK 对不确定提交结果只读取同一 operation，不自动重放写入。
- 网关 `crates/vw-agent/src/gateway/api/handlers/agent_apps/lifecycle.rs` 持有预览及运行操作，校验租户、目标主机、预览时效、确认与写入环境；消费预览后由 `Operation::start` 执行，排队后重新授权。读取、长轮询和取消复用同一操作 ID，取消等待工作结束。
- 执行端 `crates/vw-agent-apps/src/apps/lifecycle/local.rs::{preview,run}` 使用主机观察绑定安装身份；升级配方由 `update.rs::plan` 生成，`UpdatePlan::revalidate` 拒绝安装或命令漂移。`coordinator.rs::ToolLifecycleCoordinator` 持有应用写入协调边界，`process.rs` 执行命令、收集结果并支持操作上下文。卸载另走其受控执行器，不能作为重复安装路径删除。
- ShiJu 桌面自身更新：`app/message/view/panel/settings.rs` 调用 `installer_update.rs::{prepare_desktop_update,open_desktop_installer}`。准备阶段经共享下载器校验安装包，打开阶段重新校验后交给 `desktop_update_helper`；macOS 应用包替换由 `desktop_update_macos.rs` 承担。`app/desktop_update.rs` 持有当前版本快照与界面状态，不是另一执行所有者。
- 桌面发布元数据由 `crates/vw-shared/src/update/desktop_release.rs` 解析；来源是 ShiJu 官方 manifest 或 `hunzhiwange/shiju` GitHub releases，应用生命周期更新配方不使用 CC Switch 桌面品牌更新源。

## 保留与零删除依据

`crates/vw-agent/src/installation/mod.rs` 识别代理自身 CLI 的安装渠道并执行该渠道升级，与外部应用安装、桌面应用包更新具有不同目标与权限。`crates/vw-agent/src/update.rs` 仅重新导出 `vw_shared::update`，不是复制更新实现。不得把这些入口强合。

`crates/vw-agent-apps/src/lib.rs` 明确只注册正式领域模块，`upstream` 不属于库模块树或构建。来源 Tauri 初始化与 `settings.ts` 是只读来源，不是本域当前第二执行端；整树归档和删除留给任务21、22。本次未发现同权限同目标的失效运行副本，因此生产代码及业务数据零修改、删除集合为空，不制造无调用者证据或测试红灯。

前置索引：`准入与所有者.md`；应用安装已有历史交付记录见 `docs/shiju/agent-apps/local-setup.md`。历史测试/截图不升级为当前基线已验证。

## 静态证据、恢复及后续验证

使用 RustCodeGraph status/query/node 读取安装更新模块、SDK 与执行配方；对图未返回的生命周期文件使用 rg 和文件读取。`rg -n 'prepare_desktop_update|open_desktop_installer' crates/vw-desktop/src --glob '*.rs'` 确认生产调用来自设置消息入口；`rg` 核对 Iced 授权预览/提交以及正式模块树，均退出0。探索命令返回大量不相关来源符号，因此不将宽查询计数当作运行调用者数。

无切换、无业务夹具、无落盘或事件对照结果，未领取 Cargo 槽位、未设置构建缓存路径。只新增本证据，恢复时仅逆转本证据提交；保留其他工作区改动及来源快照。以后统一验证应覆盖提交一次与不确定响应恢复、跨租户/主机拒绝、漂移预览、取消与进度，以及 macOS 桌面更新签名/摘要和助手失败恢复，不能用静态检查代替。
