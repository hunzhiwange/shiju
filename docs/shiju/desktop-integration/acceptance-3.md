# 应用页单应用探测入口（任务 3）

2026-11-14：实现交付，未验证。用户本轮明确禁止 Cargo、脚本验证、单元测试、实机测试和回归；没有领取槽位，没有测试通过证据。

依赖已读取 acceptance-2.md；沿用其来源版本 ca58c0313360eae7c851fbd7553c1614c7af03e7。应用领域现有目录、安装探测 API 与 SDK 已存在。14 应用由服务端目录声明驱动，不新增能力断言。商店状态 shiju_apps 与原生应用目录职责不同，因此修改真实所有者 system_settings_apps/details.rs 与 settings/apps_catalog.rs。

新增“重新探测此应用”入口，经 RefreshObservation 调用 GatewayClient::agent_apps_installations_authorized(host, id, token)，结果仅替换对应目录行的 observation，由现有 runtime_status、安装位置、版本与运行状态展示。探测错误继续由 error_label 和 toast 展示，不推断未安装或不支持，不做领域 CRUD。

复用网关、租户、项目、目标主机与目录代次检查；未连接时拒绝请求，目录加载期间禁用入口并串行化探测；主机或令牌切换后的响应被丢弃。重试不清空其它应用及详情，返回后使列表缓存过期，后续重新进入页面仍可刷新。沿用统一按钮样式，无新增表单或字体。

后续统一验证需覆盖成功结果替换、权限错误、迟到响应、重复点击及 macOS 明暗主题。没有新增测试文件或注册模块。实现完成按本轮要求删除编号任务卡，不将卡片删除视为验证通过。
