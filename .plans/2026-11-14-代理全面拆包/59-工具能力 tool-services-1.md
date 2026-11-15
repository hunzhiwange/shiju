# 任务 59: 工具能力 tool-services-1

先阅读并遵守 [公共执行规则](plan.md)。

批次：【批次 1（依赖层级，无整批屏障）】 产物依赖：无；写入顺序：无

历史审计编号：原任务 186，仅用于对照 coverage.md、runtime-audit.md。

## 调度依据

- 无本计划内已确认前置；核对已有基线与本卡实际导出后即可进入就绪队列，不等待其它领域。
- 公共根清单、父模块、全局消息/状态和直接调用者由协调者逐项接线；公共文件冲突只串行化该次写入，不阻塞无关领域实现。
- 旧路径引用复核线索（不是迁移前置）：任务 65 的 `crate::agent::defaults`；任务 158 的 `crate::plugins::patch_input`；任务 303 的 `crate::session::runtime::dynamic_calls::ClientToolHost`。已有真实实现/窄适配可保持调用；只有必须消费未交付新接口时，记录符号证据后增加局部依赖。
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
- `crates/vw-agent/src/tools/apply_patch/apply_patch-cn.txt`
- `crates/vw-agent/src/tools/apply_patch/apply_patch.txt`
- `crates/vw-agent/src/tools/apply_patch/mod.rs`
- `crates/vw-agent/src/tools/apply_patch/tests.rs`
- `crates/vw-agent/src/tools/batch/batch-cn.txt`
- `crates/vw-agent/src/tools/batch/batch.txt`
- `crates/vw-agent/src/tools/batch/mod.rs`
- `crates/vw-agent/src/tools/batch/tests.rs`
- `crates/vw-agent/src/tools/client_tool.rs`
- `crates/vw-agent/src/tools/client_tool_tests.rs`

## 上下文

- 重点符号：`ApplyPatchTool`、`BatchTool`、`ClientTool`。
- 当前文本引用候选：`security`、`plugins::patch_input`、`patch`、`file`、`file::watcher`、`tools::apply_patch`、`tools::context`、`tools::traits`、`session::runtime`。这里只是导航证据，注释/测试引用不能作为真实依赖；实际执行须追踪use别名、super和调用者。
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


## 2026-11-15 当前基线审阅与局部前置

本轮遵守协调消息：当前 checkout，不建 worktree、不暂存或提交，不写公共清单、父模块和直接调用者；禁止 Cargo、测试与程序启动。仅更新本卡，未迁移实现，不宣称完成。

### 已核实的真实依赖

- `tools/apply_patch/mod.rs:15-18` 直接消费 `file_edit::types::{FileDescriptor, StructuredPatch}` 与 `file_edit::utils::{build_file_descriptor, build_patch_summary, display_path, snapshot_from_text}`；这些共享类型/函数仍在宿主且为 `pub(crate)`，`tool-files` 尚不存在。实际所有者是任务54，而非本卡；任务54卡中也已记录安全策略、外部目录技能状态的阻塞。不得复制这些类型或将 file_edit/utils 越界搬到 tool-services。恢复条件：任务54交付真实共享类型/函数导出及 external_directory::assert_external_directory 的宿主适配。
- 同文件 `ApplyPatchTool` 和文件内 legacy 实现都持 `Arc<SecurityPolicy>`，访问路径策略与动作预算；`agent-security/src/lib.rs` 当前没有导出此类型，仍由 `vw-agent/src/security/policy` 所有。不能仅将 import 改为 agent_security；须先有真实策略所有者或经协调确认的保留原语义窄适配。
- `tools/client_tool.rs:4,16,20,57-75` 直接持有、检查并调用 `session::runtime::dynamic_calls::ClientToolHost`：`owner()` 返回 `DynamicCallOwner`，包含 `SessionScope`、session、host；校验 tenant/session/canonical workspace 后调用 `request(&ToolSpec, Value, turn_id)`。定义仍在宿主 `session/runtime/dynamic_calls.rs`，由任务303覆盖。不得新造 SessionScope/Owner 或复制 pending/disconnect 状态。恢复条件：真实下层连接接口，或协调者确认该原签名宿主适配及唯一 owner 类型边界。
- `BatchTool` 的执行器有真实可用的泛型入口 `tool_context::executor::execute_tool_from_registry<H: ContextHost>`；current context 也可用同一 `AgentContextHost` 特化，因此这两项不构成缺失接口。尚需宿主公共适配：`toolset::boxed_registry_from_arcs` 为 `pub(super)`，其 ArcDelegatingTool 实现仍在宿主，不能绕过普通执行链改为直接 execute。迁移 BatchTool 需由协调者确定该已存在适配的归属及 `BatchTool<AgentContextHost>` 的接线，避免本卡越界修改 toolset/context。该项不能作为等待整个会话引擎迁移的理由。
- `plugins::patch_input` 已有真实所有者 `plugin_install::host_files::patch_input`，不是本卡阻塞；patch 算法已有 `patch` 库，文件与 watcher 已归 `agent-files`。

### 精确接线交接（恢复后实施，当前未写入）

1. 根 Cargo.toml 增加 tool-services workspace member；vw-agent/Cargo.toml 增加正向路径依赖。新库不得依赖 vw-agent。
2. `tools/mod.rs` 原 apply_patch、batch、client_tool 声明及导出需转发新库；BatchTool 若按现有 ContextHost 泛型实现，宿主别名必须特化同一 AgentContextHost，不能新增 task-local。
3. `tools/toolset/mod.rs` 的 BatchTool 构造保持同一 Arc 工具集合和 boxed_registry_from_arcs 的全部转发行为；普通 executor 路径必须保留 exposure、permission 与 hooks。
4. `session/runtime` 的 ClientToolHost 构造调用者仍持真实连接；客户端工具构造接线需保留 owner/tenant/session/workspace/turn 校验和取消断连语义。对应 `tools/client_tool_tests.rs` 的 ToolCatalog、SessionScope、defaults 依赖尚未解环，不能以删断言完成迁移。
5. 相邻资源 apply_patch/{apply_patch.txt,apply_patch-cn.txt}、batch/{batch.txt,batch-cn.txt} 随各自实现原布局迁移；apply_patch/tests.rs、batch/tests.rs、client_tool_tests.rs 保留独立文件及全部断言。apply_patch 文件内 legacy 实现也需保留其不同语义，不能与新版合并。
6. 待后续统一验收的具体路径为上述三组测试及 plugins/compat/t60_tests.rs、session/runtime/dynamic_calls_tests.rs；保留 wasm32 async_trait(?Send) 与非 wasm Send 的区别。本轮均未运行。

### 本轮静态证据

已读取任务、公共计划、来源实现、相邻测试、上下文/执行器及真实库导出；rustcodegraph status 与 explore 用于导航，当前工作区已有大量迁移变更，实际边界再以原文件核对。没有创建空目标包或复制宿主实现。仅本卡状态/证据更新，不创建任务文件专用提交；待协调者处理上述局部接口后重新调度。`git diff --check -- <本卡>` 通过；未运行 Cargo、单元测试及实机测试，不能声称编译或行为通过。


## 2026-11-16 领域迁移落盘，申请24号公共/Git窗口

54前置已解除。生产实现、4个txt资源及3份测试源码已迁至 tool-services 原职责目录。旧实现位置仅保留宿主特化别名，旧测试声明通过 include! 引用新位置，继续在真实宿主内验证，未删断言；include! 文件内的模块文档注释改为普通注释。cfg(any()) 历史实现保存到 apply_patch/obsolete.rs，仍禁用，不是第二个运行实现。

- ApplyPatchTool<H: FileToolHost>：复用 tool-files 公开的 FileDescriptor/StructuredPatch/utils/external_directory；SecurityPolicy 复用 agent-security，patch_input 复用 plugin-install，原 read-state 使用同一 H。
- BatchTool<H: BatchHost>：仍持原 Arc<Vec<Arc<dyn Tool>>>；有上下文时走 tool_context 泛型 executor，原 exposure/permission/hooks 路径不变。BatchHost 只要求宿主转发原 boxed_registry_from_arcs。
- ClientTool<H: ContextHost,C: ClientConnection>：tenant/session/canonical workspace/turn 校验仍在工具内。窄连接端口返回原 owner 的 tenant/session/directory 引用，request 保留 &Arc<Self> 接收者、&ToolSpec/Value/String 参数和真实 DynamicToolResponse；原 ClientToolHost 保持 pending/超时/取消/断连唯一状态，未改该文件。
- 测试调用需要的 ApplyPatchTool::schema/failure、BatchTool::schema/available_tool_names/find_tool 和 MAX_CALLS 提高可见性，其余业务行为不变。

### 本会话待窗口内执行的精确公共接线

1. 根 Cargo.toml workspace 新增 crates/libs/tool-services；vw-agent/Cargo.toml 新增 tool-services 路径依赖。禁止手写锁文件。
2. tools/context/mod.rs 实现 tool_services::BatchHost for AgentContextHost，唯一方法 fn boxed_registry_from_arcs(tools: Vec<Arc<dyn Tool>>) -> Vec<Box<dyn Tool>> 转发 super::toolset::boxed_registry_from_arcs(tools)，不可换 Arc 实例或省略 Tool 委托。AgentContextHost 同时须使用54已有 FileToolHost 实现，不能新建 task-local。
3. session/runtime/dynamic_calls.rs 为原 ClientToolHost 实现 tool_services::client_tool::ClientConnection：owner_tenant 返回 self.owner().scope.tenant()；owner_session 返回 &self.owner().session；owner_directory 返回 self.owner().scope.directory()；request(self: &Arc<Self>,spec:&ToolSpec,args:Value,turn_id:String) 精确转发 ClientToolHost::request(self,spec,args,turn_id)。保留原连接生命周期和错误。
4. tools/mod.rs 的原 apply_patch/batch/client_tool/client_tool_tests 模块声明继续有效；类型别名保持所有直接调用者构造签名。测试继续宿主内编译，tool-services 不直接声明宿主集成测试。
5. 获得24窗口后，本会话执行接线、静态复核、git-commit 技能及选择性提交，成功后删本卡。当前保留卡，尚未暂存提交。

静态证据：领域及本卡 git diff --check 通过；反向引用搜索只在宿主集成测试和 cfg(any()) 历史源码命中，生产库没有 vw-agent/crate::app 依赖。当前 python3 不含 tomllib 或 toml，清单解析尚未运行，窗口内需用已有可用解析器补充，不假报通过。未运行 Cargo、任何测试、程序或外部探测。后续待统一验收：迁入三组测试、plugins/compat/t60_tests.rs、session/runtime/dynamic_calls_tests.rs 和 wasm32/非wasm语义。


## 2026-11-16 公共源码接线完成，排队94号全局根/Git窗口

已在授权 tools/mod.rs 追加三项强类型宿主适配：BatchHost 转发原 toolset::boxed_registry_from_arcs；FileToolHost 读取现有 crate::skill::dirs().await（此前无该实现，避免重复）；ClientConnection 在工具组装层对真实 ClientToolHost 实现，三个 owner 引用及 request 精确转发原方法，未修改 session 连接实现文件。tools/toolset/mod.rs 的四处 ApplyPatchTool/BatchTool 构造由原类型别名保持同名同参数可达，不需改写 Shell/Browser/ID 分支。ApplyPatchTool 新库声明保留非 wasm cfg；Batch/Client 保持原 wasm async_trait(?Send)。

静态补证：4个 txt 资源与 HEAD 原文件逐字节相同；3个测试源码与 HEAD 原文件去除 //! 与 // 差异后相同，断言完全保留。静态 TOML 解析使用已有 uv Python 3.12 tomllib，无安装、无 Cargo。领域及获准宿主模块 git diff --check 通过。源码完整审阅已检查 owner 返回引用、request 的 &Arc<Self> 与同一 lifetime、真实 Future Send/wasm HostFuture 约束、批量 executor/Arc 集合、read-state 同一 H 以及资源 include 路径。

### 申请94窗口的精确增量

- 根 Cargo.toml：workspace members 新增 `crates/libs/tool-services`（按当前根目录已有成员模式插入）。
- crates/vw-agent/Cargo.toml：dependencies 新增 `tool-services = { path = "../libs/tool-services", version = "0.6.0" }`；核对54所需 tool-files 同步已有正向依赖，缺失时仅补真实 `tool-files = { path = "../libs/tool-files", version = "0.6.0" }`。
- Git：仅暂存 tool-services 全部、tools/apply_patch 原目录迁移删除及薄特化模块、tools/batch 同类迁移、tools/client_tool.rs、tools/client_tool_tests.rs，以及 tools/mod.rs 尾部本卡3项 adapter hunk（该文件另有50/97/51等在途变更，必须仅暂存本卡追加段）；根两清单同样仅暂存本卡 hunk，禁止整文件捎带。本卡完成后删除纳入该提交。
- 未取得94窗口，未修改根两清单、未暂存、未提交、未删卡；收到窗口后由原会话继续完成，不能交主会话。已读取 git-commit 技能，提交正文将列迁移映射、窄端口、资源/测试、cfg 和未运行项。


## 最终窗口阶段一：按共享块顺序提交

协调消息要求本轮仅纳入 tools/mod.rs 本卡 BatchHost 及直接接线，保留其后54 FileToolHost 与 ClientConnection 块。HEAD 尚无这三个块，Working 顺序为 BatchHost → FileToolHost → ClientConnection；限定入口只允许基于当前 HEAD 和 Working 同时可用的精确替换，不能跳过中间块插入后块而造成 Working 重复。因此本轮先提交领域迁移和 BatchHost，任务保持未闭环，保留本卡；待54 FileToolHost 进入HEAD后由本会话再通过入口提交属于59的 ClientConnection 精确块并删卡。禁止将领域产物提交冒充任务完成。


### 阶段一限定入口拒绝证据

已按最新指示仅准备BatchHost roots，排除54 FileToolHost和后续ClientConnection。候选根两清单静态解析及HEAD/Working锚点审阅通过。执行 `/tmp/shiju-scoped-root-commit.py /tmp/shiju59-config.json` 时，入口在私有候选索引 `git diff --cached --check` 安全拒绝：`crates/libs/tool-services/src/batch/mod.rs:260: new blank line at EOF.` 原先普通git diff --check未覆盖未跟踪新库，不能当作新文件通过证据。本次没有生成提交、未写共享根，保留本卡。遵照“安全拒绝时结束报告具体缺口”，本回合不重试、不修改helper或绕过。恢复时先移除该独占源末尾额外空行，再静态审阅并经同一入口重新执行阶段一，仍仅BatchHost、不夹带FileToolHost。

外部桌面索引快照逐条一致；入口原索引/CAS保护执行至候选检查即退出，证据保存在 `/tmp/shiju59-commit-evidence.json`。未删除本卡，未运行Cargo、测试或启动。


### 第一阶段重试

按授权最小移除batch/mod.rs末尾多余空行；新库全部文件逐行空白/末尾空行检查通过。重新核对最新HEAD与Working精确roots和两清单候选解析，保持只纳入BatchHost，不夹带54及后续ClientConnection，继续经原入口提交。
