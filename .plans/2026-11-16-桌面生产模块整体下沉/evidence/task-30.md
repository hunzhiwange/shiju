# 任务30：局部实施证据，未完成

本记录不是整项完成验收，保留编号任务卡。未编译、未测试、未绘制；严格遵守本轮禁测要求。

## 已实施

- desktop-ui-controls::system_settings_common::help 新增 settings_selectable_help_label，直接复用该库 selectable_text::markdown_selection 的真实选择文本实现；不再通过宿主 chat_panel 获取渲染能力。
- localized_settings_help_modal 持有本地化、帮助文本哈希/复制反馈、复制消息构造与弹窗布局。宿主 help.rs 仅投影 locale、last_copied_code_hash 和 Message::CopyCode，保留 App 所有权边界。
- panels.rs 删除已有共享控件的重复包装，仅保留三项带宿主 Message 类型的帮助组合适配，通用面板由原有 pub use shared::* 导出。核对 shared 对应函数均公开。
- 原 help_tests.rs 的 help_modal_fills_overlay_for_default_and_copied_states 整体迁入 desktop-settings-shell/src/host_module_30_tests.rs；保留初始/已复制两项 children().len()==2 断言，App::new 夹具换成实际输入 locale 默认值与复制哈希；测试名保持，旧挂载和旧文件删除。没有宣称覆盖复制事件本身（原测试亦未断言该事件）。
- 无新增依赖、无反向依赖、无 Cargo.lock 本任务变更。

## 静态核对与行数

按 pathlib.read_text().splitlines() 对 baseline.json task=30 的19项计数：基线4291，当前4128，净减163（包含其它任务此前已提交变化，不能全部归本任务）。本次 host help.rs 35→41、panels.rs 184→38、mod.rs 58→55、help_tests.rs 32→0，局部净减175；其中测试减少32，生产候选净减143。未创建宿主替代文件。

本任务路径 git diff --check 通过。全工作区 diff --check 有其它任务 session-engine/Cargo.toml 与 chat_panel/header_tests.rs 的 EOF 空行提示，不修改无关文件。独立 rustfmt 仅处理本任务修改文件，skip_children=true。

## 未完成范围（不转交为已完成）

1. SettingsMessage 仍位于宿主 messages.rs（541行），包含 super/host 事件及宿主别名，路由 mod.rs 仍为宿主装配与转发混合。shell 尚未聚合领域消息。
2. appearance 与 network 当前依赖 shell；直接令 shell 反向依赖这两个库会循环，下一步需要先决定/落实导航类型所属层次，再使用实际领域载荷聚合消息。不能直接把 enum 文本挪过去，也不能新增反向 path/include。
3. system_settings.rs 的页面装配、搜索与其它逻辑尚未逐函数收口。
4. help_ui_refresh_03_tests.rs、tab_appearance_tests.rs、system_settings_ui_refresh_04_tests.rs 的整组迁移与断言映射尚未完成；原源码和挂载完整保留。
5. icons/inputs/select/styles/tabs/theme/utils 已是单行共享重导出，作为兼容路径保留。types.rs 主要重导出领域类型；util.rs 含 gateway_client 系统端点获取，尚待完整职责核对。

下一步继续上述消息依赖方向与测试族迁移；本任务不得删卡或标记已完成。

未来用户授权后：python3 scripts/cargo_slot.py cargo test -p desktop-settings-shell --lib host_module_30_tests -- --test-threads=1；当前禁止执行。宿主检查沿用任务卡保存命令。
