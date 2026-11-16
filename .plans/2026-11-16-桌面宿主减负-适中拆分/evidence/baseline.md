# 宿主清单静态基线

日期：2026-11-16（Asia/Shanghai）。HEAD：`00a253d5e0f88a8f5bec927ed9251d83fa1e6780`。
初始 HEAD：`995c8028c`；取样期间其它任务推进了当前分支，以上 HEAD 为清单写入时观察值。本任务未切换分支。
初始状态：`## shiju...origin/shiju [ahead 3]`，工作区与暂存区干净。Rust：`rustc 1.99.1 (58cf8f9c3 2026-10-13)`（只查询 rustc 版本）。

## 验收覆盖与限制

用户明确要求任务1只做静态清单，不运行任何 Cargo 命令、单元测试、实机测试或最终回归。原命令 `python3 scripts/cargo_slot.py cargo test -p vw-desktop --lib -- --list` 未执行，退出码不适用；实际宿主测试总数、通过/失败/ignored 数均未知。`before-host-tests.txt` 是13个指定用例的源码候选清单，不是 libtest 输出，不可作为实际注册数或迁移收益证据。此静态交付完成后删除任务文件，不因禁测留待回归。

## 静态核对

`rustcodegraph status` 退出码0，索引12099文件。图导航不足以确认模块注册，补充精确源码搜索。Python 提取任务2至7的指定函数签名与相邻 test 属性（含 tokio::test），13个用例均在指定来源文件唯一命中；未审计全部领域断言。首次属性提取未识别带参数的 tokio::test，已修正提取规则后完成；这不是测试运行。

Cargo.toml 包名 vw-desktop，未显式配置 lib，默认入口 src/lib.rs。静态注册链如下，cfg/平台展开与编译均未验证：

- `lib.rs:38` → `app/mod.rs:10` → `components/mod.rs:195` → `system_settings_autonomy.rs:36-38`：test cfg 下 path 加载 `system_settings_autonomy_ui_refresh_22_tests.rs`，模块名 ui_refresh_22_tests。
- `lib.rs:38` → `app/mod.rs:10` → `components/mod.rs:220` → `system_settings_channels.rs:64-66`：test cfg 下 path 加载 `system_settings_channels_ui_refresh_17_tests.rs`，模块名 ui_refresh_17_tests。
- `app/mod.rs:47` → `message/mod.rs:29` → `chat/mod.rs:30` → `stream/mod.rs:11` → `stream_events.rs:1378-1380`：test cfg 下 path 加载 `stream_events_tests.rs`，模块名 stream_events_tests。
- 根 `lib.rs:45-47` 在 test cfg 下加载 tests.rs；`:49-50` 在 test cfg 下 include legacy_task_board_switches_exact_tests.rs，不计入所选13例。
- 所选三个测试文件均通过同目录 path 加载，无跨目录挂载；这不代表全仓审计。只读的领域入口中未发现反向 path/include 宿主源码；agents 入口加载自身相邻 system_settings_autonomy_tests.rs。
- `app/message/chat/processing_timing_tests.rs` 仅一行领域归属说明，已迁出，不重复计收益。

## 所选用例

|任务|源码候选完整名称|定义位置|
|---|---|---|
|2|`app::components::system_settings_autonomy::ui_refresh_22_tests::settings_help_gap_11_permissions_explanations`|`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs:846`|
|2|`app::components::system_settings_autonomy::ui_refresh_22_tests::settings_help_gap_11_channel_formats`|`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs:1009`|
|3|`app::components::system_settings_autonomy::ui_refresh_22_tests::settings_help_gap_11_list_editor_explanations`|`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs:928`|
|3|`app::components::system_settings_autonomy::ui_refresh_22_tests::ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids`|`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs:530`|
|4|`app::components::system_settings_channels::ui_refresh_17_tests::settings_help_gap_06_feishu_modes_keep_port_and_qr_controls`|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs:1245`|
|4|`app::components::system_settings_channels::ui_refresh_17_tests::settings_help_gap_06_dingtalk_keeps_allowlist_consequences`|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs:1301`|
|4|`app::components::system_settings_channels::ui_refresh_17_tests::settings_help_gap_06_ordinary_and_wildcard_lists_and_disabled_form`|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs:1363`|
|5|`app::components::system_settings_channels::ui_refresh_17_tests::ui_refresh_17_preserves_values_and_disabled_states`|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs:487`|
|5|`app::components::system_settings_channels::ui_refresh_17_tests::ui_refresh_17_edits_long_inputs_and_number_controls`|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs:607`|
|6|`app::message::chat::stream::stream_events::stream_events_tests::buffered_tool_event_starts_on_its_own_line`|`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:657`|
|6|`app::message::chat::stream::stream_events::stream_events_tests::active_workflow_update_appends_tool_block_and_preserves_message_id`|`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1174`|
|7|`app::message::chat::stream::stream_events::stream_events_tests::compaction_status_started_then_completed_updates_one_message`|`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:100`|
|7|`app::message::chat::stream::stream_events::stream_events_tests::compaction_error_and_cancel_settle_active_and_background_status`|`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs:1583`|

## 物理行数口径

按换行字节数统计（等价 wc -l），包含注释、空行和 cfg 排除内容，不等于编译归属。后续比较使用相同路径集合，并单列领域新增行数。

|路径|物理行数|
|---|---:|
|`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs`|1524|
|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs`|1451|
|`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs`|2219|

辅助总量：src 下递归 *.rs 共 1416 文件，180911 行，包含 bin 和未挂载文件，不能当作 lib 测试大小。
