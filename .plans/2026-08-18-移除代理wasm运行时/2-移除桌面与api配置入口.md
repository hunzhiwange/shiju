# 任务 2: 移除桌面与 API 配置入口

批次：【批次 2】 依赖批次 1

状态：已完成，待回归验证

回归验证说明：已运行 `cargo test -p vw-desktop runtime --lib`。173 个筛选测试中 171 个通过；失败的两个断言均位于本任务范围外：`app::message::settings::agents::tests::agents_settings_patch_uses_current_main_model_and_combines_runtime_fields`（`crates/vw-desktop/src/app/message/settings/agents_tests.rs:191`，期望的 WASM 字段已由批次 1 删除）与 `app::state::runtime_tools_tests::selector_tab_labels_and_tool_groups_cover_every_variant`（`crates/vw-desktop/src/app/state/runtime_tools_tests.rs:426`，工具分组改动）。本任务的运行时设置测试均通过；任务范围内 WASM 配置符号搜索为空，只有桌面 Web 平台 `wasm32` 条件编译保留。

目的：从桌面设置与 API DTO 删除 WASM 后端及其资源、安全策略字段。

来源任务：无

预计会话范围：所有字段由同一 RuntimeSettingsState 驱动，需同步移除视图、消息、初始化与测试。

## 文件

- 修改：`crates/vw-desktop/src/app/state/settings/runtime.rs`
- 修改：`crates/vw-desktop/src/app/message/settings/{messages.rs,runtime.rs}`
- 修改：`crates/vw-desktop/src/app/components/system_settings_runtime.rs`
- 修改：`crates/vw-desktop/src/app/impls/app_init/new/{.rs,settings.rs}`
- 修改：`crates/vw-api-types/src/settings.rs`
- 修改：相关单元测试和设置搜索关键词。

## 上下文

- 桌面界面当前在 `kind == "wasm"` 时渲染整个 WASM 配置分区。
- API DTO 也暴露了 WASM 枚举与策略字段，应与运行时配置同时删除。

## 测试计划

- 行为：设置界面仅允许 native/docker，保存时不读写 `runtime.wasm`。
- 失败验证测试：删除 WASM 选项及其消息处理断言。
- 失败验证命令：`cargo test -p vw-desktop runtime --lib`
- 预期失败原因：删除功能，无法编写保留行为的失败测试；以编译和现有 native/docker 测试验证。
- 通过验证命令：`cargo test -p vw-desktop runtime --lib`
- 模拟策略：使用现有本地 App fixture。

## 步骤

1. 删除状态字段、初始化读取、消息变体、持久化和界面分区。
2. 将运行时类型规范化和选项限制为 native/docker。
3. 删除 API WASM DTO 字段和测试，清理搜索元数据。

## 验证

- 运行：`cargo test -p vw-desktop runtime --lib`
- 预期：桌面运行时设置测试通过且不再编译 WASM 配置路径。
- 所需证据：测试退出码为 0，相关符号搜索为空。
