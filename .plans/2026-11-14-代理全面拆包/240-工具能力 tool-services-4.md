# 任务 240: 工具能力 tool-services-4

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 4（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：任务 214

历史审计编号：原任务 189，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 写入前置任务 214：同一实施范围 `tool-services` 的连续切片，共用目标清单/模块入口；这是写入顺序，不代表其它包存在接口依赖。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 1 的 `crate::project::instance::provide_context`；任务 7 的 `crate::permission::next`；任务 63 的 `crate::session::capability_policy::resolve_skill_capabilities`；任务 158 的 `crate::plugins::plugin_skill_reference_text`；任务 267 的 `crate::skills::PluginSkillSource`；任务 269 的 `crate::session::ui_store::load_session_todos_scoped`；任务 290 的 `crate::security::SecurityPolicy::default`；任务 312 的 `crate::session::ui_types::SessionTodoItem`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：已阻塞

目的：迁移，将本任务职责归入 `tool-services`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/tool-services/Cargo.toml`、`crates/libs/tool-services/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/tools/skill/invocation.rs`
- `crates/vw-agent/src/tools/skill/invocation_tests.rs`
- `crates/vw-agent/src/tools/skill/mod.rs`
- `crates/vw-agent/src/tools/skill/mod_tests.rs`
- `crates/vw-agent/src/tools/skill/prompt.rs`
- `crates/vw-agent/src/tools/skill/tests.rs`
- `crates/vw-agent/src/tools/sleep/mod.rs`
- `crates/vw-agent/src/tools/sleep/mod_tests.rs`
- `crates/vw-agent/src/tools/sleep/tests.rs`
- `crates/vw-agent/src/tools/tests/mod_tests.rs`
- `crates/vw-agent/src/tools/todo/mod.rs`
- `crates/vw-agent/src/tools/todo/normalize.rs`
- `crates/vw-agent/src/tools/todo/normalize_tests.rs`
- `crates/vw-agent/src/tools/todo/schema.rs`
- `crates/vw-agent/src/tools/todo/schema_tests.rs`
- `crates/vw-agent/src/tools/todo/store.rs`
- `crates/vw-agent/src/tools/todo/store_tests.rs`
- `crates/vw-agent/src/tools/todo/tests.rs`
- `crates/vw-agent/src/tools/todo/todoread-cn.txt`
- `crates/vw-agent/src/tools/todo/todoread.txt`
- `crates/vw-agent/src/tools/todo/todowrite-cn.txt`
- `crates/vw-agent/src/tools/todo/todowrite.txt`

## 上下文

- 重点符号：`build_invocation_result`、`description_for_skills`、`SkillTool`、`explicitly_referenced_skill_names`、`user_explicitly_requests_skill_use`、`user_invocable_skill_names`、`accessible_skills`、`build_description_text`、`build_description_text_for_skills`、`skill_prompt_content`、`SleepTool`、`read_session_todos`、`write_session_todos`、`write_session_todos_for_test`、`read`、`write`、`TodoReadTool`、`TodoWriteTool`。
- 当前文本引用候选：`tools`、`project::instance`、`security`、`session::ui_store`、`skills`、`tools::traits`、`plugins::plugin_skill_reference_text`、`agent`、`config`、`permission::next`、`question`、`session::capability_policy`、`tools::skill`、`app::agent`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：具体工具按能力分包，工厂由 tool-assembly 组合；涉及会话、插件与调度器的工具只依赖前置端口，不反向依赖会话或主循环。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/tool-services`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。

## 本次静态实施记录

- 已迁移 sleep 全部源码和两个测试文件、todo 全部源码和既有测试/四份文本资源到 tool-services；原入口仅重导出及 ToolRuntimeContext 的字符串参数适配。
- todo 状态唯一保留在 tool-services::todo::store，直接消费 agent-project::instance、session-artifact-store 和 session-ui-types；未复制持久化状态。非 wasm 持久化映射、scope key、锁恢复、速率限制、merge 算法保持原样。WASM 保留旧目录规则并显式导入 directories::ProjectDirs。
- 公共清单与 lib.rs 通过授权限定入口精确替换接线；原 tools/mod.rs、toolset/mod.rs 和外部调用者继续经原薄入口调用同一实现，无需改动工厂顺序。
- 剩余 skill/mod.rs 的实际宿主依赖：skills::load_session_skills / load_skills_with_config 仍由 vw-agent/src/skills/mod.rs 实现；agent::default_agent / permission_rules、plugins::plugin_skill_reference_text、工具上下文具体 Host 类型、block_on 的租户与项目任务局部继承也仍在主包。不得直接导入主包或回调整个业务 JSON。skill/invocation.rs、prompt.rs 及三个既有测试尚未迁移，需要继续设计并落实精确强类型 SkillHost 后才能删旧实现。未将此部分记为完成。
- tools/tests/mod_tests.rs 是组合工厂测试（memory、完整工具注册），仍归原组合入口，不可整体迁入 tool-services。
- 本轮仅完成 sleep/todo 静态源审阅、原测试导入调整与资源核对；未运行 Cargo、单元测试及实机测试，待统一验收。本卡保留，不能作为240整体完成证据。
