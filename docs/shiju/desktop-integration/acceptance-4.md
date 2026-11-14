# 供应商页入口（任务 4）

2026-11-14：代码实现完成，未验证。按本轮用户覆盖要求未运行 Cargo、单元测试、实机测试或回归；未领取槽位，未设置 Cargo 缓存路径。

直接依赖 acceptance-2.md 已存在。供应商领域合同见 docs/shiju/agent-apps/provider-contracts.md，授权桌面交付见 docs/vibewindow/agent-apps/auth/接通供应商账号选择弹窗.md；这些历史证据不代表本次验证。当前共享 checkout 实际已有列表、编辑、预览、诊断及 SDK 方法，不重做 CRUD。

提供商设置页新增“管理应用供应商”，发送 SystemTabSelected(Apps)，复用 settings/tabs.rs 到 AppsCatalogMessage::EnsureLoaded 的目录加载。随后由应用卡片已有 ListMessage::Open(app) 调用 app_provider_catalog 和 app_provider_presentation_list，结果进入 ListState::finish 并由 app_providers/list.rs 展示加载、结果和错误。不新增 API 或权限分支。

全部 14 应用继续由真实目录与各页面能力分支声明，不将尚未验证的能力宣称不支持。按钮复用统一圆角样式，说明使用主题文字颜色；没有新表单或等宽中文。

未来统一验证：提供商页跳转、目录加载、各应用供应商列表成功及权限错误，以及 macOS 明暗主题渲染。本次未新增或执行测试，未声称验收通过。按用户要求删除编号任务卡。
