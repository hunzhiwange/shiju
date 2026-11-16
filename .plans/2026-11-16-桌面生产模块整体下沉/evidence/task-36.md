# 任务36阶段证据（未完成）

本记录不表示任务整体交付。遵守禁测要求：未运行Cargo、单元测试、实机或回归；只做源码审阅、独立rustfmt及git diff --check。

## 已接通的真实路径

- 宿主search::catalog仍组合各领域描述符和当前qdrant可见性；搜索目标类型、page映射、稳定排序、本地化、字段条目和连接标签均在shell::search，真实调用者委派新入口。
- navigation::project通过ActiveNavigation的12个显式值调用shell::search_target_is_active；不传App，无反向宿主依赖，网络成员判定复用已有is_network_tab。
- device_sync::selection::inventory仍获取跨领域当前host/project授权库存；组合去重、含技能引用的不可选策略和标签在memory::selection::append_profiles。宿主按原ProjectProfileScope顺序提供借用迭代器。
- 未改变配置持久化、原生资源与权限策略，未增加能力开关。

## 原断言映射

- search_results_show_secondary_pages_with_parent_paths整组迁shell::host_module_36_tests，同名保留Browser目标、英文标签、父路径、网络配置首结果及五个子目标路径断言。
- search_matches_setting_labels_and_localizes_each_parent整组迁同模块，三语言语言字段首结果、目标、标签、父路径及Terminal/Editor字号两条路径断言保持。
- 新夹具使用真实SystemTab目录、network领域描述符和生产localize_catalog；没有App、复制的匹配算法或伪造服务。
- catalog_extracts_unique_navigable_child_tabs_and_scoped_keywords留宿主，验证跨领域描述符数量、唯一性和qdrant条件目录，所有断言保留。
- 其它来源测试未改动，本阶段未声称已完成它们的归属审计。

## 本阶段splitlines（编辑前当前HEAD到编辑后；不冒充固定基线净减量）

| 来源 | 前 | 后 |
|---|---:|---:|
| `crates/vw-desktop/src/app/components/system_settings_search.rs` | 241 | 70 |
| `crates/vw-desktop/src/app/components/system_settings/navigation.rs` | 85 | 79 |
| `crates/vw-desktop/src/app/components/system_settings_device_sync/selection.rs` | 97 | 86 |
| `crates/vw-desktop/src/app/components/system_settings_search_tests.rs` | 148 | 88 |

## 未完成项与后续实施入口

任务卡保留，不标完成或待回归。除上表文件外的baseline任务36来源未完成逐函数残留证明。尤其需要继续审阅：

- prompts/mod.rs的完整事件转换，以及providers/updates.rs、managed_shiju.rs的加载与状态策略是否仍属于领域；
- channels/common.rs、skills/browser.rs与worktrees.rs及environment.rs的局部状态和领域效果；
- companion及size_modal绘制夹具、cron list/delivery/date_time、providers和channels测试中的App依赖，完成整组迁移或逐断言证明真实跨域边界；
- 为全部来源列出迁出位置、残留理由与固定baseline的净减量，本记录尚非最终全文件验收。

已存在的security/computer_control/voice/device_sync/prompts入口大多已经正向委派既有领域库，但仅此不足以证明任务36全部完成。

以后用户明确启用验证时才运行：`python3 scripts/cargo_slot.py cargo test -p desktop-settings-shell --lib host_module_36_tests -- --test-threads=1`。本轮没有运行或通过数。
