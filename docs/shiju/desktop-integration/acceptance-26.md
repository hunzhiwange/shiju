# 导航显隐偏好（任务 26）

状态：已有实现已核对，补充行为测试；未执行验证。日期：2026-11-14。

本轮用户要求只写代码，不运行 Cargo、单元测试、实机测试或最终回归；实现后删除任务卡。本文不代表验收通过。未领取 Cargo 槽位，未设置 CARGO_TARGET_DIR 或 CARGO_BUILD_BUILD_DIR。

## 范围与依赖

直接依赖任务 1 的接口盘点已交付，见 [acceptance-1.md](acceptance-1.md)。当前应用显隐入口是“应用 → 应用设置 → 可见应用与顺序”，不是设置侧栏折叠。无需新增 SDK、权限、依赖或配置存储。

源码版本沿用来源清单的 `ca58c0313360eae7c851fbd7553c1614c7af03e7`；实现核对以当前共享 checkout 为准，未重新校验来源哈希。

## 已有生产链路

- `system_settings_apps/preferences_view.rs` 使用统一设置弹窗和复选框样式展示显隐草稿。
- `preferences.rs` 的 `SaveUser` 调用 `agent_apps_preferences_update_authorized`，只在网关确认后替换保存值；拒绝时恢复草稿，保留至少一个可见应用。
- 网关 `agent_apps/preferences.rs` 将显隐和顺序存入租户及用户隔离的 `agent_apps_user` 设置表；主机终端偏好另存 `agent_apps_host`。不写应用原生配置或业务目录。
- 应用目录加载成功后发送 `PreferencesMessage::Load`，读取同一用户偏好；`visible_rows` 只投影已保存值，不删除被隐藏应用的目录记录。

已有链路满足本任务实现目标，因此未新增重复持久化路径或更改生产行为。

## 新增覆盖与未验证项

独立文件 `crates/vw-desktop/src/app/desktop_integration_26_tests.rs` 注册测试 `desktop_integration_26_preserves_expected_behavior`：覆盖新建桌面状态接收已保存 DTO 后的显隐和顺序、草稿隔离、拒绝后恢复、保留最后一个应用，以及业务目录和主机偏好不变。该测试仅覆盖状态重建，不将 DTO 序列化当作数据库重启验证。

数据库重开及租户/用户隔离已有 `cc_apps_preferences_api_scope_persists_and_isolates` 测试；拒绝路径已有 `cc_apps_preferences_api_scope_rejects_without_writing`。这些测试本轮均未运行。

后续统一验证需运行本任务定向测试及上述网关测试，并由用户在 macOS 检查显隐保存、重启恢复、拒绝保存和弹窗实际渲染。14 个应用的安装或运行能力不属于显隐偏好，本任务不作支持结论。
