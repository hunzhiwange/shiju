# 任务18：通道设置整组静态交付

已迁移并真实接线；未编译、未测试、未运行绘制。遵守本轮禁测要求。

## 生产归属与调用链

宿主 system_settings_channels::view → desktop_settings_channels::view::view(ViewState) → 各通道面板 → overview::view。21 通道、Global、Email wasm 回退原样保留。
App 已持有领域 ChannelsSettingsState；初始化与 update::updates 的真实装配既已存在，无需新增状态或修改共享 App/message 文件。

领域 common/overview/enterprise_channels 的帮助入口原先引用公共控件中不存在的宿主专用签名，本次改为领域 help 模块：复用 desktop-chat-view 的 selectable_titled_text、desktop-ui-controls 的 SideOverlay 与表单布局。保留悬停、固定、Escape、选择、右键复制/追加/搜索，无平行算法。
Message 增加 CopyCode/Append/OpenUrlExternal，由宿主 map_message 转接既有剪贴板、输入追加与打开链接行为。

新增依赖 desktop-chat-view 为已有生产控件，不新增外部包。源码检查其依赖不回指 channels/宿主；Cargo.lock 仅补本包依赖边，无 Cargo 命令。

## 来源残留理由

- system_settings_channels.rs：view_state 借用 App 的三个既有字段；map_message 映射跨域效果；channel_tab_descriptors 转发搜索元信息；view 映射宿主消息。子模块注册仅为既有适配器/模态边界及其既有测试保留，不复制算法。
- system_settings_channels_tests.rs：整文件迁出并删除旧挂载。
- ui_refresh_17_tests：只保留 gateway_client_edits_host_and_masked_key（跨网关设置域）与 settings_tabs_capabilities_appearance_channels（和 General 等页面的实际绘制外观对照）及其必要字体、布局/输入、截图、隔离工具；无通道帮助夹具。原通道 fixture 收缩为 App::new，无领域字段初始化。

## 断言映射

以下每项旧测试的全部原断言保持，除了 App→局部 Fixture、view(&App)→view(ViewState)、生产更新→同一领域 update::updates、搜索效果→局部消息；不改预期值。字体、中文、暗黑、真实 draw/screenshot、二维码与默认项目代际隔离代码均保留。

- `view_builds_empty_enabled_channel_summary` → `host_module_18_tests`，保留本函数全部断言。
- `channels_error_banner_follows_channel_tabs` → `host_module_18_tests`，保留本函数全部断言。
- `view_builds_cli_enabled_channel_summary` → `host_module_18_tests`，保留本函数全部断言。
- `view_builds_multi_channel_enabled_summary` → `host_module_18_tests`，保留本函数全部断言。
- `ui_refresh_17_renders_channel_forms` → `host_module_18_tests::ui`，保留本函数全部断言。
- `ui_refresh_17_layout_and_interaction` → `host_module_18_tests::ui`，保留本函数全部断言。
- `ui_refresh_17_gateway_client_edits_host_and_masked_key` → `宿主原跨域契约`，保留本函数全部断言。
- `ui_refresh_17_preserves_values_and_disabled_states` → `host_module_18_tests::ui`，保留本函数全部断言。
- `ui_refresh_17_preserves_project_binding_and_qr_actions` → `host_module_18_tests::ui`，保留本函数全部断言。
- `settings_tabs_capabilities_appearance_channels` → `宿主原跨域契约`，保留本函数全部断言。
- `help_rollout_11_renders_contextual_help` → `host_module_18_tests::ui`，保留本函数全部断言。
- `settings_help_gap_06_feishu_modes_keep_port_and_qr_controls` → `host_module_18_tests::ui`，保留本函数全部断言。
- `settings_help_gap_06_dingtalk_keeps_allowlist_consequences` → `host_module_18_tests::ui`，保留本函数全部断言。
- `settings_help_gap_06_ordinary_and_wildcard_lists_and_disabled_form` → `host_module_18_tests::ui`，保留本函数全部断言。

前轮去重核对：host_split_4 保留字段权限说明/端口与用户编辑、host_split_5 保留表单宽度、密码遮罩、禁用与数字/长输入行为；本组覆盖完整页面路由、总览错误横幅、21通道、多语言帮助悬停/固定/关闭与右键菜单、项目候选和二维码。共有的标签选择消息断言用于不同场景的完整行为链，未复制前轮大型 fixture。新组共享 channel_render_fixture 的真实渲染/事件/布局工具，删除迁入时重复工具。

## 基线统计

按 pathlib.read_text().splitlines，与 baseline.json 同口径；本任务无新建宿主文件。

| 来源 | 基线 | 当前 | 净减少 |
| --- | ---: | ---: | ---: |
| `system_settings_channels.rs` | 66 | 53 | 13 |
| `system_settings_channels_tests.rs` | 89 | 0 | 89 |
| `system_settings_channels_ui_refresh_17_tests.rs` | 1113 | 334 | 779 |

宿主总净减少 881 行。生产候选文件单列见第一行；其迁出的是页面选择分支，新增跨域映射使实际行数净减较小，未将测试减少计作生产减少。

## 静态验证与限制

已读 mod/cfg 与直接调用者；无反向 include/path 生产加载，无宿主依赖。独立 rustfmt 处理新增文件及宿主入口，未调用 Cargo。
任务文件范围 git diff --check 通过；全工作区检查另见其它并行任务的尾部空行，不修改它们。
尚未验证 Rust 类型、字体整形、实际绘制或运行行为；不声称通过。

未来仅用户另行授权后运行：
`python3 scripts/cargo_slot.py cargo test -p desktop-settings-channels --lib host_module_18_tests -- --test-threads=1`
宿主跨域用例过滤：`app::components::system_settings_channels::ui_refresh_17_tests`（包 vw-desktop、--lib）。宿主 check 留待用户授权的统一验证。
