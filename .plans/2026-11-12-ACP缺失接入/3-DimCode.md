# 任务 3: 对接 DimCode

批次：【批次 3】 依赖批次 2

线性顺序：任务 3；本批只有此任务。所有批次串行，不存在可安全同时修改公共注册/分派文件的并行组。

状态：已阻塞

阻塞说明：注册、预设与原生验证通过；真实桌面验收需要解锁 Mac。

目的：让 DimCode 通过 ShiJu 现有 ACP 链路可安装或发现、启用并完成真实会话，准确暴露已验证能力。

来源任务：固定参考源码 `025_sync_and_add_acp_registry_agents.sql` 中 `dimcode`；完整来源路径见同目录 `差异清单.md`。

预计会话范围：只完成一个产品的纵向接入，复用现有协议客户端、目录和表单；不重建注册框架，不修复其他供应商。所需模型、权限与技能分支限于本接入真实支持的能力，不能复制其他供应商整套实现。

## 文件

- 修改：`crates/vw-acp/src/agent_registry.rs`，增加 `dimcode` 的 `BuiltInAgentDefinition`；稳定 key 不增加别名。
- 修改：`crates/vw-config-types/src/acp_install.rs`；测试：`crates/vw-config-types/src/acp_install_tests.rs`，新增 `acp_onboarding_dimcode_installer_contract`。
- 新建测试：`crates/vw-acp/tests/onboarding_dimcode.rs`，新增 `acp_onboarding_dimcode_registry_contract`、`acp_onboarding_dimcode_session_lifecycle`、`acp_onboarding_dimcode_permission_denial`。
- 测试：`crates/vw-agent/src/agent/catalog_tests.rs`，新增 `acp_onboarding_dimcode_catalog_binding`。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/model_source/drivers/dimcode.rs`、`dimcode_tests.rs`，仅在 CLI 支持会话模型绑定时实现和注册。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/skill_mounts/dimcode.rs`、`dimcode_tests.rs`，只在可隔离挂载时实现。
- 按实证必要修改：`crates/vw-acp/src/access_policy.rs`、`crates/vw-acp/src/client/access_policy.rs`；新增 `crates/vw-acp/src/access_policy/dimcode.rs`、`dimcode_tests.rs`，只在运行时原生权限契约得到证明时增加供应商分支。
- 桌面复核入口：`crates/vw-desktop/src/app/components/system_settings_agents/catalog.rs` 与 `crates/vw-desktop/src/app/state/settings/agents.rs`；默认无需修改，目录应由网关派生。
- 新建证据：`docs/dev/evidence/acp-onboarding-dimcode.md`，附必要的脱敏协议样本，不包含凭据、用户消息正文或本机私有配置。

## 上下文

- 从 `vw-acp::built_in_agent_specs` 派生官方 `acp-dimcode` 预设，沿用 Codex/Claude 既有安装、协议和安全门控。先读现有实现及独立测试，不在桌面绕过底层。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 来源契约与边界

- 来源启动基线：程序 `npx`，argv `["-y", "dimcode", "acp"]`；这是需验证的来源记录，不是已经通过本仓库验收的命令。
- 来源 npm 包 `dimcode`，历史核验版本 `0.3.30`。实施时核验官方发行信息并固定实际测试版本，不能把历史版本当当前推荐。
- 来源技能目录：来源未声明目录，先检查真实运行时；不能猜测目录。
- 本地二进制为 dim，npm 包名为 dimcode；验证安装后的真实 bin，不能从包名猜测命令。
- 对照 `crates/vw-acp/src/agent_registry.rs`、`src/client/protocol.rs`、`src/client/process.rs`、`src/access_policy/codex.rs`、`src/access_policy/claude.rs` 及对应测试；模型来源参考 `crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs` 和 `claude.rs`，技能参考相邻同名模块。
- 保留官方 CLI 本地配置路径；只有本会话选中的模型可通过现有中继绑定。不能复制参考项目令牌、默认模型、Electron 逻辑、品牌或数据库迁移。
- 未证明原生完全访问时返回 ClientOnly/Unavailable 等准确结果；宿主拒绝、只读、取消和版本变化均不能提升权限。上游模式名称不能直接视为授权。
- 新/恢复会话中的模型、MCP、技能、权限和租户/项目身份保持一致；能力不足明确报错，不能静默回退其他引擎、CLI 默认模型或其他会话。

## ACP 与内置智能体双重交付

- 必须同时交付 `dimcode` 的 ACP 运行链路和系统内置智能体 `acp-dimcode`，与现有 Codex、Claude Code 的内置预设一致；只添加命令/安装项不算完成。
- 内置目录通过 `crates/vw-agent/src/agent/catalog.rs` 的 `merged_agent_catalog` 派生，要求 `builtin=true`、`runtime=AgentRuntime::Acp { backend: "dimcode" }`，补齐显示名称、用途说明、推荐提示词和默认模型/权限/技能/MCP 继承。沿用现有默认启用规则及子代理限制，不自动扩大权限。
- 新增或修复必要的内置预设、配置持久化和会话选择连线：官方智能体目录可搜索，启用后出现在已启用目录及会话智能体选择器，可编辑保存默认项；重开设置/应用后保持配置；通过该预设新建会话确实启动 `dimcode`，不能落回 main、其他 ACP 或另一同名预设。
- 必读/按需修改：`crates/vw-agent/src/agent/catalog.rs`、`crates/vw-agent/src/config/schema/config_validate.rs`、`crates/vw-desktop/src/app/state/settings/agents.rs`、`crates/vw-desktop/src/app/message/settings/agents_profile.rs`；沿用已有配置和 UI 链路，不维护第二份手写目录。
- 新增失败测试：在 `crates/vw-agent/src/agent/catalog_tests.rs` 添加 `acp_onboarding_dimcode_builtin_preset_metadata`，覆盖内置标识、backend、名称/说明/推荐提示词及默认选择；在 `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs` 添加 `acp_onboarding_dimcode_builtin_profile_persists_and_runs`，通过生产配置与流入口验证保存/重读、启用/停用、新会话实际路由和身份固定。
- 失败/通过验证命令均为 `python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp_onboarding_dimcode -- --test-threads=1`；先确认新增用例因内置预设缺失或路由不完整而失败，完成后至少上述 2 个新测试实际通过，零测试无效。只替代外部 CLI/模型服务边界，不 mock 目录或持久化实现。
- 真实桌面验收必须从“官方智能体→启用→选择该内置智能体→新建会话”进入，保存并重开默认配置，检查实际后端与会话身份，完成两轮消息及现有能力验收；记录暗亮主题和证据。不得仅从 ACP 调试入口或手动指定 backend 绕过内置智能体流程。
- 完成门槛：ACP 适配与内置智能体任一缺失、配置保存失败或会话选择未打通，均不得标记 `已完成`；证据文档分别列出这两项交付结果。

## Cargo 共享槽位规则

1. 从仓库根目录直接使用已存在的 `python3 scripts/cargo_slot.py <命令及参数>`；入口自动领取或核验继承 `target/rust-slot-1` 至 `rust-slot-20`，不要在外层再手工领取第二个槽位。
2. 同一开发阶段将顺序原生检查放在一次入口调用中，持续复用缓存。例如 `python3 scripts/cargo_slot.py sh -c 'cargo check -p vw-desktop --lib && cargo clippy --all-targets --all-features -- -D warnings'`；实际 crate、features 与聚焦命令以本任务下文为准。下文裸 Cargo 表示入口内部命令，不表示允许绕过入口。
3. `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 由入口设置为同一槽位绝对路径，子脚本继承，不另建构建目录、不覆盖已领取路径、不执行 clean。任务涉及的二进制从该槽位定位。
4. 入口持有锁直到自有前台子进程组停止；常驻测试或应用存活期间不能提前释放。不启动脱离前台进程组的后台任务，不删除其它会话的锁、进程或缓存。正常、失败及中断退出后保留缓存。
5. 槽位只隔离缓存，不解除本任务的源码、任务依赖及验收资源互斥；20 个槽位不代表应并发运行 20 个重型构建。遵守现有批次与串行约定。
6. 完成时记录实际槽位、两个绝对目录、确切原生验证命令、退出码和有效测试数量。原生聚焦测试、整仓 `cargo clippy --all-targets --all-features -- -D warnings` 及要求的真实渲染、授权、音频与实机验收继续执行；零测试和热更新通过不能替代完成验收。

## 测试计划

- crate/features：`vw-acp`、`vw-config-types`、`vw-agent` 均使用其默认 features；仅规定的 clippy 使用 workspace `--all-targets --all-features`。已检查 workspace Cargo.toml 及这些 crate 的 Cargo.toml，不为此任务新增 feature。
- 行为：`dimcode` 的官方预设解析到正确安装/启动契约，在同一个 ACP 会话完成首轮、第二轮与取消，拒绝操作确实被拒绝。
- 失败测试：先创建上述独立测试文件，以公开 `built_in_agent_specs` / `resolve_agent_spec` 验证注册和 argv，并在配置测试、目录测试中验证安装映射与 `AgentRuntime::Acp` 绑定；测试名前缀统一 `acp_onboarding_dimcode`。
- 失败验证命令：`python3 scripts/cargo_slot.py cargo test -p vw-acp --test onboarding_dimcode -- --test-threads=1`；`python3 scripts/cargo_slot.py cargo test -p vw-config-types --lib acp_onboarding_dimcode -- --test-threads=1`；`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp_onboarding_dimcode -- --test-threads=1`。
- 预期失败：新增断言因 `dimcode` 尚未注册、没有安装映射或没有官方预设而失败；不是语法/依赖错误，也不能用零测试当失败验证。
- 通过验证命令：修复后重跑以上三个命令。每条命令有效测试至少 1 个，ACP 集成目标至少覆盖上述 3 个用例；模型/技能/权限新增测试也必须使用统一前缀并实际执行。
- 模拟策略：注册、目录、安装选择和 JSON-RPC 编解码使用生产代码；仅外部进程/网络通过临时可执行的 stdio ACP 对端替代，使用真实协议帧验证生命周期和拒绝回包，记录 initialize/new/prompt/cancel 形状。不得仅断言 mock 调用次数。真实 CLI 另行验证身份、实际模型请求与工具结果。

## 步骤

1. 阅读仓库 AGENTS、只读 plan.md、差异清单、本任务和相邻 Codex/Claude 实现；记录工作区现有变更，确认前一批任务已完成或已完成且仅存在不影响本接入的待回归项。源码/API 依赖仍未满足则先记录证据，不跳过依赖。
2. 从来源源码包只读提取对应迁移、运行时和能力定义；查该产品官方文档、源码与发行元数据，确认真实 CLI/适配器关系、版本、安装方式、支持平台及认证。必要时在网关隔离目录执行无凭据握手，记录来源与版本，禁止安装猜测的同名包。
3. 先添加并执行失败测试，确认缺失契约的预期失败；动态领取 Cargo 槽位，清理规则覆盖成功、失败和中断。
4. 在现有注册和安装白名单添加最小分支；本地 CLI 发现与备用启动参数均需验证。npm 包按已验收版本固定；二进制下载/安装只接受固定审核来源和平台映射，绝不能让客户端提交任意 URL、包名或 shell。上游未发行的平台返回准确不支持错误。
5. 由现有 `merged_agent_catalog` 派生 `acp-dimcode`，保持默认停用及默认不允许子代理；验证启用/停用、保存/重开及实际调用的后端身份。不要为此新增平行配置目录或手写桌面供应商列表。
6. 运行真实 CLI 的 initialize、session/new、两轮 prompt、cancel 以及它真实支持的 load/resume；测试拒绝文件/命令操作、错误认证、缺失可执行文件、错误模型和进程退出。客户端对端测试证明宿主逻辑，真实 CLI 测试证明供应商行为，两者证据不可互替。
7. 对照来源和实际握手逐项核验模型目录/切换、CLI 本地模型、会话模型绑定、权限模式、MCP stdio/http/sse、技能目录和斜杠命令。仅将真实支持项接入现有分派；上游明确不支持的功能给出稳定拒绝并写证据。不能以“全部不支持”代替调查已有上游能力，也不能为了显示可用而取消现有门控。
8. 对本产品支持的会话模型来源，使用受控本地上游响应服务和既有中继证明 provider/model 到达实际请求；认证只进入该子进程，日志/持久化不含真实密钥，CLI 原全局配置不被改写。对技能使用临时隔离目录，证明只发现选中的技能；对 MCP 验证仅所选服务进入会话，取消/恢复不扩大范围。
9. 运行 `cargo fmt --all` 后审查差异，仅保留本任务格式变更；执行聚焦通过验证及规定 clippy。实际桌面检查暗/亮主题的目录、状态、启用与会话选项；复用既有表单、4px 滚动条和 toast，中文不得使用等宽字体；变更字体或动态弹窗时确认实际整形/渲染路径无 panic。
10. 记录当前证据、平台限制和来源明确不支持项。最后审查之前所有已完成接入的注册仍保留，Codex/Claude 的关键行为未变。完成状态、提交及任务文件清理由本任务“完成”契约处理。

## 验证

- 聚焦：`python3 scripts/cargo_slot.py cargo test -p vw-acp --test onboarding_dimcode -- --test-threads=1`；`python3 scripts/cargo_slot.py cargo test -p vw-config-types --lib acp_onboarding_dimcode -- --test-threads=1`；`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib acp_onboarding_dimcode -- --test-threads=1`，记录每条退出码与有效测试数量，零测试无效。
- 周边注册保护：`python3 scripts/cargo_slot.py cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1`；`python3 scripts/cargo_slot.py cargo test -p vw-agent --lib agent::catalog_tests -- --test-threads=1`。
- 规定检查：`python3 scripts/cargo_slot.py cargo clippy --all-targets --all-features -- -D warnings`，必须执行，修复本次引入的错误/警告；既有无关失败记录文件位置与原始错误摘要，不借机扩展范围。
- 真实验收：在隔离网关配置和临时工作目录，通过实际官方目录启用并启动 `dimcode`，记录安装/发现、agentInfo 版本、两轮消息完成、工具写入临时文件、拒绝操作、取消后可继续、load/resume 支持或明确拒绝。不得仅以握手或进程退出码判定可用。
- 平台验收：为安装选择写 Unix/Windows 数据测试；不冒充已在其他 OS/架构运行。官方支持但未真实测的平台保留待回归记录。
- 模型、权限、MCP、技能：证据文档逐项标注支持/上游明确不支持/环境阻塞；能力支持结论必须对应当前版本和当前运行结果。无账号时不借用其他账号、不伪造完成。
- 所需证据：前后行为测试输出、确切命令、退出码、有效测试数、实际 Cargo 槽位及绝对路径 CARGO_TARGET_DIR、官方来源 URL/源码路径和版本、脱敏真实 CLI 回执、桌面观察结果与已审查差异。

## 完成

- 只在上述当前证据证明一个产品完整接入时标记 `已完成`；上游明确不支持的可选能力须已验证拒绝路径，不能把关键安装/会话失败算成产品完成。
- 外部依赖或缺少凭据导致核心会话无法验收：先调查合理替代路径，仍无解则仅在本文件标记 `已阻塞`，写具体原因、已尝试命令和所需输入；不得用产品登记成功替代真实接入。
- 只有无关既有编译错误等导致验证无法执行、且实现已完成时可标记 `已完成，待回归`，保留本任务文件与失败证据，不宣称已验收。
- 有实际代码/测试/配置/证据文档变更时使用 `$git-commit` 提交本任务变更，排除所有他人变更；仅任务状态或删除无需提交。最终报告完成证据，已完成则删除本任务文件；待回归/阻塞保留。不得修改 plan.md。


## 2026-11-13 执行记录

- 批次 2 依赖满足：提交 `264693d28` 及 `docs/dev/evidence/acp-onboarding-deepagents.md` 提供完整双重交付证据；未执行其他批次，未修改 plan.md。工作区原有桌面、cron、语音及计划改动均未纳入本任务。
- 当前已实现 `dimcode` 注册和固定 npm 安装 `dimcode@0.5.28`；本地候选为 `dim acp`，备用 `npx -y dimcode@0.5.28 acp`。`acp-dimcode` 由生产目录派生，无第二份桌面目录。补充独立 ACP 3 用例、Unix/Windows 安装契约及内置预设 metadata、binding、生产配置持久化与流路由用例。目录总数断言从 24 调整为 25，默认启用数仍为 6。代码保留在工作区，尚未提交，不代表任务已完成。
- 官方来源：`https://www.npmjs.com/package/dimcode`、`https://registry.npmjs.org/dimcode`、`https://www.dimcode.dev/en/docs/acp/`。官方文档当前访问会重定向到 dimagent.com 营销页面，因此功能结论以实际发行物协议为准。固定参考 tar 的 025 迁移已只读核对 `binary_name=dim` 和原 argv。
- `npm view dimcode version bin repository engines --json` 返回 0.5.28，bin 同时提供 dim/dimcode。`npm install --prefix /tmp/shiju-dimcode-probe --ignore-scripts --registry=https://registry.npmjs.org dimcode@0.5.28` 退出 0。原生 Darwin arm64 二进制 SHA256 为 `0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0`。仅安装隔离副本；HOME、模型凭据、工作目录均为临时夹具，未使用真实账号或全局配置。
- 真实 CLI 回执：agentInfo=dimcode/0.5.28，protocolVersion=1，loadSession=true；声明 image/embeddedContext、MCP http/sse，audio=false。无凭据 session/new 明确返回 Authentication required。只读、workspace-write、full-access 为上游选项名称，不据此发布 Native 权限承诺。
- 受控回环上游真实收到 `/v1/responses`、model=gpt-4o。两轮 prompt 均返回 end_turn，同进程 session/load 成功（`/tmp/shiju-dimcode-probe/two-turns.log`）。活动 prompt 取消返回 cancelled，随后同会话返回 end_turn（cancel.log）。workspace-write 的写入经真实 fs/write_text_file 回调；宿主拒绝后不产生文件（host-denial.log），宿主明确允许后临时文件内容为 fixture（write-allowed.log）。exec 经 session/request_permission，返回 cancelled 后未生成 exec-denied.txt（exec-denial.log）。早期夹具响应帧不完整曾得到 empty_model_response/incomplete_tool_call；修正 SSE 后通过，不将早期失败归因于供应商。
- 红灯：`/tmp/shiju-dimcode-probe/red.log` 中 ACP 2 通过/1 注册缺失失败；安装 0/1 缺映射失败；agent metadata/binding 0/2 缺预设失败。`red-profile.log` 中生产保存/路由测试 0/1 失败，断言 builtin/enabled 缺失。每条红灯命令均退出 101，非零测试，非语法错误。
- Cargo 槽位由 cargo_slot.py 自动领取：首轮红灯槽位 3，后续红灯和主验证槽位 2。两个变量 CARGO_TARGET_DIR、CARGO_BUILD_BUILD_DIR 分别同为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` 或 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。未手工占锁、未覆盖缓存目录。沿用批次 2 已证实的本包测试优化级别 0，使用 `cargo --config profile.test.package.vw-agent.opt-level=0 test ...`，features 保持默认；仅为降低大型 lib-test 构建成本，不改变源码或仓库 profile。
- 主阶段日志 `/tmp/shiju-dimcode-probe/green.log`：ACP 3、安装 1、agent 3 用例均通过且各退出 0。首次注册保护测试受本机已安装受管 Codex/OpenCode 影响出现 9 失败（含后继环境锁 poison），需要隔离配置复核。首次 catalog 9 通过/1 失败，定位另一处旧总数 24，已修正为 25，等待最终重跑。整仓 Clippy 正在执行，不提前宣称通过。
- 格式检查：仅 rustfmt 本任务六个 Rust 文件（skip_children=true），未格式化或重置他人文件；git diff --check 通过。
- 外部阻塞证据：cua.getState 返回 apps=[]、browsers=[]，错误为 “The Mac is locked and automatic unlock could not unlock it. Ask the user to unlock the Mac manually before continuing.” 已请求手动解锁。无法从官方目录完成启用、默认配置保存重开、选择预设后的两轮真实桌面会话与明暗主题渲染验收；不能用协议夹具替代。
- 未验收项保留：真实桌面双重交付、MCP 三类传输与所选服务隔离、技能来源隔离、跨进程恢复与其他 OS 实机。未新增 session_model/技能/Native 权限适配或宣称上游不支持。需继续调查真实支持，不能将待验证项写成“不支持”。任务不删除，不启动下一批。

- 补充验证：整仓 `cargo clippy --all-targets --all-features -- -D warnings` 已退出 0（6m19s，green.log）。同一槽位入口内执行过的命令为：
  1. `cargo test -p vw-acp --test onboarding_dimcode -- --test-threads=1`：3 通过，0。
  2. `cargo test -p vw-config-types --lib acp_onboarding_dimcode -- --test-threads=1`：1 通过，0。
  3. `cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_dimcode -- --test-threads=1`：3 通过，0。
  4. 注册保护与目录保护初次结果及原因见上文；最终在 `VIBEWINDOW_CONFIG_DIR=/tmp/shiju-dimcode-probe/isolated-gateway` 下以同样 cargo_slot.py 入口顺序复核，日志 final.log，实际槽位 2、两个缓存目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。其中 `cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1` 已 24 通过、退出 0；目录与再次聚焦测试尚在前台运行。
- 真实负例补充：本地服务 HTTP401 后 CLI 返回 refusal/provider_retry_exhausted；HTTP404 后明确返回 PROVIDER_MODEL_NOT_FOUND，所有实际模型请求仍为 gpt-4o，没有切换其它模型。日志 auth-failure.log、model-failure.log。该 404 是受控服务拒绝当前模型，并非声称 CLI 接受未知模型选项。上述“真实错误认证/错误模型负例”已得到这一传输边界证据，实际账号认证未使用。


### 最终验证结果与继续入口

- `final.log`：注册保护 24/24、目录保护 10/10、DimCode agent 聚焦 3/3，三条命令均退出 0；目录命令为 `cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1`。最后再次聚焦命令与上面第 3 条相同。结合 green.log 的 ACP 3/3、安装 1/1，规定聚焦 7 项全部通过。上文“正在执行/等待重跑”为历史进度，以本段为最终结果。
- 整仓 Clippy 已通过，之后仅修改编号任务记录；Rust 最终代码与已检查内容一致。`git diff --check` 通过。
- 最终 cargo_slot.py 前台入口正常退出 0，自有子进程结束、自有锁由入口释放，缓存保留。实际使用槽位为 3（早期红灯）、2（其余红灯和全部最终检查）；两环境变量均为对应绝对槽位目录，详见上文。未操作其它任务的进程或锁。
- 未将环境锁屏等同于无关编译失败，故保持 `已阻塞`，不标记“已完成，待回归”。不删除本文件，不提交未完成验收的任务，不推进批次 4。尚未生成最终证据文档；所有当前证据与后续缺口集中保留于本文件。
- 继续步骤：手动解锁 Mac 后，补齐真实官方目录启用→默认配置保存重开→会话选择→两轮消息及明暗主题观察，同时完成上文剩余 MCP/技能隔离与跨进程/平台边界调查；获得完整双重交付证据后再生成 docs/dev/evidence/acp-onboarding-dimcode.md，使用 git-commit 技能提交本任务文件集合并按完成契约清理任务文件。无需重复已取得的红灯证据。


### 阻塞恢复复查（2026-11-13 20:31，Asia/Shanghai）

- 按调度要求重读阻塞记录并复查 `cua.getState()`；原生应用仍返回锁屏且自动解锁失败的同一明确错误。未绕过锁屏、未修改系统权限。
- 安全替代路径核对：Chrome 扩展连接可用，但不能操作原生 Iced 窗口；浏览器或协议夹具不能替代官方目录、配置弹窗、会话选择器及明暗主题的真实桌面验收。未访问无关浏览器标签。
- 已核对 red.log、red-profile.log、green.log、final.log 及全部真实 CLI 探针日志仍存在。最终有效证据保持：聚焦 7 项通过、注册 24 项通过、目录 10 项通过、整仓 Clippy 退出 0；早期失败及最终修复结果均保留，未覆盖日志。此次未修改 Rust 代码，未重复 Cargo 检查或领取新槽位。
- 当前解除桌面阻塞所需的外部动作仅为用户手动解锁 Mac；解锁后仍须继续上文列出的其余验收，不能据此声称所有产品能力已完成。保持 `已阻塞`，不改为待回归，不提交、不删除任务文件。本轮按调度要求结束，不等待未到达的答复、不重复锁屏探针；用户解锁后在本会话继续。
