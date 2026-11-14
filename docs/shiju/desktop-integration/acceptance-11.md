# 工作区页入口收口（任务 11）

2026-11-14：仅完成代码实现与静态调用链核对，未运行 Cargo、脚本测试、单元测试、实机测试或最终回归；用户要求未来统一验证。未领取 Cargo 槽位，缓存路径不适用。编号任务卡按用户要求删除，不代表运行验收通过。

## 修改与直接依赖

项目侧栏展开、折叠两态新增“工作区”按钮，打开既有应用列表设置页。现有“识矩应用”按钮触发 ShijuAppsMessage::Open，归另一页面，不能替代此原生应用目录入口。本次不复制领域 CRUD，不新增 API、权限或自动业务写入。

已核对直接依赖任务 2 对应的真实类型、SDK 和桌面基础：`vw_api_types::agent_apps::AppId`、`GatewayClient::agent_apps_catalog_authorized`、`agent_apps_installations_authorized`、`AppsCatalogMessage::EnsureLoaded`。共享 checkout 基线核对时为 `1e523c035`；领域来源版本与唯一所有者参见 [工作区契约](../agent-apps/workspace-contracts.md) 和 [专属入口核对](../agent-apps/workspace-special-entrypoints.md)，不重签历史验收结论。

## 入口到 SDK 结果

`views/project/layout/sidebar.rs::workspace_rail` → `ViewMessage::OpenSystemSettingsTab(SystemTab::Apps)` → `SettingsMessage::SystemTabSelected` → `AppsCatalogMessage::EnsureLoaded` → `system_settings_apps::catalog::load` → 已授权 SDK 目录与安装状态查询 → 现有目录列表及错误展示。

连接缺失时沿用本机连接或连接弹窗；无网关、未授权、远端离线、响应无效沿用原错误契约。目录加载仍校验网关、项目、主机与请求代次，不在导航层扩大授权。OpenClaw 详情已有 `workspace_files::entry`，弹窗已有 SDK read/write/open-directory 与结果展示；本次只补项目页到其所属设置页的入口。

14 应用身份已从真实枚举核对：Claude、ClaudeDesktop、Codex、Gemini、GrokBuild、OpenCode、OpenClaw、Hermes、Pi、Mcode、Kimi、Dsh、Agy、Omp。目录按服务端返回结果展示，身份注册不代表每种动作均支持；专属能力仍由各领域声明和实现拥有，未验证项不标为不支持。

按钮复用主题感知的 rounded_action_btn_style，中文用默认字体，没有新增表单或滚动区域。后续统一验证应覆盖两态导航、目录 SDK 结果和拒绝路径，以及 OpenClaw 文件入口和中文实际渲染。
