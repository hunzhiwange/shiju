# 任务 9: 对接 Amp

批次：【批次 9】 依赖批次 8

线性顺序：任务 9；本批只有此任务。所有批次串行，不存在可安全同时修改公共注册/分派文件的并行组。

状态：已完成，待回归验证（生产集成未验收）

2026-11-14 再次执行生产 preset 验收，仍因 `vw-gateway-client/src/client/agent_apps_skills/repos_api.rs:7` 对 `ProjectId` 调用 `to_string()` 的 E0599 无法编译；0 项测试执行、0 次模型请求。外层退出 1/Cargo 101，槽位 2 已释放，两个缓存路径均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。确切命令和日志见证据文档“再次回归”；未使用旧产物继续模型验收，保留待回归。

目的：让 Amp 通过 ShiJu 现有 ACP 链路可安装或发现、启用并完成真实会话，准确暴露已验证能力。

来源任务：固定参考源码 `025_sync_and_add_acp_registry_agents.sql` 中 `amp-acp`；完整来源路径见同目录 `差异清单.md`。

预计会话范围：只完成一个产品的纵向接入，复用现有协议客户端、目录和表单；不重建注册框架，不修复其他供应商。所需模型、权限与技能分支限于本接入真实支持的能力，不能复制其他供应商整套实现。

## 验证阶段

Rust 编译资源和会话调度按 `.plans/references/rust.md` 的项目覆盖规则执行；下方必要定向单元测试仍在任务内完成。

- 开发阶段完成本任务和直接受影响行为的必要定向单元测试，优先复用现有测试；确认有效测试数及结果，不积压到最终回归。
- 基础检查限定受影响包和目标；同一目标已被定向测试成功编译时不重复 check。Rust 不能只编译单个 `.rs` 文件，测试过滤也可能编译整个 lib-test 目标。
- 本任务不运行整仓构建、全目标全特性 Clippy、全仓格式检查或全量测试。统一回归只补跨任务集成及公共检查，不机械重跑未受后续改动影响的已通过单元测试。
- 仅同仓库 Rust 重型编译串行排队、复用缓存；其它语言的普通定向单元测试不套用此限制。必要定向测试通过后按原完成规则收尾；仅具体集成待验收或客观外部阻塞保留待回归。

## 文件

- 修改：`crates/vw-acp/src/agent_registry.rs`，增加 `amp-acp` 的 `BuiltInAgentDefinition`；稳定 key 不增加别名。
- 修改：`crates/vw-config-types/src/acp_install.rs`；测试：`crates/vw-config-types/src/acp_install_tests.rs`，新增 `acp_onboarding_amp_acp_installer_contract`。
- 新建测试：`crates/vw-acp/tests/onboarding_amp_acp.rs`，新增 `acp_onboarding_amp_acp_registry_contract`、`acp_onboarding_amp_acp_session_lifecycle`、`acp_onboarding_amp_acp_permission_denial`。
- 测试：`crates/vw-agent/src/agent/catalog_tests.rs`，新增 `acp_onboarding_amp_acp_catalog_binding`。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/model_source/drivers/amp_acp.rs`、`amp_acp_tests.rs`，仅在 CLI 支持会话模型绑定时实现和注册。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/skill_mounts/amp_acp.rs`、`amp_acp_tests.rs`，只在可隔离挂载时实现。
- 按实证必要修改：`crates/vw-acp/src/access_policy.rs`、`crates/vw-acp/src/client/access_policy.rs`；新增 `crates/vw-acp/src/access_policy/amp_acp.rs`、`amp_acp_tests.rs`，只在运行时原生权限契约得到证明时增加供应商分支。
- 桌面复核入口：`crates/vw-desktop/src/app/components/system_settings_agents/catalog.rs` 与 `crates/vw-desktop/src/app/state/settings/agents.rs`；默认无需修改，目录应由网关派生。
- 新建证据：`docs/dev/evidence/acp-onboarding-amp-acp.md`，附必要的脱敏协议样本，不包含凭据、用户消息正文或本机私有配置。

## 上下文

- 从 `vw-acp::built_in_agent_specs` 派生官方 `acp-amp-acp` 预设，沿用 Codex/Claude 既有安装、协议和安全门控。先读现有实现及独立测试，不在桌面绕过底层。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 来源契约与边界

- 来源启动基线：程序 `amp-acp`，argv `[]`；这是需验证的来源记录，不是已经通过本仓库验收的命令。
- 来源是网关宿主本地二进制；须从官方发行元数据确定固定来源、版本、平台/架构和校验；未确定前不可添加任意下载脚本。
- 来源技能目录：来源未声明目录，先检查真实运行时；不能猜测目录。
- 区分 Amp 主 CLI 与 amp-acp 桥接二进制；只允许审核后的固定下载来源、平台/架构和校验，缺少主程序单独报错。
- 对照 `crates/vw-acp/src/agent_registry.rs`、`src/client/protocol.rs`、`src/client/process.rs`、`src/access_policy/codex.rs`、`src/access_policy/claude.rs` 及对应测试；模型来源参考 `crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs` 和 `claude.rs`，技能参考相邻同名模块。
- 保留官方 CLI 本地配置路径；只有本会话选中的模型可通过现有中继绑定。不能复制参考项目令牌、默认模型、Electron 逻辑、品牌或数据库迁移。
- 未证明原生完全访问时返回 ClientOnly/Unavailable 等准确结果；宿主拒绝、只读、取消和版本变化均不能提升权限。上游模式名称不能直接视为授权。
- 新/恢复会话中的模型、MCP、技能、权限和租户/项目身份保持一致；能力不足明确报错，不能静默回退其他引擎、CLI 默认模型或其他会话。

## ACP 与内置智能体双重交付

- 必须同时交付 `amp-acp` 的 ACP 运行链路和系统内置智能体 `acp-amp-acp`，与现有 Codex、Claude Code 的内置预设一致；只添加命令/安装项不算完成。
- 内置目录通过 `crates/vw-agent/src/agent/catalog.rs` 的 `merged_agent_catalog` 派生，要求 `builtin=true`、`runtime=AgentRuntime::Acp { backend: "amp-acp" }`，补齐显示名称、用途说明、推荐提示词和默认模型/权限/技能/MCP 继承。沿用现有默认启用规则及子代理限制，不自动扩大权限。
- 新增或修复必要的内置预设、配置持久化和会话选择连线：官方智能体目录可搜索，启用后出现在已启用目录及会话智能体选择器，可编辑保存默认项；重开设置/应用后保持配置；通过该预设新建会话确实启动 `amp-acp`，不能落回 main、其他 ACP 或另一同名预设。
- 必读/按需修改：`crates/vw-agent/src/agent/catalog.rs`、`crates/vw-agent/src/config/schema/config_validate.rs`、`crates/vw-desktop/src/app/state/settings/agents.rs`、`crates/vw-desktop/src/app/message/settings/agents_profile.rs`；沿用已有配置和 UI 链路，不维护第二份手写目录。
- 新增失败测试：在 `crates/vw-agent/src/agent/catalog_tests.rs` 添加 `acp_onboarding_amp_acp_builtin_preset_metadata`，覆盖内置标识、backend、名称/说明/推荐提示词及默认选择；在 `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs` 添加 `acp_onboarding_amp_acp_builtin_profile_persists_and_runs`，通过生产配置与流入口验证保存/重读、启用/停用、新会话实际路由和身份固定。
- 失败/通过验证命令均为 `cargo test -j 1 -p vw-agent --lib acp_onboarding_amp_acp -- --test-threads=1`；先确认新增用例因内置预设缺失或路由不完整而失败，完成后至少上述 2 个新测试实际通过，零测试无效。只替代外部 CLI/模型服务边界，不 mock 目录或持久化实现。
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
- 行为：`amp-acp` 的官方预设解析到正确安装/启动契约，在同一个 ACP 会话完成首轮、第二轮与取消，拒绝操作确实被拒绝。
- 失败测试：先创建上述独立测试文件，以公开 `built_in_agent_specs` / `resolve_agent_spec` 验证注册和 argv，并在配置测试、目录测试中验证安装映射与 `AgentRuntime::Acp` 绑定；测试名前缀统一 `acp_onboarding_amp_acp`。
- 定向单元测试命令（任务内运行）：`cargo test -j 1 -p vw-acp --test onboarding_amp_acp -- --test-threads=1`；`cargo test -j 1 -p vw-config-types --lib acp_onboarding_amp_acp -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib acp_onboarding_amp_acp -- --test-threads=1`。
- 预期失败：新增断言因 `amp-acp` 尚未注册、没有安装映射或没有官方预设而失败；不是语法/依赖错误，也不能用零测试当失败验证。
- 通过验证命令（任务内运行）：修复后重跑以上三个命令。每条命令有效测试至少 1 个，ACP 集成目标至少覆盖上述 3 个用例；模型/技能/权限新增测试也必须使用统一前缀并实际执行。
- 模拟策略：注册、目录、安装选择和 JSON-RPC 编解码使用生产代码；仅外部进程/网络通过临时可执行的 stdio ACP 对端替代，使用真实协议帧验证生命周期和拒绝回包，记录 initialize/new/prompt/cancel 形状。不得仅断言 mock 调用次数。真实 CLI 另行验证身份、实际模型请求与工具结果。

## 步骤

1. 阅读仓库 AGENTS、只读 plan.md、差异清单、本任务和相邻 Codex/Claude 实现；记录工作区现有变更，确认前一批任务已完成或已完成且仅存在不影响本接入的待回归项。源码/API 依赖仍未满足则先记录证据，不跳过依赖。
2. 从来源源码包只读提取对应迁移、运行时和能力定义；查该产品官方文档、源码与发行元数据，确认真实 CLI/适配器关系、版本、安装方式、支持平台及认证。必要时在网关隔离目录执行无凭据握手，记录来源与版本，禁止安装猜测的同名包。
3. 先添加并执行失败测试，确认缺失契约的预期失败；动态领取 Cargo 槽位，清理规则覆盖成功、失败和中断。
4. 在现有注册和安装白名单添加最小分支；本地 CLI 发现与备用启动参数均需验证。npm 包按已验收版本固定；二进制下载/安装只接受固定审核来源和平台映射，绝不能让客户端提交任意 URL、包名或 shell。上游未发行的平台返回准确不支持错误。
5. 由现有 `merged_agent_catalog` 派生 `acp-amp-acp`，保持默认停用及默认不允许子代理；验证启用/停用、保存/重开及实际调用的后端身份。不要为此新增平行配置目录或手写桌面供应商列表。
6. 运行真实 CLI 的 initialize、session/new、两轮 prompt、cancel 以及它真实支持的 load/resume；测试拒绝文件/命令操作、错误认证、缺失可执行文件、错误模型和进程退出。客户端对端测试证明宿主逻辑，真实 CLI 测试证明供应商行为，两者证据不可互替。
7. 对照来源和实际握手逐项核验模型目录/切换、CLI 本地模型、会话模型绑定、权限模式、MCP stdio/http/sse、技能目录和斜杠命令。仅将真实支持项接入现有分派；上游明确不支持的功能给出稳定拒绝并写证据。不能以“全部不支持”代替调查已有上游能力，也不能为了显示可用而取消现有门控。
8. 对本产品支持的会话模型来源，使用受控本地上游响应服务和既有中继证明 provider/model 到达实际请求；认证只进入该子进程，日志/持久化不含真实密钥，CLI 原全局配置不被改写。对技能使用临时隔离目录，证明只发现选中的技能；对 MCP 验证仅所选服务进入会话，取消/恢复不扩大范围。
9. 仅对本次修改文件按需格式化 后审查差异，仅保留本任务格式变更；执行聚焦通过验证及规定 clippy。复用既有表单、4px 滚动条和 toast，中文不得使用等宽字体。
10. 记录当前证据、平台限制和来源明确不支持项。最后审查之前所有已完成接入的注册仍保留，Codex/Claude 的关键行为未变。完成状态、提交及任务文件清理由本任务“完成”契约处理。

## 定向验证（任务内完成）

- 聚焦：`cargo test -j 1 -p vw-acp --test onboarding_amp_acp -- --test-threads=1`；`cargo test -j 1 -p vw-config-types --lib acp_onboarding_amp_acp -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib acp_onboarding_amp_acp -- --test-threads=1`，记录每条退出码与有效测试数量，零测试无效。
- 周边注册保护：`cargo test -j 1 -p vw-acp --lib agent_registry_tests -- --test-threads=1`；`cargo test -j 1 -p vw-agent --lib agent::catalog_tests -- --test-threads=1`。
- 运行本任务必要定向单元测试，审查差异并记录有效测试数和结果；不重复公共整仓检查。
- 真实验收：在隔离网关配置和临时工作目录，通过生产目录与配置启用并启动 `amp-acp`，记录安装/发现、agentInfo 版本、两轮消息完成、工具写入临时文件、拒绝操作、取消后可继续、load/resume 支持或明确拒绝。不得仅以握手或进程退出码判定可用。
- 平台验收：为安装选择写 Unix/Windows 数据测试；不冒充已在其他 OS/架构运行。官方支持但未真实测的平台保留待回归记录。
- 模型、权限、MCP、技能：证据文档逐项标注支持/上游明确不支持/环境阻塞；能力支持结论必须对应当前版本和当前运行结果。无账号时不借用其他账号、不伪造完成。
- 所需证据：前后行为测试输出、确切命令、退出码、有效测试数、实际 Cargo 槽位及绝对路径 CARGO_TARGET_DIR、官方来源 URL/源码路径和版本、脱敏真实 CLI 回执与已审查差异。

## 完成

- 任务收尾：必要定向验证通过并审查后按原完成规则处理；只有明确集成待验收或客观外部阻塞时保留待回归，不能把必要单元测试统一延后。

- 只在上述当前证据证明一个产品完整接入时标记 `已完成`；上游明确不支持的可选能力须已验证拒绝路径，不能把关键安装/会话失败算成产品完成。
- 外部依赖或缺少凭据导致核心会话无法验收：先调查合理替代路径，仍无解则仅在本文件标记 `已阻塞`，写具体原因、已尝试命令和所需输入；不得用产品登记成功替代真实接入。
- 只有无关既有编译错误等导致验证无法执行、且实现已完成时可标记 `已完成，待回归`，保留本任务文件与失败证据，不宣称已验收。
- 有实际代码/测试/配置/证据文档变更时使用 `$git-commit` 提交本任务变更，排除所有他人变更；仅任务状态或删除无需提交。最终报告完成证据，已完成则删除本任务文件；待回归/阻塞保留。不得修改 plan.md。


## 2026-11-14 本次调查与阻塞证据

- 批次8依赖：`docs/dev/evidence/acp-onboarding-sigit.md`包含受审运行时、生产预设和最终Clippy通过证据；任务8文件已删除，sigit注册/安装已在源码。当前仅执行批次9，没有启动其他批次。
- 工作区预先存在Cargo.lock、桌面源码及多个计划文件的他人修改/删除；未修改或暂存它们。plan.md保持只读。
- 已读 `skills/do-task-plans/SKILL.md`、`skills/rustcodegraph/SKILL.md`、plan.md、差异清单和本任务。RustCodeGraph status成功，explore检查注册、安装与派生目录调用关系。
- 官方ACP目录：https://raw.githubusercontent.com/agentclientprotocol/registry/main/amp-acp/agent.json ，当前固定版0.12.0，指向 https://github.com/tao12345666333/amp-acp 。不能与`@edlsh/amp-acp`同名分叉混用。桥接器源码为TypeScript/Bun，不是Rust实现。
- 固定源码：https://github.com/tao12345666333/amp-acp/tree/v0.12.0 。README要求独立Amp CLI及`amp login`或AMP_API_KEY；二进制只打包桥接器，不含Amp主程序。固定发行来源 https://github.com/tao12345666333/amp-acp/releases/download/v0.12.0/amp-acp-darwin-aarch64.tar.gz 。下载后`shasum -a 256`退出0，结果`1c5f08f44f3711710769585855cefcdbaed53e4133c2263d59a368b5a1597151`与官方目录一致；仅解出普通文件amp-acp至临时目录，未全局安装。
- 官方平台元数据包含darwin aarch64/x86_64、linux aarch64/x86_64、windows x86_64；Windows arm64未列出。只有macOS arm64实际运行，其他平台未测。
- 无凭据检查仅记录布尔值：PATH中的amp不存在，`~/.local/bin/amp`、`~/.amp/bin/amp`和`~/.config/amp/secrets.json`均不存在，AMP_API_KEY未设置。没有读取或输出其它供应商密钥，没有借用账号。
- 临时调查目录`/tmp/shiju-amp-probe`，可重复脚本`python3 /tmp/shiju-amp-probe/probe.py`退出0（仅表示探针正常收尾，不表示会话成功）。环境显式限定PATH=/usr/bin:/bin，HOME/XDG_CONFIG_HOME/XDG_STATE_HOME/AMP_ACP_STATE_DIR均指向临时隔离目录，关闭plugin list。结束已terminate并wait自有桥接进程。
- 真实NDJSON回执：initialize(protocolVersion=1, clientCapabilities={})返回agentInfo.name=amp-acp/version=0.12.0；loadSession=true、MCP HTTP/SSE=true、resume/list/delete/close及thread-lifecycle/steering扩展已声明。authenticate返回code=-32000/message=Authentication required；session/new(cwd=临时目录,mcpServers=[])可建立S-会话；session/prompt返回-32603/Internal error，details=Cannot call write after a stream was destroyed。此环境无主CLI，不能将session/new或探针退出0当作真实首轮通过。
- 固定版src/amp-transport.ts:225–245实际启动`amp --execute --stream-json-thinking/--stream-json --stream-json-input --no-archive-after-execute`，仅bypass追加`--dangerously-allow-all`。src/server.ts:288–301明示default工具可不提示；未发现requestPermission宿主审批调用，readTextFile/writeTextFile转发方法的存在不能证明工具执行受宿主拒绝约束。故宿主拒绝、只读、取消后不扩大权限仍需实际CLI验证，不能直接开放内置预设。
- 能力调查：README声明MCP stdio/http/sse、模式和effort选项、插件模式发现、持久会话load/resume。模式选择不等于任意provider/model中继绑定；所选技能隔离和宿主审批尚未证实，均未宣称不支持或已支持。未以“全部不支持”跳过调查。
- 替代路径：已绕过全局安装直接校验发行物并隔离启动；核对同名分叉后仍选择官方目录指定实现。临时伪CLI/假模型可验证协议，却无法替代任务要求的真实Amp认证、工具拒绝和实际模型请求。没有可用Amp认证，因此安装主CLI也无法解决核心认证验收；未执行交互登录或生成虚假凭据。
- 本次未修改生产代码、测试、配置或另建证据文档；仅更新本任务阻塞记录，依完成契约无需提交。未运行Cargo、格式化或Clippy：未进入代码实现阶段，实际槽位/CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR均为“不适用，未领取”。有效Rust测试数0，未声称红灯或通过。下载早期完整tar提取遇到系统Python不支持filter参数，已改为只读取普通目标文件，未进行不安全解包。
- 未完成：TDD红绿验证、固定安装映射、ACP及acp-amp-acp双重实现、配置持久化/路由、真实两轮/取消/恢复、读写拒绝、模型/技能/MCP隔离和整仓Clippy。不能标记完成或删除任务文件。
- 解除阻塞所需：可用且获准使用的Amp CLI认证环境（`amp login`或进程环境注入AMP_API_KEY，无需在聊天提供密钥），随后先核验真实工具宿主边界；若原CLI不能强制宿主拒绝，需调查可证明的隔离/权限插件方案，再按本任务先红灯、后最小双重接入和完整验证执行。阻塞属于本任务核心运行依赖，不是无关编译失败，不能改为待回归。


### 继续调查：已解决主CLI安装，确认认证是独立阻塞

按继续执行要求，实际安装主CLI后重新调查，不沿用“缺CLI”作为停止理由。

- 官方安装脚本`https://ampcode.com/install.sh`已下载并逐段审阅，未执行其全局PATH/配置写入。脚本使用`https://static.ampcode.com/cli/cli-version.txt`解析固定版本，再下载版本目录的`amp-darwin-arm64.gz`与`darwin-arm64-amp.sha256`。
- 本次固定主CLI版本`0.0.1794614453-g43b36a`。实际下载URL：`https://static.ampcode.com/cli/0.0.1794614453-g43b36a/amp-darwin-arm64.gz`；SHA来源：`https://static.ampcode.com/cli/0.0.1794614453-g43b36a/darwin-arm64-amp.sha256`。解压后SHA256=`7f4aed3a2fb9d12e1e459909ee351dd2ab2a4965444db895dc412b5edf58e0d8`，Python assert与官方摘要匹配。仅安装至`/tmp/shiju-amp-probe/amp`，未写全局路径或shell配置。
- `env -i HOME=/tmp/shiju-amp-probe/isolated PATH=/usr/bin:/bin /tmp/shiju-amp-probe/amp --version`退出0，版本输出与固定版本一致；`--help`退出0。主CLI安装障碍已解决。
- 合法现有认证调查：实际运行主CLI `account list`，原HOME与隔离HOME均退出0且返回No saved accounts；原HOME仅报告是否无账号，不输出用户身份或凭据。环境AMP_API_KEY未设置。本次未启动浏览器登录、未借其他供应商认证。
- 隔离环境`amp config model-providers list`退出1：`Invalid or missing API key. Run 'amp login' to authenticate.`。`amp config model-providers add-router --help`退出0，确有custom-url、--base-url、--api-format及--api-key-file入口，但连接范围为personal/workspace账号，不能将provider key误当AMP_API_KEY。
- 官方文档`https://ampcode.com/docs/customize/model-routing`已核对：Custom URL属于账号中的模型连接；支持OpenAI/Anthropic等格式不意味着本地无账号启动。`https://ampcode.com/docs/cli`与实际`--help`明确AMP_URL为Amp service URL，非模型base URL。实际设置参考无直接OpenAI base URL或模型提供商密钥配置项，不能凭猜测新增环境变量映射。
- 重跑`python3 /tmp/shiju-amp-probe/probe-real-cli.py`（显式AMP_CLI_PATH=/tmp/shiju-amp-probe/amp）：initialize=0.12.0，authenticate仍返回Authentication required，new成功；prompt在12秒边界无成功终态，已结束自有进程。直接无凭据execute在25秒边界亦未完成，超时不计通过。
- 受控本地服务验证`python3 -u /tmp/shiju-amp-probe/local-service-probe.py`退出0：仅loopback监听，响应401，不伪造已认证服务。用合成测试字符串作为Custom URL的provider key、空Amp认证调用`config model-providers add-router custom-url --personal --base-url https://127.0.0.1:1/v1 --api-format chat-completions --api-key-file - --no-active`，CLI退出1，仍明确Invalid or missing API key；记录到POST `/api/internal`，没有模型请求。
- 同一受控服务下`--execute --stream-json`退出1，输出No API key found并尝试登录；GET `/api/auth/oauth-config`，未发送OpenAI/Anthropic推理请求。仅保留方法/路径与认证失败摘要，不保存生成的一次性登录URL。说明本地模型响应器无法替代Amp账号控制面；模拟整个已认证Amp服务或修改二进制跳过认证不能作为真实供应商验收。
- 受控脚本显式新进程组，结束后清理自有CLI；`ps -axo pid,ppid,command`按/tmp/shiju-amp-probe进程路径核对，无残留Amp/探针。没有Cargo进程、槽位锁或持久服务。
- 结论修正：当前阻塞不再是缺主CLI，而是无可用Amp账号认证。官方Custom URL路径也要求该认证；受控模型服务不能安全替代认证。桥接器源码可审阅/修补，但无法替代闭源Amp运行时的账号身份，也不能证明真实宿主工具拒绝。未发布未验收目录/预设，不将握手登记冒充双重交付。
- 下一步所需的唯一外部输入是用户在授权环境完成`amp login`，或向运行进程提供可用AMP_API_KEY（不要在聊天贴密钥）；认证后继续权限边界、受管适配、双重交付和TDD/Clippy。到当前仍只有编号任务文件状态/证据调整，无实际产品改动，依约不提交；任务保持已阻塞并保留，未领取Cargo槽位，两个构建目录不适用。


### 无账号自定义模型专项穿透调查（用户明确不采用账号）

用户已明确“不需要，自定义模型，希望穿透，不需要账号”。以此替代上文“下一步登录”的建议，不启动登录、不要求用户提供账号，原无账号目标不变。

1. 实际检查固定官方CLI内嵌字符串：无OPENAI_BASE_URL入口；ANTHROPIC_BASE_URL邻近Claude订阅actor凭据接口，不足以作为本地推理支持证据。AMP_URL明确为控制服务地址。没有从字符串存在推断功能已支持。
2. 新建仅临时目录探针`/tmp/shiju-amp-probe/offline-probe.py`，回环HTTP服务返回本地测试对象；合成local-fixture-only标识只发送到127.0.0.1，不发送官方服务、不冒充真实账号、不借用凭据。探针不保存消息/凭据，模型服务未连接外网。它用于分离“登录前检查”与“真正执行依赖”，不是供应商真实验收。
3. 请求响应格式实际确认为ok/result；给本地getUserInfo测试响应加入id/email/features后，CLI越过登录前检查，依次请求`/api/internal?getUserInfo`、loadPlugins、loadSkills，随后POST `/api/thread-actors`，body形状为executorType=local-client、agentMode=medium、features=[]和私有threadMeta。没有请求/v1/chat/completions或/v1/responses。
4. 固定主CLI0.0.1794614453-g43b36a随后明确报`ThreadClientConnectionError: Thread actors thread creation response missing threadId or wsToken`（退出1）。因此阻塞不只是“不给API key就不启动”：真实会话需要Amp thread actor控制协议及WebSocket服务，模型接口代理不能直接填补。
5. 为排除最新版专属变化，按适配器v0.12.0 README记载的已验证主CLI基线另行安装`0.0.1793433646-gfaad62`至`/tmp/shiju-amp-probe/amp-baseline`；固定官方来源https://static.ampcode.com/cli/0.0.1793433646-gfaad62/amp-darwin-arm64.gz ，解压SHA256=`34a56be6340c804b9c5148fbbee189e6701a62be57a38df1c7be603a88ab26ba`与同目录darwin-arm64-amp.sha256匹配。
6. `python3 -u /tmp/shiju-amp-probe/offline-baseline-probe.py`实际启动上述基线，CLI退出1，也进入getUserInfo/loadPlugins/loadSkills及POST /api/thread-actors（重试4次），无模型端点请求。最新版与桥接器文档已验证基线均无法通过只替换模型接口运行。
7. 官方安全架构文档https://ampcode.com/security 本次核对明确Amp Client把工具结果送回Amp Server进行下一步agentic loop，thread actors在服务端授权后获取请求级模型凭据，且不提供self-hosted部署。与本次请求路径一致，说明所缺的是代理服务端，不是OpenAI兼容请求格式。官方自定义模型说明https://ampcode.com/docs/customize/model-routing 属于服务端模型连接；没有找到本地完整代理服务端发行物。
8. 可修补的开源amp-acp是协议桥；替换其Amp执行器为ShiJu或另一个本地agent loop可以做出“同名可运行适配器”，但不再证明本任务要求的真实Amp行为，不能悄然交付为Amp。复刻完整thread actor服务亦没有当前原产品源码/契约证据，不能通过编造threadId/wsToken或伪造成功终态作为完成。
9. 当前准确阻塞更新为：在用户要求的无Amp账号约束下，已审固定CLI依赖未提供自托管实现的Amp Server/thread actor运行循环；单独模型中继无法满足。不是继续索取账号。若未来取得官方本地运行入口或可审完整服务端发行物，可按原任务恢复；若用户允许另建独立本地代理，则需明确命名和任务范围，不能当本任务Amp完成。
10. 两项专项探针退出0仅说明有界探针正常结束，真实CLI均退出1；无Rust生产变更，未运行Cargo或领取槽位，未产生可提交产品变更。任务继续已阻塞，保留编号文件；plan.md未改。


### 参考项目完整链路复核（用户指定new-shiju-agent）

只读检查`/Users/xiangmin/worker/code/new-shiju-agent`的当前前端、manifest列出的cli-models补丁及固定aioncore源码归档（不修改参考项目）。该项目无RustCodeGraph索引，使用rg/只读tarfile。

- 固定归档`crates/aionui-db/migrations/025_sync_and_add_acp_registry_agents.sql:47–50`登记Amp，backend=amp-acp、binary_name=amp-acp、command=amp-acp、args=[]、env=[]、native_skills_dirs=NULL、yolo_id=bypass。这是目录与通用ACP启动，不是自定义模型穿透。
- 同归档`033_team_capability_criteria.sql:229–232`为Amp预置认证方法setup，terminal-auth.command=amp-acp、args=[--setup]、名称Amp API Key Setup。证明其也保留Amp认证流程，未证明无账号运行。
- 实际生效的补丁路径`packages/desktop/shiju/cli-models/aioncore.patch:150–163`的supported函数明确只允许claude/codex/qwen/opencode，不含amp-acp；不支持后端返回shiju_cli_model_unsupported。该补丁约446行能力API也仅返回这四项。
- `packages/desktop/shiju/cli-models/README.md:3`明确限定上述四CLI；团队补丁也限定同一集合。模型选择前端`src/renderer/components/agent/AcpModelSelector/ShijuCliModelSelector.tsx:43`查询cliModelSources能力入口，不是所有ACP共用一个可穿透模型入口。
- 可学习的实现是：从后端能力列表驱动UI、会话固定模型来源、仅子进程覆盖环境/参数、独立状态目录、令牌留在loopback代理、无授权时明确拒绝。现仓库已有类似模型来源门控，应沿用；不能直接把amp-acp加到allowlist，因为它没有对应启动环境与协议分支。
- 结论限定：在当前参考仓库源码及manifest指定的归档/补丁中，没有找到Amp无账号自定义模型实现或真实完成证据；其Amp接入是目录+通用ACP+原生认证，并不能据此推翻已测thread actor依赖。仍保持已阻塞，不把目录存在当成模型请求验收。


### 补丁路线突破：历史官方CLI仍含真实本地代理循环

用户明确授权采用补丁方式继续。此前“只能等待Amp Server”的结论仅适用于已测原生二进制默认路径，现已找到并实测不同官方发行路径，应以本节新证据修正，不再以账号作为当前阻塞。

- 官方npm元数据https://registry.npmjs.org/@sourcegraph%2famp ，最后一个bin=dist/main.js的可读JS发行版`0.0.1778732631-gc5875c`（Node>=20，2026年5月历史版本，不能冒充最新版）。固定包https://registry.npmjs.org/@sourcegraph/amp/-/amp-0.0.1778732631-gc5875c.tgz 。下载后SHA512 SRI匹配`sha512-+DY8uzhwq7i9BlSwfTwIDaPX6UdsTocOjdP2unf3TOEkO8WV6vP5I/xYklJHCrcyX7wMUhwwLJCCVH+wX4AvnQ==`；原始dist/main.js SHA256=`458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836`。
- 官方包内保留ThreadWorker、真实工具编排和模型客户端；默认Neo/thread actor路径之外仍有`--take-me-back`分支，不需要重新实现另一个agent loop。实际支持--stream-json-input、--stream-json-thinking，可继续验证与amp-acp0.12.0的真实兼容性。
- 已用`/tmp/shiju-amp-probe/offline-js-probe.py`运行官方JS本地分支：隔离HOME、只连接loopback控制服务，返回本地身份和空远程插件、getThread未找到；真实CLI到达/api/provider/anthropic/v1/messages，受控Anthropic SSE响应被真实ThreadWorker解析，assistant返回OK且最终result.subtype=success/is_error=false。CLI退出0，不是仅握手；这是确定性模型边界测试，不冒充真实云模型响应。
- 实际补丁探针`/tmp/shiju-amp-probe/apply-local.py`对固定JS执行三处唯一匹配替换，把默认启动/依赖/thread create路由到原生本地分支；工具invokeTool入口临时明确抛错，避免尚未实现宿主边界时运行原生工具。原CLI文件不变，新文件shiju-local-prototype.js SHA256=`b46302c7c4618f047ed60e806f577a2bf5b0469705347f7345ee340da7966a9d`。
- `python3 -u /tmp/shiju-amp-probe/patched-probe.py`不传--take-me-back，实际运行补丁CLI，退出0；真实原生回合仍返回OK和success，无/api/thread-actors请求。所有控制/模型请求仅127.0.0.1，使用本地合成身份及本地测试标识；不发送官方服务、不伪造远程账号、不消耗Amp额度。
- 这些只是隔离原型，未引入仓库生产路径或登记为官方预设；不能把单轮成功当完整交付。下一实施边界：固定双包受管产物校验、生产loopback控制适配（本地会话存储、selected provider/model中继）、在原生工具执行前转宿主并拒绝本地回退、关闭环境技能/MCP/插件发现；先红灯注册/安装/预设测试，再完成真实ACP双轮/取消/恢复/拒绝及配置路由和Clippy。
- 尚未实现宿主工具边界、固定受管安装、生产模型driver及双预设链路，因此任务进行中而非已完成。仍无仓库Rust产品代码改动，Cargo槽位与两个构建目录均未领取；仅编号文件记录更新无需提交。原型工具硬拒绝是临时保护，不作为最终“支持全部工具”结论。


### 当前实现与待回归（2026-11-14）

已实现固定双包受管安装/校验、空参数启动、私有模型桥、真实本地 ThreadWorker、宿主工具与取消/恢复边界，并派生内置 `acp-amp-acp`。能力仅声明受审 session_model，未接通的技能/MCP/原生权限扩展明确拒绝。实现细节和最新结果见 `docs/dev/evidence/acp-onboarding-amp-acp.md`。

最新槽位入口为 `python3 scripts/cargo_slot.py python3 /tmp/shiju-amp-probe/verify-proof.py`；本轮槽位2，两个缓存变量同为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。旧任务手工槽位步骤已由仓库入口等价替代，未改 plan.md/prompt.md。原生测试保持 Cargo；仅 vw-agent 测试包使用 `--config profile.test.package.vw-agent.opt-level=0` 降低编译成本，不作性能结论。

ACP 4项、安装1项、真实工具8组（允许/拒绝×读写编辑命令，均含4回合、重启load/history、取消继续、认证和模型错误）通过。生产预设与 catalog 当前被其他并发改动的 `ProjectId: Display`/技能 API 依赖编译错误阻断；旧测试程序运行失败，不计作当前集成证据。按任务约定保留本文件为待回归，不删除、不声称双重交付完成。

下一步：无关编译恢复后，使用当前 contracts 重新编译并执行 preset（含 ignored）与 session-model 真正生产中继验收、catalog 保护及全仓 Clippy；全部满足后方可删除本任务文件。

最终注册保护25项通过；受影响包及全仓 Clippy 均因未修改的 content-guard/memory-algorithms/domain-matcher lint 退出101。完整当前记录在证据文档。待回归状态保留。
