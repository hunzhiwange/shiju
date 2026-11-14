# 提示词规则唯一保存路径

2026-11-14，任务8。按用户本轮覆盖，仅静态审查，不运行 Cargo、构建、单元测试、实机或最终回归。以下不是行为验收通过声明。

## 当前调用链与所有者

- Iced `crates/vw-desktop/src/app/message/settings/prompts.rs` 的保存、删除与恢复通过 `agent_app_prompts_save` / `agent_app_prompts_delete` 调用 SDK，桌面不持有第二份模板数据库。
- SDK `crates/vw-gateway-client/src/client/agent_app_prompts_api.rs` 调用 `/agent-apps/prompts/save`、`delete` 等正式 HTTP 入口。
- `crates/vw-agent/src/gateway/api/handlers/agent_app_prompts_write.rs` 校验项目、主机与应用，分别调用 `PromptsStore::create/update/delete/copy_to_apps/import_*`。非 macOS 明确拒绝；目标路径来自执行端授权结果，不接受桌面传入任意路径。
- `crates/vw-agent/src/agent_apps/prompts_store.rs` 是目录权威，复用 `GatewaySettingsStore` 的 `agent_apps.prompts.v1` 命名空间。租户隔离由数据库提供，键区分项目、主机、应用；修改依赖 revision 与 compare_exchange/insert_if_absent。活动条目拒绝普通 update/delete，必须走激活或停用入口。
- `prompt_activation_store.rs` 扩展同一个 `PromptsStore`，`activate_ordinary` 校验 executor_host、获取应用文件锁，再经 `vw_agent_apps::prompts::activation` 和 `ProviderTransactions` 发布文件与目录 CAS。它不是第二份模板存储，不能删除其失败恢复与主机边界。
- `crates/vw-agent-apps/src/prompts/` 的目标解析、Pi 原生文件、模板和同步模块是目录向应用文件投影的不同职责，保留各应用文件语义。平台桌面设置接口 `desktop.rs` / `desktop_settings_api.rs` 不等同于应用提示词目录，不强行合并。

## 删除结论与恢复边界

本域本轮生产删除集合为空。正式包 `crates/vw-agent-apps/src/lib.rs` 明确将 `upstream` 排除在模块树和构建之外；来源 `upstream/src-tauri/src/services/prompt.rs`、`pi_prompt_files.rs`、`database/dao/prompts.rs` 与 TS API 属参考快照，不是正式运行调用链。整树归档和删除由任务21、22负责，不能在本域局部移除。

前置说明见 `docs/shiju/agent-apps/prompts/persistence.md`、`activation.md`、`write-api.md`、`desktop-editor.md` 和本目录 `准入与所有者.md`。来源母任务13；既有证据中的未测试事实保持不变。

## 静态证据与后续统一验证

RustCodeGraph status 显示索引存在，但 files --filter prompts 未覆盖当前正式提示词模块，因此使用 rg 与源码读取补足；未运行索引更新或构建。静态核对上述保存处理器、存储命名空间、CAS、激活事务及 SDK/Iced 调用。未发现同作用域第二权威写入路径，不为制造清理量新增业务或测试。

本轮没有修改生产代码、业务数据或生成脱敏夹具，故没有前后数据哈希和运行结果。恢复仅需逆转本文，不涉及数据库或应用文件。后续统一验证仍需覆盖中文保存/重读、重启一致性、跨租户与主机拒绝、revision 冲突、活动条目编辑、文件发布失败回滚，以及旧快照不被运行调用；当前不声称这些场景通过。
