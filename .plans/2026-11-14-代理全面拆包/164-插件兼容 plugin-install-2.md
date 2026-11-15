# 任务 164: 插件兼容 plugin-install-2

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 2（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：任务 29

历史审计编号：原任务 109，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 写入前置任务 29：同一实施范围 `plugin-install` 的连续切片，共用目标清单/模块入口；这是写入顺序，不代表其它包存在接口依赖。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 1 的 `crate::project::instance::provide`；任务 11 的 `crate::hooks::HookResult::Continue`；任务 17 的 `crate::plugins::hook_handler::codex_handlers`；任务 19 的 `crate::question::list`；任务 24 的 `crate::tools::mcp_common::server_name`；任务 47 的 `crate::plugins::compat::mcp_connection_sessions::close_plugin`；任务 56 的 `crate::tools::mcp_tool::mcp_tool_id`；任务 135 的 `crate::gateway::api::handlers::plugins::stateful_router`；任务 158 的 `crate::plugins::NpmPluginArchive`；任务 182 的 `crate::session::compaction_checkpoint::restore`；任务 190 的 `crate::tools::ToolCallResult`；任务 195 的 `crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected`；任务 240 的 `crate::tools::skill::SkillTool`；任务 267 的 `crate::skills::Skill`；任务 269 的 `crate::session::ui_store::current_tenant_scope`；任务 290 的 `crate::security::AutonomyLevel::Supervised`；任务 303 的 `crate::session::runtime::SessionToolset::new`；任务 305 的 `crate::gateway::start`；任务 307 的 `crate::session::session::Error`；任务 312 的 `crate::session`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：未开始

目的：迁移，将本任务职责归入 `plugin-install`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/plugin-install/Cargo.toml`、`crates/libs/plugin-install/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/plugins/compat/npm_archive.rs`
- `crates/vw-agent/src/plugins/compat/npm_source.rs`
- `crates/vw-agent/src/plugins/compat/onboarding.rs`
- `crates/vw-agent/src/plugins/compat/path_audit.rs`
- `crates/vw-agent/src/plugins/compat/platform_tests.rs`
- `crates/vw-agent/src/plugins/compat/plugin_window_fixture.rs`
- `crates/vw-agent/src/plugins/compat/portable_manifest.rs`
- `crates/vw-agent/src/plugins/compat/session_snapshot.rs`
- `crates/vw-agent/src/plugins/compat/skill_frontmatter.rs`
- `crates/vw-agent/src/plugins/compat/skill_invocation.rs`
- `crates/vw-agent/src/plugins/compat/skill_roots.rs`
- `crates/vw-agent/src/plugins/compat/t02_tests.rs`
- `crates/vw-agent/src/plugins/compat/t03_tests.rs`
- `crates/vw-agent/src/plugins/compat/t04_tests.rs`
- `crates/vw-agent/src/plugins/compat/t05_tests.rs`
- `crates/vw-agent/src/plugins/compat/t06_tests.rs`
- `crates/vw-agent/src/plugins/compat/t07_tests.rs`
- `crates/vw-agent/src/plugins/compat/t08_tests.rs`
- `crates/vw-agent/src/plugins/compat/t09_tests.rs`
- `crates/vw-agent/src/plugins/compat/t10_tests.rs`
- `crates/vw-agent/src/plugins/compat/t11_tests.rs`
- `crates/vw-agent/src/plugins/compat/t12_tests.rs`
- `crates/vw-agent/src/plugins/compat/t13_tests.rs`
- `crates/vw-agent/src/plugins/compat/t14_tests.rs`
- `crates/vw-agent/src/plugins/compat/t15_tests.rs`
- `crates/vw-agent/src/plugins/compat/t16_tests.rs`
- `crates/vw-agent/src/plugins/compat/t17_public_tests.rs`
- `crates/vw-agent/src/plugins/compat/t17_tests.rs`
- `crates/vw-agent/src/plugins/compat/t18_tests.rs`
- `crates/vw-agent/src/plugins/compat/t19_tests.rs`
- `crates/vw-agent/src/plugins/compat/t20_tests.rs`
- `crates/vw-agent/src/plugins/compat/t21_tests.rs`
- `crates/vw-agent/src/plugins/compat/t22_tests.rs`
- `crates/vw-agent/src/plugins/compat/t23_tests.rs`
- `crates/vw-agent/src/plugins/compat/t24_tests.rs`
- `crates/vw-agent/src/plugins/compat/t26_tests.rs`
- `crates/vw-agent/src/plugins/compat/t27_flow_tests.rs`
- `crates/vw-agent/src/plugins/compat/t27_tests.rs`
- `crates/vw-agent/src/plugins/compat/t28_tests.rs`
- `crates/vw-agent/src/plugins/compat/t29_tests.rs`
- `crates/vw-agent/src/plugins/compat/t30_tests.rs`
- `crates/vw-agent/src/plugins/compat/t32_tests.rs`
- `crates/vw-agent/src/plugins/compat/t33_tests.rs`
- `crates/vw-agent/src/plugins/compat/t34_tests.rs`
- `crates/vw-agent/src/plugins/compat/t35_tests.rs`
- `crates/vw-agent/src/plugins/compat/t36_tests.rs`
- `crates/vw-agent/src/plugins/compat/t37_tests.rs`
- `crates/vw-agent/src/plugins/compat/t38_tests.rs`
- `crates/vw-agent/src/plugins/compat/t39_tests.rs`
- `crates/vw-agent/src/plugins/compat/t40_tests.rs`
- `crates/vw-agent/src/plugins/compat/t41_tests.rs`
- `crates/vw-agent/src/plugins/compat/t42_tests.rs`
- `crates/vw-agent/src/plugins/compat/t43_tests.rs`
- `crates/vw-agent/src/plugins/compat/t44_tests.rs`
- `crates/vw-agent/src/plugins/compat/t45_tests.rs`
- `crates/vw-agent/src/plugins/compat/t46_tests.rs`
- `crates/vw-agent/src/plugins/compat/t47_tests.rs`
- `crates/vw-agent/src/plugins/compat/t48_tests.rs`
- `crates/vw-agent/src/plugins/compat/t49_tests.rs`
- `crates/vw-agent/src/plugins/compat/t50_tests.rs`
- `crates/vw-agent/src/plugins/compat/t51_tests.rs`
- `crates/vw-agent/src/plugins/compat/t52_tests.rs`
- `crates/vw-agent/src/plugins/compat/t53_tests.rs`
- `crates/vw-agent/src/plugins/compat/t54_tests.rs`
- `crates/vw-agent/src/plugins/compat/t57_tests.rs`
- `crates/vw-agent/src/plugins/compat/t58_tests.rs`
- `crates/vw-agent/src/plugins/compat/t59_tests.rs`
- `crates/vw-agent/src/plugins/compat/t60_tests.rs`
- `crates/vw-agent/src/plugins/compat/t61_tests.rs`
- `crates/vw-agent/src/plugins/compat/t62_tests.rs`
- `crates/vw-agent/src/plugins/compat/t64_tests.rs`
- `crates/vw-agent/src/plugins/compat/t66_tests.rs`
- `crates/vw-agent/src/plugins/compat/transaction_install.rs`
- `crates/vw-agent/src/plugins/compat/upgrade.rs`
- `crates/vw-agent/src/plugins/compat/mod.rs`

## 上下文

- 重点符号：`install_archive`、`install`、`NpmPluginArchive`、`fetch_npm_plugin`、`fetch`、`entry`、`validate_component_path`、`load_portable_manifest`、`PluginSessionSnapshot`、`PluginTurnSnapshot`、`selection_for_runtime`、`discover_for_runtime`、`SkillFrontmatter`、`parse`、`reference_text`、`SkillPackageFormat`、`package_format`、`skill_roots`。
- 当前文本引用候选：`plugins`、`session::ui_store`、`config`、`skills::load_skills_full_with_config`、`hooks::execution_hook_runner`、`session::prompt`、`tools::current_tool_use_context`、`skills`、`plugins::install`、`plugins::install_plugin_marketplace_entry`、`plugins::compat`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：复用 plugin-types；compat/MCP/hook 细分子包任务见文件清单；安装路径审计与信任确认不可省略。 agent-plugins入口延后组合；安装、MCP与hook不得相互反向依赖，跨域调用用前置tool-context或已有业务端口。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/plugin-install`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。
