# 用量页能力收口（任务 6）

2026-11-14：实现交付，未验证。按本轮用户要求未运行 Cargo、单元测试、实机测试或回归；未领取槽位，无 CARGO_TARGET_DIR。

直接依赖已核对 acceptance-2.md 的领域导航。当前共享 checkout 有其他会话的未提交工作，原样保留。依赖来源版本沿用 acceptance-2 的 ca58c0313360eae7c851fbd7553c1614c7af03e7，当前实现基线 a32c2cad1。

用量页新增应用额度入口，使用既有 ViewMessage::OpenSystemSettingsTab 导航至 Auth 或 Apps。Auth 页面沿用授权账号读取、刷新及结果详情；Apps 页面由应用供应商卡片读取用量。真实结果链为 apps_usage_cards → apps_usage_events::read_snapshot → GatewayClient::apps_usage_snapshot → /v1/apps/usage/snapshot。刷新沿用现有 operation 及事件处理。没有放宽只允许设置页操作、主机/项目范围、凭据与过期响应检查。

母任务17统一请求账本仍在开发：已有 usage DTO 和扫描器源码不代表统一 API/SDK 已交付。本任务仅接入已有余额/套餐 SDK，不添加统一账本伪 API，不重做领域 CRUD。14应用的实际查询能力继续由 snapshot.capabilities 与网关错误决定，未逐个实测，不宣称支持或不支持。

未来统一验证需检查：两个导航按钮到达相应页面；已授权主机读取快照并展示详情；拒绝、刷新失败及切换主机后旧结果隔离；macOS 明暗主题和窄窗口布局。没有新增测试或声称验证通过。按用户要求删除任务卡。
