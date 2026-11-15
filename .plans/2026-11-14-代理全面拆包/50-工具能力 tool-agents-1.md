# 任务 50: 工具能力 tool-agents-1

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 1（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：无

历史审计编号：原任务 170，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 无本计划内已确认前置；核对已有基线与本卡实际导出后即可进入就绪队列，不等待其它领域。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 1 的 `crate::project::instance::provide`；任务 63 的 `crate::session::child_agent::coordinator::mailbox`；任务 182 的 `crate::session::child_agent`；任务 190 的 `crate::tools::context`；任务 195 的 `crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected`；任务 269 的 `crate::session::ui_store::load_agent_session_scoped_for_tenant`；任务 303 的 `crate::session::runtime::SessionScope::new`；任务 305 的 `crate::gateway::ApiError`；任务 306 的 `crate::session::llm::recovery_test_support::RecoveryConfig::new`；任务 307 的 `crate::session::session::now_ms`；任务 310 的 `crate::session::task_session`；任务 312 的 `crate::session`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
- 执行时滚动调度，编号仅提供可行线性顺序；只等待本卡明确列出的前置完成并集成。在途旧编号按任务编号对照认领，禁止重复执行。

状态：已阻塞

目的：迁移，将本任务职责归入 `tool-agents`。

来源任务：用户要求尽可能全部拆分vw-agent；覆盖映射见coverage.md。

预计会话范围：一个领域的一组源码迁移；大领域已拆成连续任务，同包逐项接线。测试源码和资源仅随实现迁移，不执行。

## 并行实施边界

- 按 plan.md 的并行协议执行：独立工作树负责领域实现，主协调会话负责公共接线与逐项合入；禁止在同一检出目录同时执行并行任务。
- 本任务只修改下方来源文件、对应目标包及本编号任务记录。根清单、vw-agent 清单/lib.rs、清单外父模块和直接调用者由协调会话独占修改。
- 开始前检查当前基线的 use/super/宏及具体符号所有者。候选引用不是完整依赖图；发现需要另一在途任务的新接口时，报告具体符号，协调者将依赖方顺延，不猜签名或新造端口。
- 将公共接线逐文件列入本任务的“接线交接”记录，给出原/新导入、feature/cfg、注入位置和需删除的旧实现；不得仅交付泛化建议。
- 领域差异静态审阅后用 $git-commit 提交本工作树变更，保留本卡，报告提交与接线清单。此时仅为待合入产物，不标记已完成。协调会话接线、合入并复核后提交删除本卡，再归档已合入工作树。

## 文件

- 目标：`crates/libs/tool-agents/Cargo.toml`、`crates/libs/tool-agents/src/lib.rs` 及保持原职责布局的子模块；包已存在则修改。
- 公共接线（仅协调会话修改，本任务提供精确交接）：根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`、`crates/vw-agent/src/lib.rs`、相关原mod.rs与搜索确认的直接调用者清单。
- 来源及相邻测试/资源：
- `crates/vw-agent/src/tools/agent_tool/communication.rs`
- `crates/vw-agent/src/tools/agent_tool/communication_tests.rs`
- `crates/vw-agent/src/tools/agent_tool/mod.rs`
- `crates/vw-agent/src/tools/agent_tool/tests.rs`
- `crates/vw-agent/src/tools/delegate/agentic.rs`
- `crates/vw-agent/src/tools/delegate/agentic_tests.rs`
- `crates/vw-agent/src/tools/delegate/execution.rs`
- `crates/vw-agent/src/tools/delegate/execution_tests.rs`
- `crates/vw-agent/src/tools/delegate/mod.rs`
- `crates/vw-agent/src/tools/delegate/support.rs`
- `crates/vw-agent/src/tools/delegate/support_tests.rs`
- `crates/vw-agent/src/tools/delegate/tests.rs`
- `crates/vw-agent/src/tools/session_tool/mod.rs`
- `crates/vw-agent/src/tools/session_tool/tests.rs`

## 上下文

- 重点符号：`AgentToolChildSessionCreated`、`AgentToolChildSessionCreatedCallback`、`AgentTool`、`execute_agentic`、`execute`、`DelegateTool`、`NoopObserver`、`SessionTool`。
- 当前文本引用候选：`config`、`session::child_agent`、`providers`、`gateway::api`、`gateway`、`session::ui_store`、`agent`、`tools::context`、`util::filesystem`、`agent::loop_`、`approval`、`hooks`、`tools::current_tool_use_context`、`tools::delegated_tools`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
- 领域边界：具体工具按能力分包，工厂由 tool-assembly 组合；涉及会话、插件与调度器的工具只依赖前置端口，不反向依赖会话或主循环。

## 步骤

1. 读取上述源码、父模块声明、相邻测试和调用者；用 `rg -n 'crate::|super::|pub\(crate\)|include!' crates/vw-agent/src` 限定到本任务查看实际依赖，记录旧路径到目标模块映射。测试文件单独排程时按其生产实现实际所有者迁入，不为测试再建包。
2. 按领域边界迁移本任务源码；剥离应用宿主动作到已存在的窄接口，更新资源路径与必要可见性。禁止新增未调用抽象，不合并行为不同的旧实现。
3. 建立/扩展目标最小清单，复用已拆出的真实所有者；完整转发原feature/cfg。将调用者正向接线和旧位置薄重导出列入交接，由协调会话实施。后续任务尚未迁的调用者通过真实宿主适配接线，不让新包依赖它。
4. 逐项审阅参数/错误/异步取消、租户、权限、全局状态、序列化与平台语义等价；仅调整测试源码导入，不运行或追加测试。将尚需闭环迁移的宿主适配具体路径写入本任务提交正文。
5. 按并行实施边界提交领域变更与接线交接，保留本卡供协调会话合入后收尾；不得提前宣称任务完成。

## 静态验收

- 运行：`git diff --check`；逐文件阅读本任务完整差异和迁移后的清单。
- 运行：`rg -n 'vw-agent|vw_agent|crate::app' crates/libs/tool-agents`；审阅是否存在库对主包的反向依赖，文档与测试引用单独检查。
- 检查：本任务全部来源已迁/明确为组合转发，生产实现不重复；类型/状态只有一个所有者；工厂注册和资源完整；后续文件已分配所有者。
- 所需证据：真实迁移映射、最小依赖、窄端口调用者、cfg/feature、直接调用接线、测试/资源迁移清单及差异检查结果。仅静态证据，不能声称编译或行为通过。
- 待统一验收：本任务来源的既有测试、直接调用集成、原支持平台与特性、涉及UI的渲染和外部能力；将具体路径及组合写提交正文，本轮全部禁止运行。


## 本次静态调查（2026-11-15）

执行约束：按主会话转达的最新用户要求，在当前 checkout 工作，不创建 worktree，不运行 Cargo/测试，不暂存或提交；共享文件归主会话。未修改领域源码，未创建空目标包，保留全部原实现及测试。

### 已核实的接口缺口

- `agent_tool/mod.rs:9-12,81,213-274,495-522` 直接持有 `ChildAgentCoordinator`、`CreatedChildAgentRun`，调用 `create_with_fork/run_created/cancel_created`，后台线程访问协调器的 continuation、turn cancellation 和全局状态。`communication.rs:3,91-154` 还调用 mailbox、team snapshots、followup 和 activity wait。这些真实所有者仍在 `session/child_agent.rs`、`session/child_agent/coordinator.rs`，后者及 created 类型为 `pub(crate)`；不能通过依赖现有库取得。来源归当前任务 63、182，不在本卡写入范围。
- `session_tool/mod.rs:12-16,159-174,241-246,429-441` 直接使用 `TaskSessionCoordinator/TaskSessionSnapshot/TaskSessionState`、全局协调器、可见任务持久化及 tenant scope。协调器仍在 `session/task_session.rs`（当前任务 310），类型为 `pub(crate)`。现有 `session-port::SessionQuery` 有生命周期查询端口，但没有任务协调器、任务持久化和租户作用域执行端口，不能仅替换 import。
- `session_tool/mod.rs:5-8,30-40,52-58` 的流执行签名返回宿主 `GatewayChatStreamCollected` 和 `ApiError`；collected 结构及收集函数仍在 `gateway/api/handlers/session/stream.rs:558,1994`，属于当前任务 195。现有 `gateway-chat-stream` 是网络 SSE 客户端，不导出该服务端收集结构，不能按包名猜替代接口。
- `delegate/agentic.rs:10,52,81` 直接调用宿主 `run_tool_call_loop` 和 `delegated_tools::build_agentic_tools`，分别归当前任务 243、178。现有 `agent-runtime` 是 Native/Docker 工具环境，不是模型工具循环。`delegate/execution.rs:17` 还直接创建 provider；provider 工厂虽已有库，单独替换该调用不能解开整个 DelegateTool 的宿主依赖。

以上为本卡真实类型/动作缺口，不把所有旧路径引用一律视为前置。已检查 `.rustcodegraph` status/explore，以及当前 `session-port`、`agent-runtime`、`gateway-chat-stream`、`agent-providers` 导出；全库静态搜索未发现可复用的上述协调器、工具循环或 collected 类型。替代路径审阅：SessionQuery 只能覆盖生命周期查询；网络 SSE 客户端不能替换服务端收集；整目录平移会反向依赖宿主，复制协调器会产生双状态，均不采用。

### 恢复条件及接线交接

需要先由上述所有者交付可复用契约，或协调者明确安排强类型窄宿主适配的定义/写入范围；不能在本卡越界搬走其他任务源码或猜测未来接口。尤其应保留 child run 的取消、后续轮次、permit Drop、tenant scope、fork 权限及任务状态持久化语义。

接口就绪后，目标映射为 `tools/agent_tool/* → tool_agents::agent_tool::*`、`tools/delegate/* → tool_agents::delegate::*`、`tools/session_tool/* → tool_agents::session_tool::*`，本卡全部14个实现/测试文件随迁；`agent_tool/mod.rs:911` 引用的外部 `session/child_agent/collaboration_tests.rs` 仍归任务63，需由协调者安排宿主集成测试接线，不能复制为库单元测试。

公共接线尚未实施：根 workspace 加入 `crates/libs/tool-agents`；`vw-agent/Cargo.toml` 增加路径依赖；`tools/mod.rs:28,54,113` 的三个模块改为薄重导出，保留既有 `AgentTool/DelegateTool/SessionTool` 导出。`AgentToolChildSessionCreated`、callback 及 builder 若跨包供宿主调用须显式公开。进一步直接调用者与构造器注入须待真实端口确定后提供精确参数，当前不伪造可编译清单。两个 Tool impl 原有 wasm32 `async_trait(?Send)` 与非 wasm Send 分支必须保持；测试源独立、原断言保留。

静态证据：本次仅更新此卡；未运行 Cargo、单元测试及实机测试，未声称迁移/编译通过。无领域变更，不创建任务记录专用提交。后续建议协调者优先明确 child/task runtime 契约及委派循环宿主注入边界，再恢复任务50。


## 恢复与公共写入窗口申请（2026-11-16）

协调者已授权按真实调用定义强类型窄宿主适配；不再把缺少预成型端口视为必须等待其他整卡完成。当前阶段待公共文件窗口，旧阻塞调查作为依赖证据保留。本轮再次读取实际 child run、coordinator、task session、session tool background turn 与 stream collected 结构，未修改生产文件，未暂存/提交。

接线前须一次性确认以下公共写入范围（窗口属115，任务50自行实现、静态审阅及提交）：

1. `crates/libs/session-port/src/lib.rs`、`Cargo.toml` 及新增 `child_agent.rs`、`task_session.rs`：下沉 child/task 纯输入、状态、快照、wait outcome、fork选择与 mailbox 消息契约，公开字段保持原 serde 和错误语义。`CreatedChildAgentRun` 包含 hook context、原子状态及内部 continuation，不下沉；由宿主关联类型或强类型 run handle 持有。协调器实现及全局 OnceLock 继续留原位置，端口只能调用原实例。此项涉及契约共享所有者，不能未经窗口在目标工具包复制类型。
2. `crates/vw-agent/src/session/child_agent.rs`、`session/child_agent/coordinator.rs`、`session/child_agent/history.rs`、`session/child_agent/mailbox.rs`、`session/task_session.rs`：仅改上述纯契约导入/重导出，保留 create_with_fork、审批、取消、prepare_followup、状态机和持久化动作原实现。宿主端口适配就近新增，不移动这些业务实现。涉及任务63/182/310，需协调者锁定相应文件。
3. `crates/libs/session-port/src/chat_stream.rs` 与宿主 `gateway/api/handlers/session/stream.rs:558`：仅将三个字段 `output: String`、`message_id: Option<String>`、`parent_message_id: Option<String>` 的 `GatewayChatStreamCollected` 下沉为公开返回契约并在宿主重导出。保留 Debug/Clone；collect/commit/persist 全部原地。此项记入任务195接线记录。错误通过强类型宿主关联错误保留，不转换为任意 JSON 或吞掉 ApiError。
4. `crates/vw-agent/src/tools/mod.rs` 与新增 `tools/agent_hosts.rs`：分别将 agent_tool/delegate/session_tool 改为目标实现的宿主具体类型别名或薄工厂，并注册三个窄适配。Child adapter 包装原 coordinator 和 CreatedChildAgentRun；task adapter 包装原 task coordinator、ui_store 持久化、tenant scope 与服务端 collector；delegate adapter 只转调原 build_agentic_tools/run_tool_call_loop。所有原参数含安全策略、可信技能、迭代上限、观察器保持，不重新实现循环。不能用整工具 execute(Value) 回调替代工具实现迁移。
5. 根 `Cargo.toml`、`crates/vw-agent/Cargo.toml`：注册 tool-agents workspace member 和路径依赖；新库最小依赖依据实际 imports，复用 agent-providers，不反向依赖 vw-agent。根锁文件不手写。
6. 生产构造调用已定位：`tools/toolset/mod.rs`（AgentTool 两处）；`tools/thread_tool/mod.rs`（SessionTool 四处）。优先宿主类型别名/薄工厂保持构造签名，若泛型/注入需要调整这些文件，则纳入同一窗口；原 tests 与外部 collaboration_tests 按真实宿主适配保留集成覆盖，不使用空循环替代原断言。对应直接调用者修改范围需锁定后才能写入。

授权独占文件仍为本卡14个来源、tool-agents目标和此卡。窗口申请就绪；本轮按调度指令结束回合，不轮询。尚未获得共享契约写入窗口前不创建无调用者端口或空包。窗口授予后完成迁移、静态 TOML 解析、全差异审阅、逆向依赖搜索及 diff --check，再按 git-commit 技能使用中文信息提交；未运行Cargo、单元测试及实机测试。


## 本窗口实际产物（2026-11-16，未闭环）

已将16个真实输入/返回/状态类型移至 `session-port::{child_agent,task_session,chat_stream}`，旧6个宿主文件仅重导出并删除不再使用的 serde 导入。`session-port` 新增已有 `config-agent` 路径依赖，承接 ChildAgentRunInput 的技能策略强类型。协调器、OnceLock、CreatedChildAgentRun、hook、continuation、permit Drop、tenant scope、持久化与collector全部保留原实现；没有新造端口或JSON执行回调。任务63/182范围仅child纯类型，310范围仅task纯类型，195范围仅Collected三字段纯返回契约，均保留原路径重导出。

静态证据：16个定义与HEAD逐块比对，除pub(crate)改为pub外一致；17个可达本地清单静态TOML解析成功，依赖图无环；限定路径 diff --check 通过。Python系统解释器缺少tomllib，改用既有UV Python3.12解析成功，没有安装依赖。未运行Cargo、单元测试及实机测试，未更新锁文件。

未完成项：本卡14个来源的工具实现及测试仍在宿主，尚未创建tool-agents，也未实现三个强类型宿主适配或更改工厂构造。本提交仅可复用契约前置，不代表任务50完成，不能删卡或释放任务后继。此前“必须等待其它整卡”的阻塞判断由宿主适配授权解除；剩余为本任务实施工作，不能虚报外部阻塞。静态检查不证明编译通过，后续应继续工具迁移与接线，并复核实际类型唯一所有者、构造注入和测试源码归属。
