# 任务22 静态交付证据

## 实现及调用链

- 既有 `memory_update::update/config` 已拥有完整记忆转换和配置规范化；继续复用，不新增算法。宿主 memory::update 仅映射 SettingsMessage、执行 ClearEmbeddingRouteFeedback 跨域效果、把配置交给 update_memory_config_async。
- 新增 `storage_update::prepare_save` 组合既有 update/config，拥有完整草稿校验、成功清错及失败反馈。宿主 storage::update 仅筛选消息、调用入口并把 provider.config 交给现有 update_storage_config_async，文件与服务端口不变。
- 保存回执：settings/mod.rs 原有 memory/storage 分支分别调用 save_feedback::memory_saved/storage_saved；无需修改公共路由。
- AppState 已持有领域 MemorySettingsState/StorageSettingsState；new.rs 已通过领域 app_init 构造，无需重复接线。
- 页面真实入口继续调用领域 system_settings_memory::view；宿主仅装配真实 embedding routes 子视图并映射局部 PageMessage。

## 宿主残留逐项理由

- system_settings_memory.rs：MEMORY_TABS/QDRANT_TAB/descriptor 为跨域设置搜索描述符适配，view 为嵌入路由跨领域视图装配，memory_tab_label 为现有调用者重导出；无页面绘制算法。
- memory.rs update：跨域反馈清理及配置异步效果端口；保留无关消息与实际 Task 调度契约。
- storage.rs update：配置异步效果端口；storage_tests 仅保留无关消息不触发本域更新的路由契约。
- memory_tests：无关消息契约、真实持久化 Task/Refresh 调度契约、embedding routes 清理契约。Refresh 纯状态断言移至领域。
- UI 来源仅保留 settings_tabs_core_appearance_memory 及其局部状态初始化：使用宿主通用 tab_appearance_tests 比较 GeneralSettings 的真实视图，属于跨域外观契约。没有保留其它页面绘制夹具。
- 未新增宿主文件。

## 断言归属映射

- memory_tests::update_ignores_unrelated_messages_and_refresh_clears_only_the_error：无关消息 backend/error 断言留宿主（改名 update_ignores_unrelated_messages）；Refresh backend/error 断言移至 host_module_22_tests::memory::refresh_clears_only_the_error。
- tab_selection_updates_only_visible_memory_section：所有可见 tab、error 保持、Qdrant 拒绝/允许/退回断言移至同名领域测试；路由 save_error/save_success 清理断言留宿主 embedding_routes_selection_clears_cross_domain_feedback，领域额外断言对应具名 Effect。
- memory_edits_schedule_persistence_while_refresh_and_unrelated_messages_do_not：保留宿主真实 Task.units 契约。
- storage_update_applies_each_field_and_clears_save_error、storage_save_accepts_blank_and_maximum_timeout、storage_update_rejects_invalid_and_overflowing_timeouts、valid_timeout_after_parse_error_clears_save_error：全部原字段、错误文本及解析边界断言移至 host_module_22_tests::storage 同名测试；持久化前校验直接调用真实 config，状态更新调用真实 prepare_save。
- non_storage_message_leaves_storage_state_unchanged：原两条状态断言保留宿主路由测试。
- UI 全组除上述 General 比较契约外，原测试名、中文、明暗主题、布局、输入事件、帮助文本、draw/screenshot、像素比较及复制断言全部移至 host_module_22_tests::ui（见下方用例清单）。夹具只持有三个真实领域状态、locale/theme；测试消息包装只适配真实 PageMessage 和 providers 消息，不包含 App 或重写算法。
- 与既有 desktop_split_29_tests 逐断言核对：该模块覆盖字段钳制、生命周期、缓存、SQLite/Qdrant 原始输入；本次覆盖 tab/Refresh/UI/保存校验，未重复迁入其五组断言。desktop_split_02_tests 为默认值，亦未复制。
- 新增 save_receipts_replace_only_the_owned_error 覆盖领域保存回执覆盖、成功清错及 memory/storage 隔离。

## 依赖与风险

新增仅 dev-dependencies：iced_runtime/image 用于原绘制与截图；desktop-assets 复用原字体；desktop-settings-providers 复用真实路由状态、更新与视图；desktop-chat-view 复用原可选择帮助文本。无库依赖宿主或反向 path/include，无生产依赖变化。Cargo.lock 仅手工同步本包依赖记录，未运行 Cargo。

源码核对发现既有 fields.rs/system_settings_memory.rs 使用公共控件根模块当前未导出的帮助接口；领域原有帮助视图与旧 UI 复制消息断言可能已有不一致。保留全部断言，未以禁测为由改写或删去；需后续授权编译和真实渲染确认。不能宣称字体整形无 panic 或编译通过。

## 静态检查与行数

只运行源码读取、独立 rustfmt 与 diff 检查，未运行任何 Cargo、测试、实机或绘制。全工作区 diff --check 曾报告其它任务 skills UI 文件末尾空行；本任务路径检查单独执行。

口径：Python pathlib.read_text().splitlines()，包含注释及空行。生产候选只是 baseline 文件名分类，领域保存反馈迁移单列，不将测试减少当生产减少。

| 来源 | 基线 | 当前 | 净减少 |
|---|---:|---:|---:|
| `crates/vw-desktop/src/app/components/system_settings_memory.rs` | 38 | 38 | 0 |
| `crates/vw-desktop/src/app/components/system_settings_memory_ui_refresh_20_tests.rs` | 1002 | 54 | 948 |
| `crates/vw-desktop/src/app/message/settings/memory.rs` | 17 | 17 | 0 |
| `crates/vw-desktop/src/app/message/settings/memory_tests.rs` | 100 | 65 | 35 |
| `crates/vw-desktop/src/app/message/settings/storage.rs` | 22 | 18 | 4 |
| `crates/vw-desktop/src/app/message/settings/storage_tests.rs` | 127 | 16 | 111 |

来源总计 1306 → 208，宿主净减少 1098 行；生产候选 77 → 73，净减少 4 行。生产职责迁出：存储草稿校验结果反馈分支及保存配置准备；其余生产入口已是必要跨域装配，未为行数目标额外搬壳。

UI 用例：
- `ui_refresh_20_narrow_tabs_stay_reachable`
- `ui_refresh_20_layout_and_interaction`
- `ui_refresh_20_preserves_values_and_disabled_states`
- `help_rollout_13_renders_contextual_help`
- `settings_help_gap_09_storage_fields_and_sections`
- `settings_help_gap_09_memory_sections`
- `settings_help_gap_09_storage_controls_and_validation`
- `settings_help_gap_09_memory_navigation_and_templates`

## 后续仅授权后执行

- `python3 scripts/cargo_slot.py cargo test -p desktop-settings-memory --lib host_module_22_tests -- --test-threads=1`
- 宿主跨域契约过滤：`python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib app::message::settings::memory::memory_tests -- --test-threads=1`；storage::storage_tests 同路径替换；UI 为 `app::components::system_settings_memory::ui_refresh_20_tests`。
- 宿主目标：`python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。

静态交付完成，未编译、未测试。
