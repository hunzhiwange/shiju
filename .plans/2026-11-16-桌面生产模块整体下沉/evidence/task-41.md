# 任务41实施检查点

本检查点未完成任务41，不删除任务卡。未运行任何 Cargo、单元测试或实机验证。git diff --check 已执行，未报告空白错误。

## 已接通生产路径

- App::title → desktop_navigation::window_lifecycle::title；宿主持有窗口 ID，保留宠物窗口标题选择。
- App::update → native_menu::snapshot → desktop_navigation::native_menu；菜单动作解析结果通过已有 top_bar::menus::map 转为宿主消息；macOS 仍在宿主执行菜单安装与订阅事件装配。
- App::subscription → needs_activity_animation_tick → desktop_platform::subscriptions::Activity::needs_tick；App 仅投影各领域活跃条件。

## 断言映射

native_menu/actions_tests.rs 全部7个用例迁至 desktop-navigation/src/host_module_41_tests.rs，保留原名和全部断言；App 夹具改为真实 NativeMenuSnapshot 与 TabInput。项目文件、剪贴板、帮助、非项目禁用、菜单分区、动态标签过滤/活动态/标题、语言快照均保留。

新增 desktop-platform/src/host_module_41_tests.rs 覆盖活动策略，宿主 app_subscription_tests.rs 原用例完整保留，继续验证领域状态到策略输入的实际投影。其他原测试未删除、未声称迁移完成。

## 后续必要工作（不是验证延期）

- app_basic_tests.rs 的领域夹具尚未逐组迁出；标题断言目前仍由宿主测试覆盖。
- desktop_integration_29/46/47/48/49、installer_update_tests、plugin_compat_t25、恢复与插件窗口夹具尚需完整断言/归属审查和局部输入改造。
- 托盘切换结果的 App 装配、基础缓存/反馈与平台通知异步入口还需逐函数审查，不把本次检查点当成全部必要边界已获证明。
- build.rs 目前未改动，资源生成和链接仍属桌面可执行程序的构建职责。

## task=41基线逐文件清单（splitlines）

| 文件 | 基线 | 当前 | 本检查点归属 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/desktop_integration_29_tests.rs` | 46 | 46 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_integration_46_tests.rs` | 28 | 28 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_integration_47_tests.rs` | 57 | 57 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_integration_48_tests.rs` | 48 | 48 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_integration_49_tests.rs` | 52 | 52 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_lifecycle/native.rs` | 32 | 32 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_lifecycle/tray_switch.rs` | 68 | 68 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_lifecycle/window.rs` | 81 | 81 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_lifecycle.rs` | 44 | 44 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_models.rs` | 4 | 4 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_notification.rs` | 71 | 71 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/desktop_update.rs` | 2 | 2 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/impls/app_basic.rs` | 389 | 340 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/impls/app_basic_tests.rs` | 400 | 400 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/impls/app_subscription.rs` | 464 | 463 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/impls/app_subscription_tests.rs` | 563 | 563 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/installer_update.rs` | 13 | 13 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/installer_update_tests.rs` | 198 | 198 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/native_menu/actions.rs` | 98 | 30 | 领域决策迁至 desktop-navigation/src/native_menu.rs，保留App输入投影及消息映射 |
| `crates/vw-desktop/src/app/native_menu/actions_tests.rs` | 158 | 0 | 整组迁至 desktop-navigation/src/host_module_41_tests.rs |
| `crates/vw-desktop/src/app/native_menu/macos.rs` | 29 | 44 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/native_menu/mod.rs` | 21 | 17 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/plugin_compat_t25_tests.rs` | 41 | 41 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/plugin_compat_window.rs` | 366 | 366 | 尚待完整审查，保持现有实现与覆盖 |
| `crates/vw-desktop/src/app/recovery_window_tests.rs` | 406 | 405 | 尚待完整审查，保持现有实现与覆盖 |

后续用户授权验证命令（本次未运行）：
```sh
python3 scripts/cargo_slot.py cargo test -p desktop-platform --lib host_module_41_tests -- --test-threads=1
python3 scripts/cargo_slot.py cargo test -p desktop-navigation --lib host_module_41_tests -- --test-threads=1
```
