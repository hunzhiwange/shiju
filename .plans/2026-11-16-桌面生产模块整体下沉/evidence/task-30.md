# 任务30：设置外壳与帮助控件静态交付

状态：静态职责收口完成。未编译、未测试、未运行原生 UI、未执行回归；不能据此推断构建或绘制通过。原局部提交018c13ae4由本次后续实现补齐，早期未完成清单以本记录为准。

## 生产归属与真实调用链

- 宿主 message/settings/messages.rs 只特化 desktop-settings-shell::messages::SettingsMessage；380个事件变体及领域消息重导出只在 shell 定义。四个具名泛型实参分别为现有 SettingsTab、搜索目标、AppsCatalogMessage、AppProviders ListMessage，承接真正的跨域宿主信封；没有复制 App、引入动态派发或新的能力开关。
- SettingsMessage::route 消费事件，按原56个匹配分支逐项返回 SettingsRoute。所有380个变体均在路由源码出现，未增加默认丢弃分支。宿主 settings::update 只组装目标控制器、配置端点、启动设置编辑生命周期与通知效果；原冗长消息分类规则从宿主删除。每个路由携带原始载荷或原消息，无克隆或类型擦除。
- types.rs 的任务结果重导出归 shell::types；宿主保留稳定导出路径。
- 帮助卡的选择文本、上下文帮助、三个帮助标题/字段组合与本地化弹窗全部使用 desktop-ui-controls 的真实实现。宿主 help.rs 只传 locale、last_copied_code_hash、Message::CopyCode 及关闭消息；panels.rs 只重导出共享控件。领域可直接使用共享实现或注入明确类型的 renderer，无需依赖宿主函数。
- SystemTab、GeneralSettingsTab、分组和关键字定义下沉至 desktop-ui-controls::settings_navigation。shell 继续原路径重导出。general_fields 归 appearance（真实使用者及 editor 字段依赖所在层），shell 亦重导出。appearance/network/navigation 的直接调用者和对应夹具引用新基础路径，删除三者对 shell 的反向依赖。原有 key、枚举身份和页面分组保持。
- 移动端外壳布局与重试说明、tabbed_page_owns_save_error 规则归 shell::layout。宿主提供当前页面、帮助按钮与跨域返回/刷新消息；样式、留白与文字保持。
- 原帮助、面板和导航基础定义没有复制两套；兼容路径均为重导出/最小消息投影。

## 来源残留逐组理由

| 宿主残留 | 必要装配理由 |
| --- | --- |
| system_settings::view、settings_content、settings_shell | 连接多个独立领域视图、App窗口可见性、移动端生命周期、macOS链接预览及原生跨域浮层；具体领域页面均调用原领域实现，移动端UI已归shell。 |
| active_tab_help_modal_open、active_settings_help_tab、active_tab_save_error | 在当前系统页与智能体子页之间投影多个领域的帮助/错误状态；不改变领域状态或持久化策略。 |
| system_tab_matches_query（仅宿主测试） | 跨多个领域描述符目录验证整体导航搜索契约；不拥有生产搜索算法或字段目录。 |
| settings::apply_agent_config_saved | 按跨域保存来源把网关错误交给各领域 save_feedback；不重做领域状态转换。 |
| settings::update_config_panel | 保留启动时编辑标记生命周期与 App 持有的控制器入口，随后直接委派。 |
| settings::update、reload_agents_config_task | 把shell路由结果、网关/原生任务与App装配，wasm分支保持。 |
| settings/util::load_gateway_providers、wasm_save_task | 获取宿主管理的网关连接/生命周期任务；实际加载、解析及同步工具在providers库。 |
| help.rs | locale、复制状态和宿主复制/关闭消息投影，不含帮助绘制或本地化策略。 |
| common/mod.rs与icons/inputs/panels/select/styles/tabs/theme/utils | 稳定兼容导出与宿主测试注册，无独立领域实现。 |
| messages.rs、types.rs | 跨域消息类型特化和稳定导出，不持有业务枚举或转换。 |
| tab_appearance_tests.rs | 仅隔离网关资源、进程环境、字体初始化和两个测试接口的App投影；所有几何、像素、绘制、操作及截图实现归shell测试夹具。 |
| system_settings_ui_refresh_04_tests.rs | 保留实际App更新、跨页导航、编辑器/终端/预览事件、焦点、网关重试与浮层装配契约；通用Scene、字段定位、滚动、选择复制和截图实现已归shell。 |

宿主没有新增状态字段或替代文件；沿用既有领域状态所有权，无额外AppState初始化变化。

## 测试与断言逐组映射

所有既有名称保留。共享绘制夹具在 shell 的 host_module_30_tests 独立注册；宿主测试通过正向path加载领域拥有的通用测试夹具，只有宿主测试接口实现读取App，领域源码无任何宿主path/include或App引用。测试专用接口只含locale/general_view与消息投影，不进入生产依赖API。

| 原测试/夹具 | 新归属及保留的证据 |
| --- | --- |
| help_modal_fills_overlay_for_default_and_copied_states | shell::host_module_30_tests；初始/已复制2个子节点断言均保留；去掉App::new，输入真实locale与哈希。 |
| help_rollout_01_renders_contextual_help | shell::host_module_30_tests::help_ui_refresh_03_tests；中英繁体/明暗/窄屏、帮助内容只在卡内出现、悬停开闭、原选择器及Asia/Tokyo事件。 |
| ui_refresh_03_layout_and_interaction_keeps_actions_fixed | 同上；卡片/滚动条/标题与底部像素、选择器、Esc/取消/保存/背景交互，真实draw/screenshot全保留。 |
| ui_refresh_03_preserves_values_and_disabled_states | 同上；行居中、水平间隔、禁用切换及截图。 |
| ui_refresh_03_renders_localized_help_and_keeps_copy_and_close | 同上；使用真实共享本地化弹窗，三种语言、长内容、复制原文与关闭消息。 |
| ui_refresh_03_renders_model_draft_and_error_with_original_actions | 同上；使用真实providers模型弹窗和ProviderSettingsState，不复制算法；保留长ID/繁体草稿/错误、背景不丢草稿、取消及保存。 |
| tab_appearance_tests 的 Labels、TabAppearance、TabPixels、press/fill/edge/assert_pixel | shell::host_module_30_tests::tab_appearance；原27个断言宏在通用夹具+宿主资源契约中仍为27个；所有渲染/输入/滚动/截图方法保留。 |
| ui_refresh_04_layout_and_interaction | 宿主跨域契约；真实外壳的侧栏、页面、帮助与Iced输入分派；通用Scene归shell。 |
| ui_refresh_04_preserves_values_and_disabled_states | 宿主跨域契约；App搜索/导航更新、按钮状态与Apps刷新事件。 |
| ui_refresh_04_renders_each_page_and_preserves_help_close | 宿主跨域契约；多个实际领域页面与全局帮助装配。 |
| ui_refresh_04_keeps_general_select_overlay_interactive | 宿主跨域契约；App常规页与全局外壳选择器浮层。 |
| ui_refresh_04_search_opens_the_language_control_row | 宿主跨域契约；真实搜索消息更新、焦点及滚动任务。 |
| help_rollout_27_quick_messages_renders_contextual_help | 宿主跨域契约；Agents子页选择与App快捷消息数据投影及全局卡片Esc行为。 |
| settings_help_gap_01_terminal_theme_moves_into_help | 宿主跨域契约；General外壳中的TerminalMessage/主题、字体和字号原事件。 |
| settings_help_gap_01_editor_theme_and_lsp_keep_their_controls | 宿主跨域契约；EditorMessage更新、Preview LSP事件、帮助选择复制/滚动。 |
| settings_help_gap_01_search_reveals_both_theme_rows | 宿主跨域契约；App搜索结果、字段定位和真实滚动任务。 |
| messages_tests::test_module_is_wired、types_tests::test_module_is_wired | 移入shell::host_module_30_tests同名子模块；模块末尾名称断言保留，旧文件与旧挂载删除。 |

数量核对：帮助UI原40个断言宏迁后40个；标签夹具原27个迁后领域+宿主27个；系统外壳原112个迁后领域+宿主112个；最早帮助弹层2个原断言保留。新增消息路由、移动端真实绘制、通用标签像素/几何/事件测试源码只补迁移边界，均未执行。

与前轮共享控件测试核对：desktop-ui-controls 的 ui_refresh_03_layout_and_interaction_hides_background_menu 专门验证背景菜单隐藏，未替代本次固定操作区/本地化/禁用值/真实provider草稿断言；不以同名页面推断等价，原用例保留且本次旧宿主挂载已删除。

## 行数与静态检查

固定口径：pathlib.read_text().splitlines()，含注释空白，按baseline.json task=30的19项统计。来源基线4291→1808，净减2483；生产候选2140→1057，净减1083。无新宿主文件。任务开始时此前任务已令system_settings.rs增加22、settings/mod.rs减少10；扣除这12行前置净增，本任务实际来源净减2495，其中生产候选净减1095。额外删除两个宿主注册冒烟文件合计22行不混入19项数字。明细见task-30-counts.json。

| 来源（相对 app/） | 基线行 | 当前行 |
| --- | ---: | ---: |
| `components/system_settings.rs` | 545 | 553 |
| `components/system_settings_common/help.rs` | 35 | 38 |
| `components/system_settings_common/help_tests.rs` | 32 | 0 |
| `components/system_settings_common/help_ui_refresh_03_tests.rs` | 614 | 0 |
| `components/system_settings_common/icons.rs` | 1 | 1 |
| `components/system_settings_common/inputs.rs` | 1 | 1 |
| `components/system_settings_common/mod.rs` | 58 | 55 |
| `components/system_settings_common/panels.rs` | 184 | 1 |
| `components/system_settings_common/select.rs` | 1 | 1 |
| `components/system_settings_common/styles.rs` | 1 | 1 |
| `components/system_settings_common/tab_appearance_tests.rs` | 540 | 115 |
| `components/system_settings_common/tabs.rs` | 1 | 1 |
| `components/system_settings_common/theme.rs` | 1 | 1 |
| `components/system_settings_common/utils.rs` | 1 | 1 |
| `components/system_settings_ui_refresh_04_tests.rs` | 965 | 636 |
| `message/settings/messages.rs` | 541 | 15 |
| `message/settings/mod.rs` | 733 | 363 |
| `message/settings/types.rs` | 14 | 2 |
| `message/settings/util.rs` | 23 | 23 |

静态检查：本任务路径git diff --check无输出；独立rustfmt只用于修改文件且skip_children=true，依赖路径调整文件保持最小差异。按清单文本读取生产和target依赖，遍历393个可达库未发现返回shell的依赖路径。380个事件名与路由引用集合完全一致，56组宿主效果目标保留。两处宿主到领域测试夹具path已解析存在；领域中未反向引用宿主。

## 尚未运行的验证与风险

本轮禁止Cargo、测试、绘制或回归，因此类型检查、所有定向测试、真实字体整形及原生平台/wasm cfg均未验证。静态检查不能证明这些行为运行成功；共享枚举和依赖调整需以后获得授权后检查。

仅以后用户授权时运行：
- python3 scripts/cargo_slot.py cargo test -p desktop-settings-shell --lib host_module_30_tests -- --test-threads=1
- python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib

宿主跨域契约的原过滤路径保留，未来由授权的统一阶段按实际版本选择，不自动运行。没有发布、推送或启动回归。
