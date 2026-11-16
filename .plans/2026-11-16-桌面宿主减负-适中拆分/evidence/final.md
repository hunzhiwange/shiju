# 迁移收益静态核对

2026-11-16（Asia/Shanghai），取样 HEAD `25a3c47b217cf7c2a2c0ba0bdb38d5e6b8d15434`。读取 baseline.md、前清单及 task-2.md 至 task-7.md，核对当前源码与注册。保持原目录、分支。

## 验证口径

用户覆盖验收：任务8只做静态清单，禁止全部 Cargo 命令、单元测试、实机测试和最终回归。本次全部未执行；退出码不适用，实际注册总数、passed/failed/ignored 均未知。前后清单都仅包含所选13项源码候选，非 libtest 输出，不能作为编译归属证明。按用户要求静态交付后删除任务8，不因禁测保留待回归。

RustCodeGraph status 成功，explore 未覆盖新增模块，故补充源码搜索。13个领域同名函数各唯一命中；12个宿主函数各唯一命中，仅任务5长输入函数已移除。函数搜索不等于 cfg 展开、类型检查或运行验证。

## 具名映射

完整原宿主名称见 before-host-tests.txt 和 after-host-tests.txt；领域候选为 crate 与模块路径，尚未经编译确认。

|任务|原用例|领域候选|结果|
|---|---|---|---|
|2|`settings_help_gap_11_permissions_explanations`|`desktop-settings-agents::host_split_2_tests::settings_help_gap_11_permissions_explanations`|部分迁出，宿主保留|
|2|`settings_help_gap_11_channel_formats`|`desktop-settings-agents::host_split_2_tests::settings_help_gap_11_channel_formats`|部分迁出，宿主保留|
|3|`settings_help_gap_11_list_editor_explanations`|`desktop-settings-agents::system_settings_autonomy::host_split_3_tests::settings_help_gap_11_list_editor_explanations`|部分迁出，宿主保留|
|3|`ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids`|`desktop-settings-agents::system_settings_autonomy::host_split_3_tests::ui_refresh_22_tool_menu_hover_scroll_and_add_keep_original_ids`|部分迁出，宿主保留|
|4|`settings_help_gap_06_feishu_modes_keep_port_and_qr_controls`|`desktop-settings-channels::host_split_4_tests::settings_help_gap_06_feishu_modes_keep_port_and_qr_controls`|部分迁出，宿主保留|
|4|`settings_help_gap_06_dingtalk_keeps_allowlist_consequences`|`desktop-settings-channels::host_split_4_tests::settings_help_gap_06_dingtalk_keeps_allowlist_consequences`|部分迁出，宿主保留|
|4|`settings_help_gap_06_ordinary_and_wildcard_lists_and_disabled_form`|`desktop-settings-channels::host_split_4_tests::settings_help_gap_06_ordinary_and_wildcard_lists_and_disabled_form`|部分迁出，宿主保留|
|5|`ui_refresh_17_preserves_values_and_disabled_states`|`desktop-settings-channels::host_split_5_tests::ui_refresh_17_preserves_values_and_disabled_states`|部分迁出，宿主保留|
|5|`ui_refresh_17_edits_long_inputs_and_number_controls`|`desktop-settings-channels::host_split_5_tests::ui_refresh_17_edits_long_inputs_and_number_controls`|完整迁出|
|6|`buffered_tool_event_starts_on_its_own_line`|`desktop-session::host_split_6_tests::buffered_tool_event_starts_on_its_own_line`|部分迁出，宿主保留|
|6|`active_workflow_update_appends_tool_block_and_preserves_message_id`|`desktop-session::host_split_6_tests::active_workflow_update_appends_tool_block_and_preserves_message_id`|部分迁出，宿主保留|
|7|`compaction_status_started_then_completed_updates_one_message`|`desktop-session::host_split_7_tests::compaction_status_started_then_completed_updates_one_message`|部分迁出，宿主保留|
|7|`compaction_error_and_cancel_settle_active_and_background_status`|`desktop-session::host_split_7_tests::compaction_error_and_cancel_settle_active_and_background_status`|部分迁出，宿主保留|

完整迁出1项，领域断言部分迁出12项，等价去重0项；12项保留宿主断言。领域新增13个源码候选，所选宿主13→12，净减少1。不能推广为整个宿主实际测试数或编译成本变化。

## 注册与生产接线

- agents：lib.rs → host_split_2_tests / autonomy_test_support；lib.rs → system_settings_autonomy → 同目录 path host_split_3_tests。两组非 wasm，任务2/3共用局部夹具。
- channels：lib.rs → host_split_4_tests / host_split_5_tests / channel_render_fixture，两组非 wasm，复用领域 Fixture。
- session：lib.rs → host_split_6_tests / host_split_7_tests / test_messages，均 cfg(test)。
- 宿主：lib.rs → app → components → system_settings_autonomy/system_settings_channels → 对应 ui_refresh 测试模块；app → message → chat → stream → stream_events → stream_events_tests。三个来源仍经各自父模块注册。
- 三个领域库源码和清单未发现反向依赖宿主或 path/include 加载宿主；已有 path 指向领域自身文件，vw_desktop 字符串命中为日志 target。无 App/AppState 复制。
- 宿主 autonomy view/map、channels panel/map 已接领域；工具队列调用新增 prepare_buffered_tool_line，工作流 upsert、压缩 upsert/settle 仍委托 desktop-session。静态核对未发现本轮漏注册、重复挂载或直接接线缺口，本任务无需修改 Rust。

## 逐组收益及剩余范围

- 任务2/3：4条重 UI 用例转为领域断言；宿主保留4条消息转换测试。领域源码保留真实字体、事件、布局与截图路径，尚未运行。
- 任务4：3组表单、权限说明断言迁出；宿主保留3条帮助 hover/pin/复制/关闭交互。CopyCode 是宿主消息，未扩张测试专用生产接口。
- 任务5：长输入/数字编辑用例整条迁出；另一条保留宿主页签点击、settings::update 回显。领域保留禁用、密码、编辑及布局截图断言。
- 任务6/7：4条仅迁文本、角色与索引投影断言；宿主保留请求路由、队列刷新、ID、前后台隔离和请求结束。App/请求夹具仍在；没有整体夹具减量结论。
- 宿主其它 UI 测试族、公共字体/渲染夹具、帮助卡、真实权限、存储和跨系统装配仍在，故宿主仍大。本轮不扩展领域重构。
- processing_timing_tests.rs 当前仅一行领域归属说明，为既有迁移，本轮收益0。
- task-4/task-5 记录的 desktop-ui-controls 既有导出/签名不匹配风险未验证、未跨域修复；静态注册不证明能编译或渲染。

## 物理源码辅助统计

与 baseline 相同换行字节口径，包含注释空行，仅三个来源文件；不代表编译归属或性能收益。

|来源|基线行数|当前行数|差值|
|---|---:|---:|---:|
|`crates/vw-desktop/src/app/components/system_settings_autonomy_ui_refresh_22_tests.rs`|1524|1229|-295|
|`crates/vw-desktop/src/app/components/system_settings_channels_ui_refresh_17_tests.rs`|1451|1113|-338|
|`crates/vw-desktop/src/app/message/chat/stream/stream_events_tests.rs`|2219|2221|+2|

领域新增测试和夹具行数另列如下（不含父模块及生产窄逻辑）：

- `crates/libs/desktop-settings-agents/src/host_split_2_tests.rs`：153行。
- `crates/libs/desktop-settings-agents/src/host_split_3_tests.rs`：195行。
- `crates/libs/desktop-settings-agents/src/autonomy_test_support.rs`：323行。
- `crates/libs/desktop-settings-channels/src/host_split_4_tests.rs`：275行。
- `crates/libs/desktop-settings-channels/src/host_split_5_tests.rs`：294行。
- `crates/libs/desktop-settings-channels/src/channel_render_fixture.rs`：165行。
- `crates/libs/desktop-session/src/host_split_6_tests.rs`：43行。
- `crates/libs/desktop-session/src/host_split_7_tests.rs`：37行。
- `crates/libs/desktop-session/src/test_messages.rs`：12行。

没有可比冷暖缓存编译数据；未清缓存、未宣称提速。以后允许验证时建议补领域13项与宿主12项定向验证、真实字体绘制及 libtest 清单。本次所有行为检查未执行。
