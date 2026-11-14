# 任务 16: 对接 Stakpak

批次：【批次 16】 依赖批次 15

线性顺序：任务 16；本批只有此任务。所有批次串行，不存在可安全同时修改公共注册/分派文件的并行组。

状态：未开始

目的：让 Stakpak 通过 ShiJu 现有 ACP 链路可安装或发现、启用并完成真实会话，准确暴露已验证能力。

来源任务：固定参考源码 `025_sync_and_add_acp_registry_agents.sql` 中 `stakpak`；完整来源路径见同目录 `差异清单.md`。

预计会话范围：只完成一个产品的纵向接入，复用现有协议客户端、目录和表单；不重建注册框架，不修复其他供应商。所需模型、权限与技能分支限于本接入真实支持的能力，不能复制其他供应商整套实现。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件

- 修改：`crates/vw-acp/src/agent_registry.rs`，增加 `stakpak` 的 `BuiltInAgentDefinition`；稳定 key 不增加别名。
- 修改：`crates/vw-config-types/src/acp_install.rs`；测试：`crates/vw-config-types/src/acp_install_tests.rs`，新增 `acp_onboarding_stakpak_installer_contract`。
- 新建测试：`crates/vw-acp/tests/onboarding_stakpak.rs`，新增 `acp_onboarding_stakpak_registry_contract`、`acp_onboarding_stakpak_session_lifecycle`、`acp_onboarding_stakpak_permission_denial`。
- 测试：`crates/vw-agent/src/agent/catalog_tests.rs`，新增 `acp_onboarding_stakpak_catalog_binding`。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/model_source/drivers/stakpak.rs`、`stakpak_tests.rs`，仅在 CLI 支持会话模型绑定时实现和注册。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/skill_mounts/stakpak.rs`、`stakpak_tests.rs`，只在可隔离挂载时实现。
- 按实证必要修改：`crates/vw-acp/src/access_policy.rs`、`crates/vw-acp/src/client/access_policy.rs`；新增 `crates/vw-acp/src/access_policy/stakpak.rs`、`stakpak_tests.rs`，只在运行时原生权限契约得到证明时增加供应商分支。
- 桌面复核入口：`crates/vw-desktop/src/app/components/system_settings_agents/catalog.rs` 与 `crates/vw-desktop/src/app/state/settings/agents.rs`；默认无需修改，目录应由网关派生。
- 新建证据：`docs/dev/evidence/acp-onboarding-stakpak.md`，附必要的脱敏协议样本，不包含凭据、用户消息正文或本机私有配置。

## 上下文

- 从 `vw-acp::built_in_agent_specs` 派生官方 `acp-stakpak` 预设，沿用 Codex/Claude 既有安装、协议和安全门控。先读现有实现及独立测试，不在桌面绕过底层。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 来源契约与边界

- 来源启动基线：程序 `stakpak`，argv `["acp"]`；这是需验证的来源记录，不是已经通过本仓库验收的命令。
- 来源是网关宿主本地二进制；须从官方发行元数据确定固定来源、版本、平台/架构和校验；未确定前不可添加任意下载脚本。
- 来源技能目录：`.stakpak/skills`。
- 区分 CLI 本地配置与会话模型来源，不能覆盖用户全局配置或静默借用其他供应商凭据。
- 对照 `crates/vw-acp/src/agent_registry.rs`、`src/client/protocol.rs`、`src/client/process.rs`、`src/access_policy/codex.rs`、`src/access_policy/claude.rs` 及对应测试；模型来源参考 `crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs` 和 `claude.rs`，技能参考相邻同名模块。
- 保留官方 CLI 本地配置路径；只有本会话选中的模型可通过现有中继绑定。不能复制参考项目令牌、默认模型、Electron 逻辑、品牌或数据库迁移。
- 未证明原生完全访问时返回 ClientOnly/Unavailable 等准确结果；宿主拒绝、只读、取消和版本变化均不能提升权限。上游模式名称不能直接视为授权。
- 新/恢复会话中的模型、MCP、技能、权限和租户/项目身份保持一致；能力不足明确报错，不能静默回退其他引擎、CLI 默认模型或其他会话。

## ACP 与内置智能体双重交付

- 必须同时交付 `stakpak` 的 ACP 运行链路和系统内置智能体 `acp-stakpak`，与现有 Codex、Claude Code 的内置预设一致；只添加命令/安装项不算完成。
- 内置目录通过 `crates/vw-agent/src/agent/catalog.rs` 的 `merged_agent_catalog` 派生，要求 `builtin=true`、`runtime=AgentRuntime::Acp { backend: "stakpak" }`，补齐显示名称、用途说明、推荐提示词和默认模型/权限/技能/MCP 继承。沿用现有默认启用规则及子代理限制，不自动扩大权限。
- 新增或修复必要的内置预设、配置持久化和会话选择连线：官方智能体目录可搜索，启用后出现在已启用目录及会话智能体选择器，可编辑保存默认项；重开设置/应用后保持配置；通过该预设新建会话确实启动 `stakpak`，不能落回 main、其他 ACP 或另一同名预设。
- 必读/按需修改：`crates/vw-agent/src/agent/catalog.rs`、`crates/vw-agent/src/config/schema/config_validate.rs`、`crates/vw-desktop/src/app/state/settings/agents.rs`、`crates/vw-desktop/src/app/message/settings/agents_profile.rs`；沿用已有配置和 UI 链路，不维护第二份手写目录。
- 新增失败测试：在 `crates/vw-agent/src/agent/catalog_tests.rs` 添加 `acp_onboarding_stakpak_builtin_preset_metadata`，覆盖内置标识、backend、名称/说明/推荐提示词及默认选择；在 `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs` 添加 `acp_onboarding_stakpak_builtin_profile_persists_and_runs`，通过生产配置与流入口验证保存/重读、启用/停用、新会话实际路由和身份固定。
- 失败/通过验证命令均为 `cargo test -j 1 -p vw-agent --lib acp_onboarding_stakpak -- --test-threads=1`；先确认新增用例因内置预设缺失或路由不完整而失败，完成后至少上述 2 个新测试实际通过，零测试无效。只替代外部 CLI/模型服务边界，不 mock 目录或持久化实现。
- 完成门槛：ACP 适配与内置智能体任一缺失、配置保存失败或会话选择未打通，均不得标记 `已完成`；证据文档分别列出这两项交付结果。

## Cargo 共享槽位规则（最多 10 个）

本任务保留动态领取、继承复用、双缓存路径、所有权核验及退出清理能力；直接运行原生 Cargo，共享池最多 10 个槽位。

1. 同一仓库所有计划和任务共用 `target/rust-slot-1` 至 `target/rust-slot-10`；槽位编号仅为 1–10，不新增第 11 个槽位、不按任务编号固定分配、不创建任务专属缓存。
2. 从仓库根目录以原子 `mkdir target/rust-slot-locks/slot-N.lock` 领取空闲槽位；仅创建成功者持有。占用时尝试下一个，全部占用时等待，不删除其它活跃任务的锁。
3. 锁中记录真实持有进程 PID 以及任务/会话标识；持有进程须覆盖整个构建、验证及产物使用阶段，不能记录即将退出的临时 shell PID。遗留锁仅在核实持有进程及使用者都已退出后清理；归属或存活状态不明则保持占用。
4. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 都设置为同一槽位的绝对路径；原生 Cargo 和所有子进程继承，不通过配置、别名或 `--target-dir` 覆盖。二进制等产物也从该槽位定位。
5. 同阶段顺序命令和嵌套调用复用已持有槽位；继承前核验锁的持有 PID 属于当前进程或仍存活的祖先进程，路径与锁对应，不重复领取，不借用其它任务的槽位。无需包装脚本或额外测试运行器。
6. 正常、失败与取消时先结束自有 Cargo、rustc、链接器及仍使用槽位产物的子进程，确认退出后才删除自有占用记录、释放自有槽位锁；保留缓存。不删除其它任务的锁、进程或缓存。
7. 记录实际槽位、两个绝对缓存路径、确切验证命令、退出码和有效测试数；编译成功或零测试不能替代行为验收。
8. 10 是缓存容量，不是编译并发数。会话并发按 `.plans/references/rust.md` 的项目覆盖规则执行，槽位隔离不解除源码、依赖及共享资源约束；同仓库高内存编译仍串行排队，不利用空闲槽位绕过内存限制。

取消或超时后确认自有 Cargo/rustc、链接器及产物使用者退出，再释放槽位或重试；不能将已发信号视为已退出。顺序验证前项成功才继续，禁止失败后无条件启动后续重型检查。

## 测试计划

- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 行为：`stakpak` 的官方预设解析到正确安装/启动契约，在同一个 ACP 会话完成首轮、第二轮与取消，拒绝操作确实被拒绝。
- 失败测试：先创建上述独立测试文件，以公开 `built_in_agent_specs` / `resolve_agent_spec` 验证注册和 argv，并在配置测试、目录测试中验证安装映射与 `AgentRuntime::Acp` 绑定；测试名前缀统一 `acp_onboarding_stakpak`。
- 定向单元测试命令（任务内运行）：`cargo test -j 1 -p vw-acp --test onboarding_stakpak -- --test-threads=1`；`cargo test -j 1 -p vw-config-types --lib acp_onboarding_stakpak -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib acp_onboarding_stakpak -- --test-threads=1`。
- 预期失败：新增断言因 `stakpak` 尚未注册、没有安装映射或没有官方预设而失败；不是语法/依赖错误，也不能用零测试当失败验证。
- 通过验证命令（任务内运行）：修复后重跑以上三个命令。每条命令有效测试至少 1 个，ACP 集成目标至少覆盖上述 3 个用例；模型/技能/权限新增测试也必须使用统一前缀并实际执行。
- 模拟策略：注册、目录、安装选择和 JSON-RPC 编解码使用生产代码；仅外部进程/网络通过临时可执行的 stdio ACP 对端替代，使用真实协议帧验证生命周期和拒绝回包，记录 initialize/new/prompt/cancel 形状。不得仅断言 mock 调用次数。真实 CLI 另行验证身份、实际模型请求与工具结果。

## 步骤

1. 阅读仓库 AGENTS、只读 plan.md、差异清单、本任务和相邻 Codex/Claude 实现；记录工作区现有变更，确认前一批任务已完成或已完成且仅存在不影响本接入的待回归项。源码/API 依赖仍未满足则先记录证据，不跳过依赖。
2. 从来源源码包只读提取对应迁移、运行时和能力定义；查该产品官方文档、源码与发行元数据，确认真实 CLI/适配器关系、版本、安装方式、支持平台及认证。必要时在网关隔离目录执行无凭据握手，记录来源与版本，禁止安装猜测的同名包。
3. 先添加并执行失败测试，确认缺失契约的预期失败；动态领取 Cargo 槽位，清理规则覆盖成功、失败和中断。
4. 在现有注册和安装白名单添加最小分支；本地 CLI 发现与备用启动参数均需验证。npm 包按已验收版本固定；二进制下载/安装只接受固定审核来源和平台映射，绝不能让客户端提交任意 URL、包名或 shell。上游未发行的平台返回准确不支持错误。
5. 由现有 `merged_agent_catalog` 派生 `acp-stakpak`，保持默认停用及默认不允许子代理；验证启用/停用、保存/重开及实际调用的后端身份。不要为此新增平行配置目录或手写桌面供应商列表。
6. 运行真实 CLI 的 initialize、session/new、两轮 prompt、cancel 以及它真实支持的 load/resume；测试拒绝文件/命令操作、错误认证、缺失可执行文件、错误模型和进程退出。客户端对端测试证明宿主逻辑，真实 CLI 测试证明供应商行为，两者证据不可互替。
7. 对照来源和实际握手逐项核验模型目录/切换、CLI 本地模型、会话模型绑定、权限模式、MCP stdio/http/sse、技能目录和斜杠命令。仅将真实支持项接入现有分派；上游明确不支持的功能给出稳定拒绝并写证据。不能以“全部不支持”代替调查已有上游能力，也不能为了显示可用而取消现有门控。
8. 对本产品支持的会话模型来源，使用受控本地上游响应服务和既有中继证明 provider/model 到达实际请求；认证只进入该子进程，日志/持久化不含真实密钥，CLI 原全局配置不被改写。对技能使用临时隔离目录，证明只发现选中的技能；对 MCP 验证仅所选服务进入会话，取消/恢复不扩大范围。
9. 仅对本次修改文件按需格式化 后审查差异，仅保留本任务格式变更；执行聚焦通过验证及规定 clippy。复用既有表单、4px 滚动条和 toast，中文不得使用等宽字体。
10. 记录当前证据、平台限制和来源明确不支持项。最后审查之前所有已完成接入的注册仍保留，Codex/Claude 的关键行为未变。完成状态、提交及任务文件清理由本任务“完成”契约处理。

## 定向验证（任务内完成）

- 聚焦：`cargo test -j 1 -p vw-acp --test onboarding_stakpak -- --test-threads=1`；`cargo test -j 1 -p vw-config-types --lib acp_onboarding_stakpak -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib acp_onboarding_stakpak -- --test-threads=1`，记录每条退出码与有效测试数量，零测试无效。
- 周边注册保护：`cargo test -j 1 -p vw-acp --lib agent_registry_tests -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib agent::catalog_tests -- --test-threads=1`。
- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 真实验收：在隔离网关配置和临时工作目录，通过生产目录与配置启用并启动 `stakpak`，记录安装/发现、agentInfo 版本、两轮消息完成、工具写入临时文件、拒绝操作、取消后可继续、load/resume 支持或明确拒绝。不得仅以握手或进程退出码判定可用。
- 平台验收：为安装选择写 Unix/Windows 数据测试；不冒充已在其他 OS/架构运行。官方支持但未真实测的平台保留待回归记录。
- 模型、权限、MCP、技能：证据文档逐项标注支持/上游明确不支持/环境阻塞；能力支持结论必须对应当前版本和当前运行结果。无账号时不借用其他账号、不伪造完成。
- 所需证据：前后行为测试输出、确切命令、退出码、有效测试数、实际 Cargo 槽位及绝对路径 CARGO_TARGET_DIR、官方来源 URL/源码路径和版本、脱敏真实 CLI 回执与已审查差异。

## 完成

- 任务收尾：必要定向验证通过并审查后按原完成规则处理；只有明确集成待验收或客观外部阻塞时保留待回归，不能把必要单元测试统一延后。

- 只在上述当前证据证明一个产品完整接入时标记 `已完成`；上游明确不支持的可选能力须已验证拒绝路径，不能把关键安装/会话失败算成产品完成。
- 外部依赖或缺少凭据导致核心会话无法验收：先调查合理替代路径，仍无解则仅在本文件标记 `已阻塞`，写具体原因、已尝试命令和所需输入；不得用产品登记成功替代真实接入。
- 只有无关既有编译错误等导致验证无法执行、且实现已完成时可标记 `已完成，待回归`，保留本任务文件与失败证据，不宣称已验收。
- 有实际代码/测试/配置/证据文档变更时使用 `$git-commit` 提交本任务变更，排除所有他人变更；仅任务状态或删除无需提交。最终报告完成证据，已完成则删除本任务文件；待回归/阻塞保留。不得修改 plan.md。
