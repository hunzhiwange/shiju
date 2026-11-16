# 任务29 静态交付证据

未运行任何 Cargo、测试、窗口或绘制。本证据只表示源码迁移与静态检查。

## 生产归属与真实调用链

- App.expert_hub 继续持有 desktop-skill-hub::ExpertHubState；无需新增平行状态。
- 宿主 SkillHubMessage → expert-hub 消息桥 → skill-hub::message::skill_hub::update。技能选择、列表、详情、版本、安装、升级和流式下载写盘连同原测试从专家库移至技能库。
- 专家消息桥仅分派 Connectors / Knowledge / Public，技能事件按变体显式转换；SkillUpdate、Effect、ToastKind 和局部事件定义归技能库。依赖维持 expert-hub → skill-hub，无反向依赖。
- 宿主 expert_hub::view / shiju_skills_view / shiju_apps::view → expert-hub::views::workspace。workspace 只借用七组局部输入及项目聊天可用性，复用既有组件，无 App 引用、Deref 或反向路径加载。
- 宿主 message/skill_hub.rs 保留账号/项目上下文提取、消息枚举映射、标签生命周期、会话技能选择、Toast、剪贴板/应用重载等跨域效果装配。connectors.rs、knowledge.rs、public.rs、upgrades.rs 仅保留真实桥接及测试适配。
- 所有旧页面包装模块与旧测试挂载删除，页面组合与显式事件定义归专家库 workspace，底层既有 views 不复制。

## 断言归属

- 原 expert_hub 下 connectors、list、mod、experts、public_skill_detail、public_skills 与 connectors_ui_refresh_39 测试/工具完整迁入 views/workspace 同名文件，测试名、布局、事件、明暗主题、中文和 screenshot/draw 逻辑保留；夹具只构造局部领域状态。
- UI 子进程隔离仍保留，fixture 不启动桌面 App；原无效网关端点前置断言改为从同一隔离配置文件读取 host.trim()=="["。不再需要宿主全局配置或启动异步任务。
- 原 skill-hub controller/tests、upgrades_tests、skill_operations/tests 原样随生产入口归技能库；controller 测试注册为 message::skill_hub::host_module_29_tests。
- 原宿主 upgrades_tests 四个纯转换测试迁入 host_module_29_tests::upgrade_scenarios，Toast.kind 断言改检验真实 Effect::Feedback.kind；账户清理直接调用同一 ExpertHubState::clear_for_account。首个导航测试留宿主，验证真实标签效果。
- 公共详情项目切换用例迁入 workspace::public_project_tests，使用真实公共更新入口和局部项目上下文。
- 宿主 skill_hub_tests 保留 App.update 全链路、任务数、标签导航、账号/弹出菜单与 Toast/缓存同步组合契约；领域 reducer 的状态断言已经在迁入的整组 controller tests 独立覆盖。未删断言以追求减量。
- public_tests 仅保留实际聊天技能选择/切换项目标签、SelectPublicSuites 效果落实、安装目录刷新跨层契约。
- shiju_knowledge_reader_tests / shiju_knowledge_view_tests 留宿主，仅验证被接受错误才更新真实 Toast，重复响应不再次通知。

## 来源逐文件审计

| 来源 | 基线行 | 当前行 | 归属/保留原因 |
|---|---:|---:|---|
| `crates/vw-desktop/src/app/message/skill_hub/connectors.rs` | 13 | 13 | 局部上下文及跨域效果装配 |
| `crates/vw-desktop/src/app/message/skill_hub/knowledge.rs` | 13 | 13 | 局部上下文及跨域效果装配 |
| `crates/vw-desktop/src/app/message/skill_hub/public.rs` | 15 | 15 | 局部上下文及跨域效果装配 |
| `crates/vw-desktop/src/app/message/skill_hub/public_tests.rs` | 210 | 171 | 真实跨域事件、导航、反馈契约 |
| `crates/vw-desktop/src/app/message/skill_hub/upgrades.rs` | 6 | 6 | 局部上下文及跨域效果装配 |
| `crates/vw-desktop/src/app/message/skill_hub/upgrades_tests.rs` | 151 | 46 | 真实跨域事件、导航、反馈契约 |
| `crates/vw-desktop/src/app/message/skill_hub.rs` | 517 | 519 | 局部上下文及跨域效果装配 |
| `crates/vw-desktop/src/app/message/skill_hub_tests.rs` | 805 | 805 | 真实跨域事件、导航、反馈契约 |
| `crates/vw-desktop/src/app/views/expert_hub/connectors.rs` | 21 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/connectors_tests.rs` | 79 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/connectors_ui_refresh_39_tests.rs` | 851 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/detail.rs` | 14 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/detail_tests.rs` | 1 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/experts.rs` | 27 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/experts_tests.rs` | 152 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/install_guide.rs` | 22 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/installed_skills.rs` | 25 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/knowledge/mod.rs` | 13 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/list.rs` | 57 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/list_tests.rs` | 150 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/mod.rs` | 82 | 60 | 局部上下文及跨域效果装配 |
| `crates/vw-desktop/src/app/views/expert_hub/mod_tests.rs` | 30 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/public_skill_detail.rs` | 33 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/public_skill_detail_tests.rs` | 114 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/public_skills.rs` | 19 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/public_skills_tests.rs` | 218 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/shiju_apps/api_key_binding.rs` | 1 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/shiju_apps/mine.rs` | 1 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/shiju_apps/mod.rs` | 12 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/shiju_apps/styles.rs` | 1 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/shiju_knowledge_reader_tests.rs` | 63 | 63 | 真实跨域事件、导航、反馈契约 |
| `crates/vw-desktop/src/app/views/expert_hub/shiju_knowledge_view_tests.rs` | 31 | 31 | 真实跨域事件、导航、反馈契约 |
| `crates/vw-desktop/src/app/views/expert_hub/styles.rs` | 2 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |
| `crates/vw-desktop/src/app/views/expert_hub/upgrade_actions.rs` | 14 | 0 | 专家库 workspace 同名模块/测试（旧入口删除） |

同 baseline splitlines 口径：来源 3763 → 1742，宿主净减少 2021 行；生产候选 908 → 626，减少 282 行。没有新建宿主替代文件。分类含空白/注释，生产候选不是运行代码测量。

## 静态检查与以后验证

已用独立 rustfmt 解析本任务新增/迁移模块，并检查 git diff --check；核对 cfg(test)、父模块注册、真实宿主调用和 Cargo 依赖方向。未编译、未测试，不承诺运行、字体整形或构建性能。

以后仅另行授权时：
- `python3 scripts/cargo_slot.py cargo test -p desktop-skill-hub --lib host_module_29_tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-expert-hub --lib views::workspace -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo test -p desktop-skill-hub --lib skill_operations::tests -- --test-threads=1`
- `python3 scripts/cargo_slot.py cargo check -p vw-desktop --lib`

风险：禁测期间仅源码核对，跨库类型、借用和平台条件尚无编译证据。其它并行任务正在迁移 navigation_tabs，本任务宿主装配读取当前真实字段；不提交其批量字段替换。
