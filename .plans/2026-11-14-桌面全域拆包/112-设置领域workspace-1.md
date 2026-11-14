# 任务 112: 设置领域workspace（切片 1/2）

先阅读并遵守 [公共执行规则](plan.md)。本任务无需读取 prompt.md。

批次：【批次 112】 依赖：任务 111（批次 111）

状态：未开始

目的：完成设置领域workspace的本片迁移与实际调用接线，缩小桌面 crate 的实现范围。

来源任务：用户要求尽可能全部拆分桌面包；文件覆盖见同目录源码归属清单。

预计会话范围：同一职责的窄切片，生产源码约 1942 行；只处理以下文件与必要调用/类型接线，不扩大为整个子系统迁移。较长现存文件先按实际符号分离职责，再迁移本片。

## 文件

目标：`crates/libs/desktop-settings-workspace`。普通领域目标新建最小 Cargo.toml、src/lib.rs、对应模块及独立测试；同领域前片已创建则扩展。保持来源子目录避免同名冲突。

来源及相邻测试：

- `crates/vw-desktop/src/app/components/system_settings_projects.rs`
- `crates/vw-desktop/src/app/components/system_settings_projects_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_projects_ui_refresh_30_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/environment.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/environment_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/managed.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/managed_tests.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/recovery.rs`
- `crates/vw-desktop/src/app/components/system_settings_worktrees/recovery_tests.rs`
- `crates/vw-desktop/src/app/message/settings/projects.rs`
- `crates/vw-desktop/src/app/message/settings/projects_tests.rs`
- `crates/vw-desktop/src/app/message/settings/worktrees.rs`
- `crates/vw-desktop/src/app/message/settings/worktrees/environment.rs`

接线修改：根 `Cargo.toml`、`crates/vw-desktop/Cargo.toml`、目标库清单、上述文件父模块和实际调用者。非 Rust 资源依照 include 宏与目录引用一并归属。

## 上下文与边界

迁移 projects, worktrees, project_profiles 子页的领域状态、局部消息、更新和视图；拆出实际读取字段，网关写入通过 desktop-config；跨子页导航由壳处理，不传完整 App。

当前真实符号入口：current_project_worktree, quiet_button_style, project_icon, menu_item, project_row, view, message, page, view_overlays, editor, event, protection, overlay, update, set_worktree_enabled, environment_command_saved, start_worktree_save, wait_for_worktree_saves, WorktreeSettingsMessage, apply, gateway_changed, EnvironmentMessage, can_add, scope, save_command, load_projects。

当前跨桌面引用样例：crate::app::assets::Icon；crate::app::components::overlays::BelowOverlay；crate::app::components::system_settings_common::{；crate::app::i18n::I18n；crate::app::message::SettingsMessage；crate::app::message::settings::worktrees::WorktreeSettingsMessage as Event；crate::app::components::system_settings_common::*；crate::app::message::settings::worktrees::{；crate::app::{Message, i18n::I18n, state::WorktreeEnvironmentState}；crate::app::message::settings::worktrees::ManagedMessage；crate::app::{Message, i18n::I18n, state::ManagedWorktreesState}；crate::app::message::settings::worktrees::{ManagedMessage, recovery::RecoveryMessage}；crate::app::{Message, state::WorktreeSettingsState}；crate::app::config::gateway_client_endpoint；crate::app::projects::{save_recent_projects_background, save_recent_projects_meta_background}；crate::app::state::{WorktreeSettingsIdentity, WorktreeSettingsState, WorktreesTab}；cr。这些是静态快照，先搜索确认，禁止按名字臆造共享类型。

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
rg -n 'crate::app::|vw_desktop|vw-desktop' crates/libs/desktop-settings-workspace
git diff --check
git diff --stat
```

逐一审查搜索命中，不把 rg 零命中退出码 1 当作失败或编译通过。另查暂存差异，防止提交无关改动。所需证据：本片迁移映射、局部状态/消息边界、旧调用接线、测试注册、cfg/资源审阅及 diff 检查结果。新库不得反向依赖桌面；壳适配不得残留本片业务副本。明确报告未运行 Cargo、单元测试和实机验收。
