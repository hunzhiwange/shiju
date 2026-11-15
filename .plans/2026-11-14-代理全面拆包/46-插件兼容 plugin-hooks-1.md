# 任务 46: 插件兼容 plugin-hooks-1

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 1（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：无

历史审计编号：原任务 162，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 无本计划内已确认前置；核对已有基线与本卡实际导出后即可进入就绪队列，不等待其它领域。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 1 的 `crate::project::instance::directory`；任务 11 的 `crate::hooks::HookRunner::new`；任务 24 的 `crate::tools::mcp_common::server_name`；任务 52 的 `crate::tools::computer_control::NativeControlRuntime::shared`；任务 158 的 `crate::plugins::mcp_session_connections`；任务 190 的 `crate::tools::scope_tool_use_context`；任务 269 的 `crate::session::ui_store::current_tenant_scope`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：已阻塞

目的：迁移，将本任务职责归入 `plugin-hooks`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/plugin-hooks/Cargo.toml`、`crates/libs/plugin-hooks/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/plugins/compat/hook_async.rs`
- `crates/vw-agent/src/plugins/compat/hook_command.rs`
- `crates/vw-agent/src/plugins/compat/hook_compact.rs`
- `crates/vw-agent/src/plugins/compat/hook_manifest.rs`
- `crates/vw-agent/src/plugins/compat/hook_matcher.rs`
- `crates/vw-agent/src/plugins/compat/hook_mcp.rs`
- `crates/vw-agent/src/plugins/compat/hook_model.rs`
- `crates/vw-agent/src/plugins/compat/hook_permission.rs`
- `crates/vw-agent/src/plugins/compat/hook_post_tool.rs`
- `crates/vw-agent/src/plugins/compat/hook_pre_tool.rs`
- `crates/vw-agent/src/plugins/compat/hook_prompt.rs`
- `crates/vw-agent/src/plugins/compat/hook_session.rs`
- `crates/vw-agent/src/plugins/compat/hook_stop.rs`
- `crates/vw-agent/src/plugins/compat/hook_subagent.rs`
- `crates/vw-agent/src/plugins/compat/hook_trust_content.rs`

## 上下文

- 重点符号：`AsyncHooks`、`AsyncHookTurn`、`interpret`、`bind_input`、`prepare_data_directory`、`retain_context`、`validate`、`parse`、`emit`、`context`、`CodexHookEvent`、`CodexHookMatcherGroup`、`CodexHookHandler`、`CodexHooksManifest`、`CodexHookDiagnostic`、`HookMatcher`、`execute`、`expand`。
- 当前文本引用候选：`tools`、`tools::current_tool_use_context`、`tools::scope_tool_use_context`、`plugins`、`session::ui_store`、`tools::mcp_common`、`project::instance`、`config::get_project_for_tenant`、`hooks::execution_hook_runner`、`plugins::mcp_session_connections`、`hooks`、`tools::execute_tool_from_registry`、`tools::computer_control`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：复用 plugin-types；compat/MCP/hook 细分子包任务见文件清单；安装路径审计与信任确认不可省略。 agent-plugins入口延后组合；安装、MCP与hook不得相互反向依赖，跨域调用用前置tool-context或已有业务端口。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/plugin-hooks`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。


## 2026-11-15 领域产物与精确交接

遵从主会话最新指示：只使用当前 checkout；不改公共清单、父模块和直接调用者；不暂存、不提交；不运行 Cargo、测试或程序。领域产物已落盘，待主会话串行审阅接线；本卡尚非全部迁移完成。

### 已迁映射及唯一所有者

- 原 compat 下 hook_async、hook_matcher、hook_permission、hook_post_tool、hook_pre_tool、hook_trust_content 完整实现迁至 crates/libs/plugin-hooks/src 同名文件；原文件仅薄重导出。AsyncHooks/AsyncHookTurn 唯一所有者变为 plugin-hooks，不复制状态。
- hook_command::{interpret,prepare_data_directory}；hook_compact::{validate,parse}；hook_mcp::expand；hook_model::{validate,token_usage}；hook_prompt::{validate,parse}；hook_session::validate；hook_stop::{validate,parse,outcome}；hook_subagent::{validate,contexts,parse} 共16个完整函数迁至新库同名模块，原模块重导出，其余宿主动作暂留。
- hook_manifest 仍沿任务17现有 plugin-types::hook_manifest 重导出，本任务没有覆盖其变更；新库同名模块同样转发唯一类型。hook_stop::Outcome 仍为 agent_hooks::TurnEndOutcome，同一类型身份。
- 无新增 feature/cfg；AsyncHooks 保持原 tokio JoinSet/限流/取消语义；路径审计及内容指纹保留原实现；Stop/Interrupt 解析及错误保持原行为。
- 测试源码没有迁移或修改：原测试位于 compat/{t34_tests,t35_tests,t36_tests,t37_tests,t38_tests,t39_tests,t40_tests,t41_tests,t42_tests,t43_tests,t44_tests}.rs 等父模块测试族，仍经薄转发验证同一实现；没有外部资源路径变化。

### 主会话公共接线

1. 根 Cargo.toml workspace members 注册 crates/libs/plugin-hooks。
2. crates/vw-agent/Cargo.toml 增加 plugin-hooks = { path = "../libs/plugin-hooks", version = "0.6.0" }。
3. compat/mod.rs 现有所有模块声明原样保留，原模块已薄转发迁出符号，不需删父模块声明。hook_manifest 保留任务17的 plugin-types 转发。
4. tools/context/mod.rs 的 AgentContextHost::AsyncHooks 原旧路径经 hook_async 重导出仍是唯一新类型，无需重写该关联类型；plugins/hook_handler.rs、plugins/hook_trust.rs 及 compat 原测试可保留现有导入。若改直引，hook_matcher、hook_async、hook_trust_content 及上述16函数一一对应 plugin_hooks::<同模块>::<同符号>，无需参数变化。
5. 保留 hook_stop::emit 的 unix NativeControlRuntime::shared().stop_context 清理，保留 hook_mcp::execute 的 new_hook_runner() 隔离构造；均未删改。禁止改回 HookRunner::new() 或新建 Outcome 类型。

### 未闭环宿主职责及具体前置

本次不猜造端口或越界迁移任务24/158/190/269。剩余文件仍含真实业务宿主适配，不能宣称全卡迁完：
- hook_command::{bind_input,retain_context} 读取宿主 current_tool_use_context，操作 ToolUseContext 与 plugin event；后续可用现有 tool_context::context::{ToolUseContext,current_tool_use_context}<H> 泛型继续下沉，事件须用 plugin_types::hooks::PluginHookEvent，不能复制类型。
- hook_compact::{emit,context} 依赖项目目录、租户配置与 execution_hook_runner 工厂；context 的配置解析/工厂属于宿主，emit 可用既有 ContextHost 泛型下沉。
- hook_session::{sessions,key,start,close} 持有静态 HashMap<String,Arc<宿主 ToolUseContext>>；租户 key 调用 session::ui_store::current_tenant_scope。现有 tool-context 是 ContextHost 泛型，没有可供本库直接持有的统一宿主类型。需要显式会话生命周期容器和宿主实例注入，保留 tenant/root/session 组合及全局唯一状态。
- hook_prompt::submit、hook_model::emit、hook_subagent::{emit,emit_boundary} 仍使用宿主具体 ToolUseContext/HookRunner，可沿真实 ContextHost<HookRunner=agent_hooks::HookRunner> 泛型迁移；未新增万能回调。
- hook_stop::emit 在执行前有 unix NativeControlRuntime 清理，后续拆出协议执行时须保留宿主清理顺序、abort_token biased select 与 Interrupt 3秒清理期限。
- hook_mcp::execute 仍调用 tools::mcp_common::load_resolved_acp_config_from、mcp_tool::{discover_mcp_tool_specs,mcp_tool_id,McpTool}、plugins::mcp_session_connections、tools::execute_tool_from_registry，且授权来源核对依赖 compat::mcp_auth::hook_source_matches。它们是工具发现/授权/实际执行边界，不能用 JSON 回调伪迁移或让 plugin-hooks 反向依赖 vw-agent；须在其真实所有者交付后继续闭环。

### 静态证据

- 6个完整模块与HEAD字节对比（仅 pub(crate)→pub）一致；上述16个函数体逐一与HEAD比较（仅可见性调整）一致。
- 定向 git diff --check 通过；新库 rg 无 vw-agent/vw_agent/crate::app/crate::tools/crate::plugins/crate::session 反向引用。
- 最小依赖：agent-hooks（Outcome唯一类型）、plugin-types（manifest）、anyhow、serde_json、regex、sha2、hex、tokio(rt/sync)、tracing。
- 当前 python3 无 tomllib，未成功执行静态 TOML 解析；清单逐项人工审阅，待主会话可用解析器复核。未运行 Cargo、单元测试及实机测试，待统一验收；不声称编译通过。


## 独占集成提交窗口复核

主会话现已授权本卡限定提交，覆盖上方“不暂存、不提交”的早期指示。根/宿主清单仅纳入 plugin-hooks 两行；独立索引提交，不收录共享索引或其他任务产物。主会话已完成根、宿主、新库3份 TOML 静态解析；早期 tomllib 缺失记录保留为历史证据。已复读全部拆分宿主适配及新库函数，移除 hook_session 迁移后不再使用的 Value 导入。保留已阻塞状态和全部未闭环职责。未运行 Cargo、单元测试及实机测试，待统一验收。
