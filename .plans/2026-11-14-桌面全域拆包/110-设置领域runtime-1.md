# 任务 110: 设置领域runtime（切片 1/2）

先阅读并遵守 [公共执行规则](plan.md)。本任务无需读取 prompt.md。

批次：【批次 110】 依赖：任务 109（批次 109）

状态：未开始

目的：完成设置领域runtime的本片迁移与实际调用接线，缩小桌面 crate 的实现范围。

来源任务：用户要求尽可能全部拆分桌面包；文件覆盖见同目录源码归属清单。

预计会话范围：同一职责的窄切片，生产源码约 2155 行；只处理以下文件与必要调用/类型接线，不扩大为整个子系统迁移。较长现存文件先按实际符号分离职责，再迁移本片。

## 文件

目标：`crates/libs/desktop-settings-runtime`。普通领域目标新建最小 Cargo.toml、src/lib.rs、对应模块及独立测试；同领域前片已创建则扩展。保持来源子目录避免同名冲突。

来源及相邻测试：

- `crates/vw-desktop/src/app/components/system_settings_hooks.rs`
- `crates/vw-desktop/src/app/components/system_settings_hooks_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_hooks_ui_refresh_28_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_multimodal.rs`
- `crates/vw-desktop/src/app/components/system_settings_multimodal_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_observability.rs`
- `crates/vw-desktop/src/app/components/system_settings_observability_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_query_classification.rs`
- `crates/vw-desktop/src/app/components/system_settings_query_classification_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_reliability.rs`
- `crates/vw-desktop/src/app/components/system_settings_reliability_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_reliability_ui_refresh_21_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_runtime.rs`
- `crates/vw-desktop/src/app/components/system_settings_runtime_tests.rs`
- `crates/vw-desktop/src/app/message/settings/hooks.rs`
- `crates/vw-desktop/src/app/message/settings/hooks_tests.rs`
- `crates/vw-desktop/src/app/message/settings/multimodal.rs`
- `crates/vw-desktop/src/app/message/settings/multimodal_tests.rs`
- `crates/vw-desktop/src/app/message/settings/observability.rs`
- `crates/vw-desktop/src/app/message/settings/observability_tests.rs`
- `crates/vw-desktop/src/app/message/settings/query_classification.rs`
- `crates/vw-desktop/src/app/message/settings/query_classification_tests.rs`

接线修改：根 `Cargo.toml`、`crates/vw-desktop/Cargo.toml`、目标库清单、上述文件父模块和实际调用者。非 Rust 资源依照 include 宏与目录引用一并归属。

## 上下文与边界

迁移 runtime, reliability, observability, multimodal, query_classification, hooks, infrastructure 子页的领域状态、局部消息、更新和视图；拆出实际读取字段，网关写入通过 desktop-config；跨子页导航由壳处理，不传完整 App。

当前真实符号入口：field_row, enabled_toggled, command_logger_toggled, plugin_hooks_toggled, catalog_refresh_requested, audit_refresh_requested, trust_requested, number_control, bool_control, image_model_options, image_model_control, view, LabeledOption, fmt, view_overlays, slider_row, provider_retries_changed, provider_backoff_ms_changed, channel_initial_backoff_secs_changed, channel_max_backoff_secs_changed, scheduler_poll_secs_changed, scheduler_retries_changed, text_row, persist_hooks_settings, catalog_refresh_task, audit_refresh_task, trust_task, revoke_task, update, valid_ocr_model, multimodal_config_from_state, persist_multimodal_settings, normalize_observability_backend, normalize_runtime_trace_mode, persist_observability_settings, rules_for_config, rule_input_from_config, validation_message, validate。

当前跨桌面引用样例：crate::app::components::system_settings_common::{；crate::app::i18n::settings_extensions::text as tr；crate::app::message::settings::HooksMessage；crate::app::components::number_input::NumberInput；crate::app::message::settings::{MultimodalMessage, SettingsMessage}；crate::app::i18n::I18n；crate::app::i18n::settings_runtime::text as runtime_text；crate::app::assets::Icon；crate::app::{App, Message, message}；crate::app::session_gateway::{；crate::app::{App, Message}；crate::app::{App, Message, update_multimodal_config_async}；crate::app::{App, Message, state::QueryClassificationRuleInput}。这些是静态快照，先搜索确认，禁止按名字臆造共享类型。

## 步骤

1. 读取所列文件、父模块及直接调用者，列出本片字段、函数、状态、消息、资源和平台依赖；核对前片真实导出与已有未提交改动。
2. 处理本片对 App/全局 Message 的访问：领域状态归目标库，纯函数改为显式输入，视图/更新返回局部消息及真实必要效果；桌面保留映射与跨域编排。已解耦底层文件直接迁移，不为形式新增 context 或 trait。
3. 把本片实现移动到目标所有者，旧模块只保留窄适配/重导出；删除原有重复实现。首期已有库负责的能力只复用，不能复制一份。未迁移的同域文件通过适配继续工作，不创建库间循环。
4. 逐一处理 pub(crate)、私有兄弟模块访问、cfg、Task/Element 类型、资源相对路径与测试注册；仅提升真实跨 crate 调用所需可见性。保留原状态默认值、错误、权限和清理时序。
5. 源码搜索核对入口调用已经指向真实新实现；记录旧新路径/符号、最终测试位置及未运行项。按公共收尾规则提交后删除本文件。

## 静态审阅计划

这是结构迁移，TDD 运行步骤因用户禁令不适用。只迁移和必要适配已有测试源码，不运行、也不假造失败/通过结果。全局组合测试可留壳，本域断言随实现迁移；跨库私有测试采用正确模块组织，不能删断言或滥增生产 API。

后续统一验收范围：本片所列测试、实际被本片改动的直接调用者，以及相应目标的编译/平台集成；最终文件和真实测试名写入提交说明，由后续验收任务选定命令。

## 验证与交付证据

运行目录：`/Users/xiangmin/worker/code/shiju`。只运行轻量搜索和差异审阅：

```sh
rg -n 'crate::app::|vw_desktop|vw-desktop' crates/libs/desktop-settings-runtime
git diff --check
git diff --stat
```

逐一审查搜索命中，不把 rg 零命中退出码 1 当作失败或编译通过。另查暂存差异，防止提交无关改动。所需证据：本片迁移映射、局部状态/消息边界、旧调用接线、测试注册、cfg/资源审阅及 diff 检查结果。新库不得反向依赖桌面；壳适配不得残留本片业务副本。明确报告未运行 Cargo、单元测试和实机验收。
