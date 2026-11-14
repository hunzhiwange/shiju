# 项目组合 macOS 交付证据

## 任务 28：Claude 组合闭环静态盘点（未验证）

2026-11-14：按本轮覆盖要求，仅完成源码与依赖盘点及必要交付合同；未运行 Cargo、构建、检查、单元测试、实机测试或最终回归。未启动 Gateway、未调用 SDK、未创建隔离 HOME 或配置产物；无测试数量、运行退出码、平台版本或脱敏前后差异证据。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。编号卡按要求删除，不表示 Claude 闭环已实现或验收通过。

直接依赖任务 27 已有桌面进度实现 `vw-desktop/src/app/message/project_profiles_progress.rs`；其回执绑定 operationId、profileId、group，缺少有效回执时保持未知结果，不重发应用。对应提交 `28e326c1b`、`f44508351` 可追溯。当前执行端仍存在以下缺项，桌面产物不能代替执行端交付：

| 验收环节 | 当前源码入口（路径相对 crates） | 静态结论及后续交付条件 |
| --- | --- | --- |
| 四类实际状态快照、创建 A/B | vw-agent/src/gateway/api/handlers/project_profiles.rs | create 的四个读取回调均返回 Unavailable；需领域所有者接同一授权项目/主机/组的有效状态读取，不能用目录期望值替代 |
| 重拍 | vw-agent/src/gateway/api/handlers/project_profiles_maintenance.rs | Resnapshot 明确返回 snapshot_unavailable；需接真实读取及版本事务 |
| 预览和应用 | vw-agent/src/gateway/api/handlers/project_profiles_execution.rs | 授权、槽归属、实体/版本核对后分别返回 503 profile_observation_unavailable / profile_execution_unavailable；尚未接 prepare_switch、四类执行回调和结果持久化 worker |
| 进度、取消和断线恢复 | 同上；vw-gateway-client/src/client/project_profile_api.rs | 服务端操作查询/取消返回 404；SDK 已有独立方法，但成功载荷尚未冻结，需绑定租户/项目/主机/组的操作注册表与本次操作终态，不能复用最近组结果作回执 |
| 直连/代理保持、自动保存和跨组隔离 | vw-agent-apps/src/project_profiles/{apply_provider,autosave,apply}.rs | 有领域回调和编排合同，生产 API 尚未接通；仅阅读函数不能确认配置恢复或代理发布成功 |

生产注册静态可追溯至 handlers/agent_apps/mod.rs → project_profiles::router，并由 gateway/router.rs 和 gateway/bootstrap.rs 合入；实际 SDK 路径为 `/v1/agent-apps/project-profiles`。未新增第二套路由、观察器或占位测试，拟定的 profile_macos_tests.rs 本轮未新增。

来源 `vw-agent-apps/upstream/src-tauri/tests/profile_roundtrip.rs` 的 `profile_snapshot_apply_roundtrip_restores_configuration` 定义四类引用恢复与 Desktop 独立断言；`profile_switch_in_routing_mode_changes_the_route_only` 定义代理模式保持、有效路由切换、直连指针保留及退出代理恢复。来源测试直接调用旧 ProfileService，不经过本仓库 Gateway/SDK，不能作为目标闭环证据。

解除验证禁令且上述生产入口交付后，独立 macOS 场景 `profile_t28_macos_claude_roundtrip` 应通过真实 Gateway/SDK，在可恢复隔离 HOME、临时项目及动态端口下创建 A/B，逐项断言供应商、MCP、技能、提示词恢复；离开 A 前改变配置，再返回 A 核对自动保存，仅 Claude 组改变。直连与代理分别覆盖，代理下同时核对有效路由、直连指针及原生代理配置保持；比较 Desktop/Codex 快照、指针与文件前像，补权限拒绝、版本冲突及失败不冒充 applied 的断言。环境变更持既有环境锁，原生客户端窗口核验仍交用户。后续通过仓库 cargo_slot.py 执行原卡的 profile_t28_ 聚焦测试，记录实际槽位、两个绝对缓存路径、命令/退出码/有效数量、macOS/CPU 与 Gateway/SDK 版本，以及脱敏路径与差异；不得记录密钥、令牌或敏感正文。

## 任务 11：应用供应商槽位

2026-11-14：已实现指定组供应商槽位编排与模块注册，合同见 [profile-provider-apply.md](profile-provider-apply.md)。null 不调用领域入口；同一有效供应商不调用写回调；缺失引用逐项 Missing。切换保留当前模式，核对实际目标、attached、stack 和非活动指针；发布待恢复不能返回成功。文件投影、登录事务、路由发布与权限仍归原领域执行端，生产回调由后续应用编排接线，当前不构成平台闭环。

静态阅读了任务 10 预览、模式观察、三个应用原生事务及 Claude 路由发布。按本轮用户覆盖要求，未编写或执行测试，未执行 Cargo、构建、检查、实机测试或最终回归；无 Cargo 槽位，两个缓存环境变量未设置。无编译或测试通过结论。后续补充并执行合同列出的 profile_t11_ 场景，再验收 macOS 实际事务与路由。

## 任务 1：领域接入契约

2026-11-14：契约文档编写完成，见 [profile-contracts.md](profile-contracts.md)。本条仅记录静态盘点，不代表 macOS 功能验收通过。

- 已阅读计划、任务 1、来源映射和母计划执行分线；逐项提取任务 2–30 的文件落点和实际父模块。
- 来源 `git -C /Users/xiangmin/worker/code/cc-switch rev-parse HEAD` 退出 0，得到 `ca58c0313360eae7c851fbd7553c1614c7af03e7`。
- 已静态核对三组 ProfileScope、未拍选择/切走补拍、项目作用域与路径设置所有者、四类领域入口、主机策略和锁顺序。来源到后续任务的对应关系见契约首表。
- 依赖缺项与下一步见契约“当前生产入口与缺项”；组合 DTO/API/SDK/Iced 尚待本计划实施，Desktop MCP 等不支持能力不能伪装成功。
- 用户本轮明确禁止执行检查、构建、单元测试、实机测试和最终回归，因此任务卡的 Python 关键字检查也未运行。没有测试通过数、编译通过或平台验收结论；以上只为源码阅读证据。
- 后续推荐先实施任务 2 的分组/快照 DTO；验证禁令解除后补各卡聚焦验证，28–31 再汇总 macOS 闭环证据。

## 任务 2：分组与快照 DTO

2026-11-14：实现完成，未验证。`vw-api-types::project_profiles` 定义三个受限组、独立分槽、组合 ID/实体/版本、四类引用快照及逐项/整体应用结果；已在 crate 根注册。复用 AppId 映射，拒绝不支持的组及未知载荷字段，Desktop wire key 为 `claude-desktop`。

- null/省略槽位表示不动，空 MCP/技能集合表示清空启用集，空提示词 ID 表示明确停用；任意非 null 类别表示该组已拍摄。`merge_scope_from` 只替换指定组。
- 技能引用保存 installationId，app_id 由其所在的组确定，两者共同定位安装；供应商/MCP/提示词保存领域 ID，启用集合表达启用位，不复制密钥或正文。未拍组和已拍空组可区分。
- selected 与 applied 分离；逐项状态包含成功、缺失、冲突、拒绝、失败、取消、不支持。调用者只有确认全部所需步骤后才可返回 applied；取消保留已提交结果。
- 已编写 5 个 `profile_t02_` 独立测试，覆盖 null/空集合、三组映射和独立覆盖、空提示词、非法组拒绝、部分成功序列化。源码审阅不等于测试通过。
- 按本轮明确覆盖要求，未执行 Cargo、构建、检查、任何测试、实机验收或最终回归；未领取 Cargo 槽位，CARGO_TARGET_DIR 未设置，无测试通过数量或退出码。后续解除禁令后通过仓库 cargo_slot.py 运行任务卡原定聚焦测试。

## 任务 14：应用提示词槽位

2026-11-14：实现指定组 `project_profiles/apply_prompt.rs` 与 `PromptsStore::apply_profile_prompt` 执行端适配。null 不读写；空串停用；相同激活项不重写；目标缺失返回 Missing，保留当前选择。读取在原生槽锁内取得目录 revision 和文件前像；激活调用母任务 13 的 activate_ordinary，停用调用 reconcile_ordinary，保留原事务、授权和冲突检查。无 current 标记或快照写入。调用者仍须完成执行主机路径解析、项目与物理槽授权及外层操作串行化；尚未接生产应用 API，不构成平台闭环。

按用户本轮覆盖要求，仅写代码及必要合同，未编写或运行测试，未执行 Cargo、构建、检查、实机或最终回归。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。实现未经编译或行为验证。后续需验证 profile_t14_ 的 null、空串、重复应用零写入、缺失引用保留、版本/原生前像冲突、授权拒绝与组隔离，再验收 macOS。

## 任务 19：列表、当前状态与创建入口

2026-11-14：新增 GET/POST `/agent-apps/project-profiles` 及 GET `/agent-apps/project-profiles/current`。camelCase 参数为 projectId、targetHostId、scope，创建另需 name。租户只取认证 Extension，执行主机只取现有本地主机策略授权结果；不接受请求目录或租户。scope 仅 claude、claude-desktop、codex，未知字段拒绝。策略 grant 新增独立 `project_profiles_read`、`project_profiles_create` 项目 ID 数组，默认空；不从 workspace/provider 权限推导组合权限。列表实体按契约在租户内共享，当前标记额外检查项目占用。无授权在构造存储前拒绝，坏记录整次返回 profile_state_invalid，不伪装空快照。所有响应 no-store。

缺项：任务 7/8 提供同步快照回调合同，但四类实际状态读取尚无完整统一生产适配。创建调用现有 create_from_current 服务，读取明确返回 Unavailable，HTTP 503 snapshot_unavailable，零实体写入；当前不能成功创建，不能据此宣称任务故事或平台闭环完成。后续由领域所有者接入已授权实际状态读取，再补至少四个 profile_t19_ 场景（权限/租户/项目拒绝、非法 scope 零读取、坏记录、成功创建不激活），不得以目录期望状态替代原生有效状态。

遵照本轮覆盖仅写代码及合同、删除编号卡；未编写或运行测试，未执行 Cargo、构建、检查、实机测试或回归。未领取槽位，CARGO_TARGET_DIR 和 CARGO_BUILD_BUILD_DIR 未设置。没有编译、行为或验收通过结论。

## 任务 18：领域进度与安全取消

2026-11-14：实现完成，未验证。任务 17 的 `with_operation` 锁与任务 16 的应用汇总/结果存储入口已静态核对。`ProfileApplyOperation` 复用母任务 2 的 Operation/start_task/watch 生命周期，单次提交，取消仅设信号，终态等待工作 future 完成。领域进度只发布 Provider/Mcp/Skill/Prompt 枚举；句柄由后续已授权操作注册表保留，不依赖 HTTP 订阅连接。应用调用者须将句柄传入 ProfileOperation::apply_prepared，并完整等待同一持锁 blocking worker，不能 abort 或脱离后台写入。

汇总器在每个领域回调完成后保存已完成项，下一领域启动前检查取消；取消保留已提交项、记录 cancelled、不启动余项、不写 applied。领域返回 Cancelled 也停止后续领域。末领域完成后再检查取消，未拍组也支持取消。存储失败返回错误，生命周期为 Failed，保留原有保守非 applied 记录。application_result 沿任务 16 的租户/项目/主机/组约束读取持久结果，断线不重放工作。当前边界为整个领域回调，不能中止正在执行的领域事务；MCP/技能回调内部逐项事务本轮不新增中断入口。生产 API/操作注册表接线仍由后续任务完成，不宣称平台闭环。

按用户本轮覆盖要求仅写代码及必需合同，未编写或执行测试，未运行 Cargo、构建、检查、实机或最终回归；无 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。没有编译或行为通过证据。后续补 profile_t18_ 聚焦场景：明确边界取消后余项零启动且成功项保留、最后领域取消不记 applied、订阅断线终态可读、持久化失败、重复提交及权限隔离。编号卡按本轮指令删除。

## 任务 22：组合维护 SDK（实现完成，未验证）

`vw-gateway-client::project_profile_api` 提供 `ProjectProfileApiScope`、结构化错误与 current/delete 响应；GatewayClient 的 `project_profile_list/current/create/rename/resnapshot/delete/clear` 对接任务 19/20 的 `/v1/agent-apps/project-profiles` 生产路由。列表与 current 使用 query，写操作使用 JSON（含 DELETE）；发送 projectId、targetHostId、scope，实体修改带 expectedVersion。租户必须匹配 endpoint，组合 ID 按路径段编码，current 的 profileId 保留 null。实体复用共享 ProjectProfile DTO；客户端不持有业务存储、不读取本地配置。

沿用 native_transport 禁止重定向与重试，保留 HTTP 状态和完整 JSON 错误（非 JSON 保留文本），与传输、解码、客户端参数错误分离。创建和重拍当前仍由服务端明确返回 snapshot_unavailable，SDK 不将其替换成成功。读取与维护沿既有 endpoint 鉴权；不新增权限或令牌所有者。

按本轮用户覆盖要求，未新增或执行测试，未运行任何 Cargo、构建、检查、实机或回归；无测试通过证据。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续需补充 profile_t22_ 本地 HTTP 场景并验证范围/主机编码、版本、null 当前标记、结构化错误及写入不重放。

## 任务 24：桌面组合列表与当前状态（实现完成，未验证）

支持 Claude、Claude Desktop、Codex 的应用详情分别读取任务 22 SDK 的 list/current；展示当前组合、未拍快照和名称搜索。其它应用不挂载入口；读取失败保留明确错误及重试入口，不把失败当空列表。当前标记只表达已选择，不声明已应用。切换/关闭详情、离开设置、重连及更换项目/主机时失效；响应同时核对网关、租户、项目 ID/路径、主机、目录代次及读取代次。复用 settings 输入与按钮样式、默认字体及既有页面滚动容器。后续应用/维护动作由任务 25/26 接入。

按用户覆盖要求只写代码和必要契约，未新增或运行测试，未执行 Cargo、构建、检查、实机或回归。仅静态核对接口与接线；没有验证通过证据。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续验证三组隔离、名称搜索、未拍快照、失败重试、过期响应拒绝和 macOS 深浅主题实际渲染。

## 任务 25：桌面组合维护弹窗（实现完成，未验证）

2026-11-14：创建、改名、当前组重拍、删除确认及清标记接入任务 22 SDK；改名、重拍、删除携带读取版本。共享组合操作由服务端执行，不在桌面复制配置或密钥，不实现应用。列表操作打开统一 settings 弹窗，空白名称禁止提交，处理中禁用输入、确认和取消，关闭消息在处理期间忽略；失败保留输入并使用错误 toast，允许重试，版本冲突提示取消后刷新。成功确认后重新请求 list/current，不乐观修改实体或标记。响应复用网关/租户/项目/主机/目录与请求代次校验，状态失效同时关闭维护弹窗。统一 modal layout 保留固定标题/操作、4px 滚动条与两侧 7px 留白，使用默认字体和主题样式。

已静态阅读任务 24 状态、SDK 的五类维护接口及统一表单契约；创建/重拍仍可能由服务端返回 snapshot_unavailable，UI 明确报错，不能宣称成功闭环。按本轮覆盖要求仅写代码及必要合同，未新增或运行测试，未执行 Cargo、构建、检查、实机或最终回归。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置；无编译或测试通过证据。后续需验证 profile_t25_ 的服务端确认顺序、空名称、失败重试、删除确认、忙碌态与过期响应，并检查 macOS 深浅主题和真实字形渲染。编号任务卡按用户要求删除。

## 任务 26：桌面预览与单次应用（实现，未验证）

2026-11-14：列表接入统一 settings 预览弹窗；预览读取成功后才开放确认，提交立即锁定，返回失败或未知响应不重发、不显示成功。响应按网关/租户/项目/主机/目录代次/组合读取代次隔离；显示后端逐项差异、未拍和空集合语义，以及完整应用结果和局部失败明细。只接受匹配组合和组的同步最终结果，启动响应、操作 ID 和持久化的旧组结果不能充当本次成功凭证。维护与应用弹窗互斥。

生产依赖限制：任务 23 SDK 的 preview/apply 为未冻结 JSON，当前 API 拒绝执行，尚无可用生产闭环。桌面对预览仅接受严格 `group/captured/items` 对象，item 为 `category/reference_id/action`，action 为 select/enable/disable/missing；未知形状直接禁止应用。这是消费端预留形状，不能宣称已冻结公共协议。应用仅接受 ProjectProfileApplyResult；异步 admission/progress 接线仍待生产契约交付。返回后重读 list/current、失效目录缓存和提示词状态；四类原生状态完整自动刷新尚未完成，必须在执行端观察入口交付后补接，不以本地推断替代。

按本轮要求未新增或运行测试，未执行 Cargo、构建、检查、实机或回归；未领取槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置。后续需补 profile_t26_ 单次提交、未知响应、过期响应、局部失败与四类刷新测试，及 macOS 深浅主题真实渲染。编号卡按用户覆盖要求删除；实现不代表验收通过。


## 任务 30：Codex 隔离与空快照（静态盘点完成，未验收）

2026-11-14：按本轮覆盖要求仅盘点必需契约；未新增测试或生产代码，未执行 Cargo、构建、检查、单元测试、实机测试或最终回归。任务卡删除表示本轮静态工作收尾，不代表平台验收通过。未领取 Cargo 槽位，CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR 未设置；无测试数量、退出码或通过证据。未启动隔离 HOME、Gateway 或原生客户端，因此 macOS/CPU、运行版本、实际配置路径及脱敏前后差异均未采集。

直接依赖任务 27 的提交 `28e326c1b`、`f44508351` 和 [领域合同](profile-contracts.md) 已静态核对：桌面保留原 operationId 查询/取消，身份包含网关、租户、项目与执行主机；服务端尚无对应可受理的生产操作。`gateway/api/handlers/project_profiles_execution.rs` 对有效实体/版本的 preview/apply 分别返回 503 `profile_observation_unavailable` / `profile_execution_unavailable`，operations/cancel 返回 404 `profile_operation_not_found`。创建/重拍的实际快照读取亦未交付；不得将 API 注册、桌面轮询或领域闭包存在视作应用闭环。读取与写入适配、操作注册和授权目标上的实际投影必须先交付，再恢复本故事验收。

来源 `cc-switch/src-tauri/tests/profile_roundtrip.rs` 的 `shared_profile_sides_are_isolated_and_mergeable` 与 `profile_apply_reports_dangling_references_and_continues` 已阅读，未运行。来源覆盖分组独立、补拍保留另一组和悬空引用后有效项继续；不能据此证明目标实现的 Codex 文件投影、租户/主机/项目隔离或 applied 状态正确，尤其不能照搬来源 warnings 后仍设 current 为全部成功。

| 待验行为 | 当前静态落点与契约 | 恢复验证时必须取得的证据 |
| --- | --- | --- |
| 只拍 Claude 后应用 Codex | `project_profiles/apply.rs` 区分未拍组；各槽位只读取指定组 | 未拍 Codex 不调用四领域读写，返回 uncaptured；Codex 原生文件不变，另外两组不变 |
| null 与显式空集合 | `apply_mcp.rs`、`apply_skills.rs` 对 null 提前返回，对空集合计算停用差异 | null 零读写；空集仅停用目标组已启用项；无关原生配置保留，不能以数据库启用位替代文件观察 |
| 提示词停用与重复应用 | `apply_prompt.rs` 区分 null、空串、已有激活项；由提示词所有者发布 | 空串经原事务停用，相同选择不重写；缺失引用保留当前提示词，前像/版本冲突明确失败 |
| 悬空引用部分失败 | MCP/技能逐项 Missing 并继续，提示词/供应商缺失明确返回；`apply.rs` 汇总 partial | 有效项真实生效，缺失项及失败状态可见，不能记 applied；失败不扩大到其它组/项目/主机 |
| 三组切换与作用域 | `profile_store.rs` 校验执行主机、租户及同主机/组的项目占用；任务 27 按原身份查询 | 在隔离项目和主机身份下反复切换三组，拒绝越权与他项目占槽；自动保存只替换旧组合当前组，快照与指针均不串组 |
| 重复应用稳定 | 四槽编排按实际状态计算最小差异 | 连续两次应用对比实际文件和领域写入次数；第二次无冗余写入，保留未托管内容，不能只比较响应或 current |

后续先完成上述生产依赖，再新增并注册独立 `gateway/api/handlers/profile_macos_tests.rs` 的 `profile_t30_macos_codex_scope_and_empty_snapshot` 场景；本轮未创建该文件，也无有效测试证据。解除验证禁令后，通过仓库共享槽位入口执行原定 `cargo test -j 1 -p vw-agent --lib profile_t30_ -- --test-threads=1`，记录实际槽位、绝对缓存路径、有效数量及退出码。测试使用可恢复的隔离 HOME/项目和既有环境锁，所有路径由执行主机环境解析；不得改用户当前配置、打印密钥或把模拟编排当实测。另行记录实际平台/版本和脱敏文件差异，原生客户端读取与窗口验收交用户完成。


## 任务 29：macOS Desktop 独立分组（静态盘点完成，未验证）

2026-11-14：按本轮覆盖要求，纯验收任务仅补充必要静态盘点。已阅读只读 plan.md、任务卡、领域契约和执行分线，使用当前共享 checkout。未新增测试或改动业务代码；编号任务卡按本轮要求删除，不表示 macOS 验收通过。

直接依赖任务 27 的实现提交为 `28e326c1b`，切页保留操作的补充为 `f44508351`；已静态核对 `project_profiles_progress.rs`、任务 27 消费者契约及生产 API。进度恢复核对原操作/组合/组和网关、租户、项目、主机身份；取消回执未终结时不宣称取消成功。外部产物尚未闭环：`project_profiles_execution.rs` 的 preview/apply 当前分别返回 503 `profile_observation_unavailable` / `profile_execution_unavailable`，progress/cancel 返回 404 `profile_operation_not_found`；创建/重拍尚缺领域实际状态读取。故当前无法证明一次 Desktop 组合应用真实生效，不能用 DTO、来源测试或最近组结果替代。

来源 `upstream/src-tauri/tests/profile_roundtrip.rs::claude_desktop_profile_scope_is_independent` 已阅读，未运行：它断言 Desktop 快照不填 Claude/Codex 槽、Desktop 供应商恢复及 Desktop current 标记。该来源场景未完整断言 Claude Code 文件前后不变、共享实体维护及不支持路径失败，恢复验证时需补齐目标实现证据。

| 待验行为 | 当前静态落点与边界 | 后续必须取得的证据 |
| --- | --- | --- |
| 三组快照与标记隔离 | `vw-api-types/src/project_profiles.rs` 的 `ProjectProfileScope` / `ProjectProfilePerApp` 明确 claude、claude-desktop、codex；`merge_scope_from` 只替换指定组。`profile_store.rs` 按租户存实体，选择按执行主机/组定位并核对项目占用 | Desktop 拍摄、切换、重拍及清标记前后，对比 Claude/Codex 快照、选择与 applied 记录；其他组和项目不受写入影响 |
| macOS 文件根隔离 | `providers/claude_desktop.rs::ClaudeDesktopFiles::open` 锁定 `$HOME/Library/Application Support/Claude/claude_desktop_config.json`、`Claude-3p/claude_desktop_config.json` 及 `Claude-3p/configLibrary/` 下固定 profile JSON 和 `_meta.json`；`providers/claude.rs` 使用独立 `.claude` 目录和 settings 文件 | 在隔离 HOME 中经真实领域服务完成 Desktop 切换，对比四个 Desktop 事务文件及 Claude Code 实际配置前像；反向切换同样核对，不只比较目录字符串 |
| 共享实体改名与删除 | `profile_store.rs::rename` 在同一 settings 更新事务内核对版本并更新共享实体；`delete` 同事务删除实体与所有引用它的 selections。API 维护入口走该所有者；桌面维护成功后重新读取 list/current | 三组读取同一 ID 的改名结果；删除后所有主机/组引用及 application 结果均清理，其他实体保留；旧版本冲突零写入 |
| Desktop 不支持项 | `apply_mcp.rs` 对非 null Desktop MCP 返回 Unsupported；`profile_skills.rs` 调用 `require_distribution`，Desktop 为 NoDirectorySync；`prompts/targets.rs::resolve` 明确拒绝 Desktop。组合预览 UI 当前无 Unsupported action，结果页仅输出状态与 reason_code，尚不能确认不支持项展示故事完成 | MCP、技能和提示词不支持时有明确可见结果，不借用 Claude 路径；null 类别不读写，非 null 不支持类别不能被空观察冒充成功；待执行端协议交付后核对预览、结果及 toast |
| 路径不可用不能伪报成功 | `snapshot_current` 要求读取失败返回 Denied/Unavailable/Unsupported 等错误，禁止空集兜底；生产执行 API 当前在缺接线时拒绝执行 | 不可读目录、非法/越界路径、符号链接及未授权执行主机均零越权写入、不产生 applied；失败保留原配置且错误可见 |

需先由现有执行端所有者完成真实观察、授权与领域应用接线，并冻结与任务 27 一致的操作回执；不在验收任务复制写文件逻辑或建立第二套存储。Desktop 不支持类别与仅供应商快照的兼容方式仍须在该接线中明确：当前 `snapshot_current` 顺序读取四类，不支持读取不能伪装为成功空集合。

后续新增并注册独立 `gateway/api/handlers/profile_macos_tests.rs` 的 `profile_t29_macos_desktop_scope_is_independent`，通过仓库共享槽位入口执行任务卡原定聚焦命令 `cargo test -j 1 -p vw-agent --lib profile_t29_ -- --test-threads=1`，记录有效测试数、退出码、实际槽位及两个绝对缓存路径。使用可恢复的隔离 HOME/项目、既有环境锁和执行主机环境路径；记录实际 macOS/CPU、Gateway/客户端版本及脱敏文件差异，不输出令牌。原生客户端读取、窗口及深浅主题渲染另交人工核验。

本轮未执行任何 Cargo 相关命令、构建、检查、单元测试、实机测试或最终回归；无测试通过数量、退出码或平台通过结论。未领取 Cargo 槽位，未设置 CARGO_TARGET_DIR / CARGO_BUILD_BUILD_DIR，无本任务槽位锁需清理。

## 任务 31：来源覆盖与 macOS 证据汇总（静态盘点完成，未验证）

2026-11-14：本轮只补必需交付合同，使用当前共享 checkout；未执行任何 Cargo 相关命令、构建、检查、单元测试、实机测试或最终回归。原卡的 Python 关键字检查也未运行。以下为文件读取及符号导航所得静态映射，不提供测试退出码、有效数量或平台通过结论；编号卡删除仅表示本轮盘点收尾。未领取槽位，未设置两个 Cargo 缓存变量。

### 11 个来源文件与实际所有者

来源冻结身份沿用 [领域合同](profile-contracts.md) 登记的 CC Switch 4.0.25 / `ca58c0313360eae7c851fbd7553c1614c7af03e7`；本轮未重新验证来源 Git 身份。下表与母清单 `来源清单.json`、`命令与事件覆盖.md` 及子计划 `来源映射.md` 交叉阅读，保留来源职责，不修改三份清单。来源路径相对同级 cc-switch；目标路径相对 crates。图索引未覆盖 project_profiles 新文件，因此这些文件直接读取，未重建索引。

| 来源文件 | 目标实现 / 证据段落 | 当前边界 |
| --- | --- | --- |
| `src-tauri/src/commands/profile.rs` | `vw-agent/src/gateway/api/handlers/project_profiles{,_maintenance,_execution}.rs`；本节六命令、两事件表 | API 已注册；有入口不等于可成功执行 |
| `src-tauri/src/database/dao/profiles.rs` | `vw-agent/src/gateway/agent_apps/profile_store.rs`；领域合同的存储与标记段 | 共享实体与租户/项目/主机/组标记使用现有 settings 所有者，不迁入来源数据库 |
| `src-tauri/src/services/profile.rs` | `vw-agent-apps/src/project_profiles/{snapshot,create,rename,resnapshot,preview,autosave,apply,apply_provider,apply_mcp,apply_skills,apply_prompt}.rs` | 编排与领域回调存在，生产观察/执行接线未完成 |
| `src-tauri/tests/profile_roundtrip.rs` | 本节八场景表及任务 28–30 段 | 来源测试不能充当目标 Gateway/SDK 或 macOS 证据 |
| `src/components/profiles/ProfileManageDialog.tsx` | `vw-desktop/src/app/components/project_profiles/management.rs`；任务 25 段 | trim、删除确认、显式重拍与维护错误处理；未验证 |
| `src/components/profiles/ProfileSwitcher.tsx` | `vw-desktop/src/app/components/project_profiles/{selection,apply}.rs`；任务 24、26 段 | 搜索、当前/未拍提示与预览；同当前组合跳过规则尚有差异 |
| `src/components/profiles/scope.ts` | `vw-api-types/src/project_profiles.rs::ProjectProfileScope` / `ProjectProfilePerApp`；任务 2 段 | 实际三独立组；不沿用 Switcher 过时的“两组”注释 |
| `src/config/piThinkingProfiles.ts` | 模型任务 13：`vw-agent-apps/src/models/{pi_thinking,preset_pi}.rs`、`models/presets/pi_thinking.json` | 模型推理档位，与项目组合 ID 无关；见本节 Pi 交接 |
| `src/lib/api/profiles.ts` | `vw-api-types/src/project_profiles.rs`、`vw-gateway-client/src/client/project_profile_api.rs`；任务 22 段 | 共享 DTO + 独立 SDK，不提供任意 Tauri command 调用 |
| `src/lib/query/profiles.ts` | `vw-desktop/src/app/message/project_profiles{,_apply,_progress}.rs`；本节刷新规则 | 维护后重读 list/current；四类刷新尚缺接线 |
| `tests/config/piThinkingProfiles.test.ts` | `vw-agent-apps/src/models/pi_thinking_tests.rs`；模型任务 13 历史证据 | 已有独立 8 场景源码；本轮未运行 |

### 六命令到 API / SDK / UI

HTTP 基址为 `/v1/agent-apps/project-profiles`，表中省略基址；SDK 位于 `GatewayClient`。注册经 handlers/agent_apps/mod.rs 汇入 router/bootstrap；本轮未发 HTTP 请求。所有行均未验证，静态可调用形状不能证明授权、存储或投影行为通过。

| 来源命令 | HTTP 与 SDK | 桌面 / 当前缺项 |
| --- | --- | --- |
| `list_profiles` | GET `/`、GET `/current` → `project_profile_list/current` | selection.rs 读取列表与当前组；损坏记录明确错误，不降为空列表 |
| `create_profile` | POST `/` → `project_profile_create` | management.rs 创建；四类读取回调为 Unavailable，当前返回 503 `snapshot_unavailable` |
| `update_profile` | PATCH `/{id}`、POST `/{id}/resnapshot` → `project_profile_rename/resnapshot` | 共享改名、指定组重拍携带 expectedVersion；重拍当前返回 503 `snapshot_unavailable` |
| `delete_profile` | DELETE `/{id}` → `project_profile_delete` | 删除确认；由存储事务删除实体和相关选择引用 |
| `clear_current_profile` | POST `/current/clear` → `project_profile_clear` | 清标记确认，不撤销原生配置 |
| `apply_profile` | POST `/{id}/preview`、POST `/{id}/apply` → `project_profile_preview/apply`；GET `/current/result` 与 `/operations/{operation_id}`，POST 对应 `/cancel` → `project_profile_result/progress/cancel` | 有效实体/版本后 preview/apply 仍返回 503 `profile_observation_unavailable` / `profile_execution_unavailable`；操作查询/取消返回 404 `profile_operation_not_found`，没有受理操作 |

### 两事件、TS 查询及组件规则

来源 `emit_profile_apply_events` 对 scope.apps() 发 `provider-switched`，载荷含 appType、proxyEnabled、autoFailoverEnabled、providerId；另发 `profile-applied`，载荷含 profileId、scope，并刷新托盘。目标不依赖 Tauri 事件字符串，需在执行端确认后提供等价行为；目前两项均未形成完整生产证据。

| 来源规则 | 当前映射与剩余工作 |
| --- | --- |
| `provider-switched` 仅通知目标组有效供应商 | apply_provider.rs 保留模式/路由合同；桌面 finish 目前仅触发 `Event::Refresh(scope)` 读取组合 list/current，未看到本路径刷新四类有效状态。执行端真实观察、有效路由及目标组供应商刷新待交付 |
| `profile-applied` 通知选择/应用结果 | project_profiles_progress.rs 按原操作与作用域核对终态，区分 Applied/Partial/Failed/Uncaptured/Cancelled；未接通服务端操作注册，不能把选择或最近组结果当本次成功 |
| TS profiles 查询及创建/更新/删除/清标记成功后失效刷新、失败 toast | project_profiles.rs 在服务端确认后重读 list/current，失败 toast；SDK 保留结构化 HTTP 错误，桌面当前多处折叠为通用失败，详细诊断展示仍待核对 |
| TS apply 后刷新 providers（三应用）、mcp/all、skills，并按 warnings/error/success 提示 | 目标要求按目标组刷新并补提示词有效状态；progress.rs 有按结果状态 toast，四类刷新尚未完成，不能据任务 26 的早期描述认定缓存已全部失效 |
| Switcher 搜索、仅三应用可见、未拍提示 | selection.rs 名称过滤与 scope_captured 提示，应用详情仅支持三组；未验证实际渲染 |
| 来源 `handleApply` 在 id == currentId 时不再次应用 | 目标 selection.rs 对当前项仍显示应用按钮，Open 分支仅检查忙碌/实体版本，未拒绝当前 ID。保留差异：由应用故事明确处理“重复点击”与“显式重新应用”的契约，不能声称已覆盖来源规则 |
| 名称 trim、空名称禁用、删除确认、警告可见 | management.rs 与领域改名/创建负责名称校验，删除有确认；来源 UI 隐去重拍而 command 保留，目标按母需求提供显式重拍。部分失败有逐项结果；Unsupported 预览 action 尚无对应值，待冻结生产预览协议后补齐 |
| 托盘事件与菜单更新 | 由母任务 20 承接，共用执行端结果和目标组刷新；本计划未交付托盘入口，不声明完成 |

### 上游八个 roundtrip 场景

以下完整名称来自 `src-tauri/tests/profile_roundtrip.rs`，均未运行。本表映射的是待取得的目标证据；领域闭包、来源测试和编译均不能替代真实配置投影。

| 来源测试 | 目标实现 / 验收归属 | 尚需证据 |
| --- | --- | --- |
| `profile_snapshot_apply_roundtrip_restores_configuration` | snapshot.rs、四类 apply_*.rs、apply.rs；任务 28 | 经 Gateway/SDK 创建 A/B 并恢复四类引用，核对实际文件及配置加载 |
| `shared_profile_sides_are_isolated_and_mergeable` | DTO merge_scope_from、resnapshot.rs、profile_store.rs；任务 30 | 三组快照合并、标记与原生文件互不串写 |
| `profile_apply_reports_dangling_references_and_continues` | 四类槽位 Missing 与 apply.rs Partial；任务 28/30 | 有效项继续且实际生效，缺失项保留原选择，不能记 applied |
| `clear_current_profile_only_clears_scoped_marker` | profile_store.rs 清标记、维护 API；任务 28/30 | 只清目标标记，配置/实体/其它组保持 |
| `switching_profile_autosaves_previous_profile_state` | autosave.rs::prepare_switch 与组重拍；任务 28/30 | 切走前仅保存旧组合当前组，版本冲突或读取失败不写入错误快照 |
| `applying_profile_without_active_prompt_disables_current_prompt` | apply_prompt.rs 与原 PromptsStore 停用事务；任务 28/30 | 空串停用、null 零读写、重复应用不重写、前像冲突拒绝 |
| `profile_switch_in_routing_mode_changes_the_route_only` | apply_provider.rs 与原供应商路由所有者；任务 28 | 保持代理模式、切有效路由、不改直连指针，退出代理恢复一致 |
| `claude_desktop_profile_scope_is_independent` | 三组 DTO、Desktop 原生供应商适配与 profile_store.rs；任务 29 | Desktop 四个事务文件与 Claude/Codex 前像对照、不支持项明确返回 |

### Pi 两文件交接与交叉来源

模型子计划原目录已不在当前文件树，不重建任务卡或重复实现。正式证据为 [模型实现证据「任务 13：Pi 推理档位绑定」](../model-catalog/implementation-evidence.md#任务-13pi-推理档位绑定)，登记提交 `9754a8983`。其历史记录为 2026-11-08、macOS 27.0 arm64，`catalog_pi_thinking_` 8 条、models 66 条以及固定来源投影对照；这些是历史文档记录，不是本轮新通过结果，也不证明项目组合 macOS 闭环。

- `src/config/piThinkingProfiles.ts` → pi_thinking.rs 的档位、显式目录键/协议绑定、null/缺失区分；preset_pi.rs 的 apply_thinking_profiles / thinking_level_map；JSON 映射数据由模型领域单独拥有。
- `tests/config/piThinkingProfiles.test.ts` → pi_thinking_tests.rs 的 rejects_incompatible_binding、keeps_local_overrides_at_their_deployment、distinguishes_null_from_an_absent_level、pairs_adaptive_compat_with_the_anthropic_binding、keeps_source_references_and_maps_out_of_omp、rejects_adaptive_compat_conflict、returns_independent_maps、requires_the_exact_catalog_key 八场景（共同前缀 `catalog_pi_thinking_`）。来源关于全部 reasoning 预设有 map 的断言在目标未逐字保留：模型合同明确无兼容绑定时省略，历史记录解释空映射投影差异；本轮不扩大该合同。
- 项目组合四类调用分别消费母任务 4 供应商事务、11 MCP 绑定、12 已安装技能分发、13 提示词事务；项目身份与偏好仍归母任务 15。Pi thinking 不加入四类快照，不借用 AgentProfile 身份，不复制配置写入所有者。托盘仍交母任务 20。

### 三组 macOS 汇总与恢复顺序

直接依赖任务 28、29、30 的编号卡已删除；正式证据在本文对应小节，提交分别为 `2090ff7f9`、`33309881a`、`0e3020a13`。三项均为静态盘点，未交付运行证据。三组为 Claude、Claude Desktop、Codex，Pi 是交叉来源而非第四验收组。

| 项目 | Claude（28） | Claude Desktop（29） | Codex（30） |
| --- | --- | --- | --- |
| 实际组合操作、四类配置恢复及原生客户端加载 | 未验证 | 未验证；MCP/技能/提示词不支持路径须明确 | 未验证 |
| 隔离 HOME/项目、租户/主机/组隔离及脱敏前后差异 | 未验证 | 未验证 | 未验证 |
| 取消、断线恢复、重复提交与失败不显示 applied | 未验证 | 未验证 | 未验证 |
| 桌面深浅主题、中文字形整形及弹窗实际渲染 | 未验证 | 未验证 | 未验证 |
| 平台/CPU、Gateway/SDK/客户端版本 | 未采集 | 未采集 | 未采集 |

下一步由现有执行端所有者先补授权执行主机上的四类真实观察、创建/重拍、预览/应用 worker、结果持久化和操作注册，并冻结 SDK/桌面回执；明确 Desktop 不支持类别如何拍摄，补目标组四类刷新及上述 UI 差异。解除本轮验证禁令后，再由任务 28–30 所列独立 macOS 场景经真实 Gateway/SDK 取得隔离文件证据，补权限、版本、取消和断线行为验证；最后交用户核对原生客户端加载、深浅主题和中文窗口。母任务 16 不可据本次清单标为全部完成。
