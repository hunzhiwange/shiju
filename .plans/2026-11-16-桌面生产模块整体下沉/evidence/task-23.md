# 任务23：技能管理与技能中心

静态交付完成；未编译、未测试、未运行绘制或截图。仅迁移源码，不宣称运行通过。

## 生产归属与接线

- `desktop-settings-skills::page::view` 拥有页面标题、10个中心操作入口、帮助按钮、标签布局和保存错误横幅；沿用原有控件样式、文字与事件。
- 宿主 `system_settings_skills::view` 将浏览器/发现顺序/配置内容传入领域页面；`page_message` 将具名事件映射为现有 SettingsMessage。
- 原有领域 `update::state::State` 已由 App.skills_settings 持有，初始化无新增字段。`update::catalog::update` 原已拥有目录提供者、表单设置、选择、搜索、启禁用/删除/创建及对应效果；沿用而不建立平行算法。
- 技能中心导入、安装、仓库、更新等按钮通过旧 SettingsMessage 到现有 `skills::{import_modal,install_view,repo_modal,updates_view}::update`，后者调用既有领域 update；未修改相邻任务来源文件。
- `skills.rs` 保留旧枚举映射、Task 边界执行、项目打开/刷新、网关调用与配置持久化装配。`update_input` 保留账号 token/tenant/project/host 的跨域授权上下文装配，检查条件不变。
- 无新增生产依赖。测试使用 desktop-chat-view 的真实 selectable_titled_text 与 desktop-assets 字体；没有 App 类型、宿主 include/path 反向加载或依赖环。

## 全组断言映射

以下用例保持原名称；循环、assert/assert_eq/matches、Geometry、事件发送、draw、screenshot、中文/暗黑/窄屏样本原样迁移，仅将宿主消息外壳替换为真实局部事件外壳。

- `view_builds_every_skills_settings_tab` → `host_module_23_tests::view_builds_every_skills_settings_tab`。
- `skills_error_banner_follows_skills_tabs` → `host_module_23_tests::skills_error_banner_follows_skills_tabs`。
- `view_overlays_builds_with_help_closed_and_open` → `host_module_23_tests::view_overlays_builds_with_help_closed_and_open`。
- `copy_install_command_emits_the_documented_template` → `host_module_23_tests::copy_install_command_emits_the_documented_template`。
- `ui_refresh_19_renders_catalogs_and_modals` → `host_module_23_tests::ui::ui_refresh_19_renders_catalogs_and_modals`。
- `ui_refresh_19_layout_and_interaction` → `host_module_23_tests::ui::ui_refresh_19_layout_and_interaction`。
- `ui_refresh_19_preserves_values_and_disabled_states` → `host_module_23_tests::ui::ui_refresh_19_preserves_values_and_disabled_states`。
- `help_rollout_10_renders_contextual_help` → `host_module_23_tests::ui::help_rollout_10_renders_contextual_help`。
- `help_rollout_10_plugin_formats_keep_audit_visible` → `host_module_23_tests::ui::help_rollout_10_plugin_formats_keep_audit_visible`。
- `settings_help_gap_14_plugin_catalog` → `host_module_23_tests::ui::settings_help_gap_14_plugin_catalog`。

`copy_install_command_emits_the_documented_template` 原断言任务输出 CopyCode 等于文档模板，现直接断言真实 catalog update 的 Copy 效果等于同一模板；不再启动 App。

宿主仅保留三个契约：
- `settings_tabs_capabilities_appearance_skills`：真实 App.update 经 SettingsMessage 投影目录作用域和标签，保留真实场景检查。
- `skills_project_selection_refreshes_current_and_opens_other_project`：刷新会同时触达 Agents，打开项目会更改 App/agents 的路径及 loading，全部原断言保留。
- `help_rollout_10_agent_policy_and_scope`：真实 agents 页面策略和保存事件、selectable help、字体整形与截图。agents 依赖 skills，不能反向放入 skills。宿主夹具已缩为 App::new；删除全部技能/插件目录和审计夹具，保留其实际使用的渲染/点击/帮助辅助函数。

与既有 desktop_split_14_tests 对照：该组覆盖 field_row、tab button、system_config_view、browser 的局部结构；本次迁入页面完整组合、错误横幅及绘制/交互，不复制该组原断言。原测试注册均已移除对应迁出用例，宿主挂载只剩上述跨域契约。

## 逐项宿主保留理由

- page_message/config_context/help/heading_help：局部事件与宿主消息、selectable chat 文本的适配。
- SKILLS_TABS/skills_tab_labels：宿主设置搜索与导航描述符。
- system_config_view/view：三个现有领域视图的类型映射与内容装配。
- view_overlays/with_plugin_install：其它独立页面族和插件安装领域浮层的消息适配与装配。
- skills.rs 的 gateway task 函数与 execute：网关系统资源边界和跨域 Task 装配；update：旧大枚举与独立页面族入口路由；update_input：账号及应用安装上下文投影。
- 测试模块保留范围详见上面三个跨域契约，无领域目录/审计/App配置大型夹具残留。

## 基线同口径统计

Python pathlib.read_text().splitlines()，新建宿主文件为0。

| 来源 | 基线 | 当前 | 净减少 |
|---|---:|---:|---:|
| `crates/vw-desktop/src/app/components/system_settings_skills.rs` | 249 | 198 | 51 |
| `crates/vw-desktop/src/app/components/system_settings_skills_tests.rs` | 94 | 53 | 41 |
| `crates/vw-desktop/src/app/components/system_settings_skills_ui_refresh_19_tests.rs` | 1039 | 319 | 720 |
| `crates/vw-desktop/src/app/message/settings/skills.rs` | 248 | 246 | 2 |
| `crates/vw-desktop/src/app/message/settings/skills_tests.rs` | 58 | 45 | 13 |

宿主合计 1688 → 861 行，净减少 827 行。生产候选两文件合计 497 → 444 行，减少53行；其余减少为测试，不混为生产减量。

## 静态验证与风险

- 独立 rustfmt（仅修改文件、skip_children）已执行；本任务路径 git diff --check 通过。
- 全工作区 diff --check 曾指出其他并行任务 vw-agent 三个测试文件 EOF 空行，未改动其文件。
- 源码核对领域测试注册、宿主调用者、局部事件映射、依赖方向。10个领域用例独立注册；三个跨域用例保留。
- 未运行任何 Cargo 命令、测试、UI 或最终回归。新增 dev-dependencies 和迁移后的类型/生命周期、真实渲染行为仍须以后授权验证。
- 后续唯一领域命令：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-skills --lib host_module_23_tests -- --test-threads=1`，预期10个有效用例，尚未运行。
- 后续宿主检查：`python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`，尚未运行。
