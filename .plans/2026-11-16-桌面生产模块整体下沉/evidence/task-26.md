# 任务26：外观通用与编辑器设置

静态交付完成；遵照禁测要求，未运行 Cargo、测试、绘制或回归。独立 rustfmt 仅用于修改文件的语法/格式检查，不表示类型检查通过。

## 生产归属与调用链

- general.rs 的通用状态分支调用 general_update：页签及画廊错误清理、时区白名单、语言即时选择与保存合并、通知保存合并、开机启动忙碌及回读结果。原字段使用局部可变借用，不引入 App 副本或反向依赖。
- 通用 view → appearance::general；编辑器 view/content → appearance::editor → desktop_editor::components::editor_settings_fields。编辑器参数更新保留 desktop-editor 原领域。
- 快捷消息 → quick_messages_update::update → Save/Toast 效果；主题 → system_settings_themes::update → ApplyTheme/Toast 效果。旧算法没有保留副本。
- 宿主 general.rs 保留系统开机启动、托盘/单实例/深链、通知权限和发送、语言变化对各领域的刷新、持久化 Task 与全局字段补丁协调。均依赖真实系统资源或跨域装配。
- tabs.rs 为系统导航与各领域加载的跨域分发；本任务未更改其算法。
- 宿主 general/quick_messages/themes 适配器仅装配持久化、toast 和跨域效果；UI 宿主仅做 App 字段投影、帮助渲染函数注入和旧消息映射。

## 测试与断言映射

- host_module_26_tests 独立注册；ui_fixture 仅含当前页面的局部值和借用视图装配，调用真实领域绘制/更新/主题存储，不构造 App、不包含配置更新算法。
- 编辑器 UI 的系统设置外壳截图改为领域 editor_overlay + gallery 真实弹窗装配；原截图、draw、字体加载、键鼠事件和布局断言保留。全局导航字段设置不属于领域绘制，已去除。
- 通用设置测试中的 helper 选择断言留在宿主验证跨域 Message 映射，其余页面构建、中文、布局、上下文帮助、tab 外观与点击断言迁至领域。预览弹窗使用 desktop-editor 的原 build_settings_overlay。
- theme/quick_messages 断言改验真实领域状态及具名效果；toast 文案用同一 I18n 格式化检查。没有 mock 被测算法。
- general_tests 的页签两例移入领域；其余为 bootstrap 配置恢复、平台效果任务、时区合并补丁、通知权限与全局反馈契约，保留宿主。其开机启动失败用例已有 busy=false 断言与生产失败后回读 busy=true 不一致，本轮保持原断言，后续授权验证时需审查，未宣称其通过。
- 前轮 desktop_split_16_tests 的主题编辑小控件与纯选项断言保留原归属，本轮只迁此前仍在宿主的页面/原生绘制测试；逐用例名核对未复制同名整组断言。

| 来源 | 基线 | 当前宿主 | 保留职责 |
|---|---:|---:|---|
| system_settings_editor.rs | 21 | 25 | 跨域装配/投影/契约 |
| system_settings_editor_tests.rs | 67 | 0 | 已迁领域并删除旧挂载 |
| system_settings_editor_ui_refresh_05_tests.rs | 926 | 0 | 已迁领域并删除旧挂载 |
| system_settings_general.rs | 96 | 124 | 跨域装配/投影/契约 |
| system_settings_general_tests.rs | 1037 | 72 | 跨域装配/投影/契约 |
| general.rs | 389 | 413 | 跨域装配/投影/契约 |
| general_tests.rs | 231 | 200 | 跨域装配/投影/契约 |
| quick_messages.rs | 21 | 18 | 跨域装配/投影/契约 |
| quick_messages_tests.rs | 44 | 0 | 已迁领域并删除旧挂载 |
| tabs.rs | 340 | 340 | 跨域装配/投影/契约 |
| themes.rs | 44 | 41 | 跨域装配/投影/契约 |
| themes_tests.rs | 64 | 0 | 已迁领域并删除旧挂载 |

基线同口径 splitlines：来源 3280 → 1233，宿主净减 2047 行；生产候选 911 → 961，净减 -50 行（格式化及具名参数装配造成候选行数增长，不将测试减量算成生产减量）。本任务没有新增宿主文件。

### 原用例归属清单

- `view_uses_app_theme_when_following_system_theme` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor.rs`
- `view_uses_editor_theme_when_not_following_system_theme` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor.rs`
- `view_default_values_does_not_panic` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor.rs`
- `view_with_min_boundary_values` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor.rs`
- `view_with_max_boundary_values` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor.rs`
- `ui_refresh_05_preserves_values_and_disabled_states` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `ui_refresh_05_layout_and_interaction` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `ui_refresh_05_gallery_uses_original_theme_id_and_paging_messages` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `ui_refresh_05_image_fit_keeps_canonical_value` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `ui_refresh_05_theme_name_and_color_picker_keep_original_messages` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `settings_help_gap_02_gallery_rules_and_card_status` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `settings_help_gap_02_gallery_opens_builtin_and_preset_editors` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `settings_help_gap_02_creation_help_keeps_validation_and_picker` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/editor_ui.rs`
- `system_settings_general_tests_are_wired` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `setting_row_builds_text_and_control_columns` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `selection_helpers_preserve_theme_shell_and_terminal_theme` → `crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- `terminal_font_helper_preserves_empty_and_named_families` → `crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- `auto_save_helper_preserves_every_supported_mode` → `crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- `timezone_helper_preserves_supported_iana_timezone` → `crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- `view_builds_each_general_settings_tab` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `font_size_helper_preserves_slider_boundaries_and_fraction` → `crates/vw-desktop/src/app/components/system_settings_general_tests.rs`
- `view_builds_supported_general_setting_combinations` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `general_settings_render_chinese_in_light_and_dark_without_overflow` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `help_rollout_03_terminal_renders_contextual_help` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `help_rollout_03_preview_renders_contextual_help` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `help_rollout_03_editor_renders_contextual_help` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `help_rollout_03_themes_render_contextual_help` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `settings_tabs_general_appearance` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general.rs`
- `tab_selection_updates_general_settings_content` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general_update.rs`
- `theme_tab_selection_clears_error_and_refreshes_gallery` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/general_update.rs`
- `timezone_selection_updates_immediately_and_persists_last_choice` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `desktop_language_selection_survives_reload` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `desktop_language_restores_and_reports_save_failure` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `login_launch_failure_preserves_previous_state_and_unlocks_control` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `tray_quit_starts_shutdown_even_when_close_to_tray_is_enabled` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `notification_settings_restore_and_report_delivery_failure` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `notification_setting_serializes_rapid_toggle_saves` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `notification_permission_failure_disables_and_persists_notification_setting` → `crates/vw-desktop/src/app/message/settings/general_tests.rs`
- `quick_message_settings_save_validates_content_and_reports_result` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/quick_messages.rs`
- `themes_apply_custom_updates_app_theme` → `crates/libs/desktop-settings-appearance/src/host_module_26_tests/themes.rs`

## 依赖与未验证风险

- 新增的依赖仅为迁移原生测试所需 dev-dependencies：desktop-editor、desktop-terminal、desktop-chat-view、desktop-assets、iced-code-editor、iced_runtime、image、serde_json、tokio；生产库依赖方向未扩大。Cargo.lock 仅同步已有包名，不解析依赖。
- 已静态核对 mod/cfg、旧挂载删除、调用者与无 path/include 反向引用；类型、运行、字体整形与像素结果均未经验证。
- 以后授权时：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-appearance --lib host_module_26_tests -- --test-threads=1`；宿主 `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`。本轮未执行。
