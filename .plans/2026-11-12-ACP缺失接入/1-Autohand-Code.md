# 任务 1: 对接 Autohand Code

批次：【批次 1】 无

线性顺序：任务 1；本批只有此任务。所有批次串行，不存在可安全同时修改公共注册/分派文件的并行组。

状态：进行中

目的：让 Autohand Code 通过 ShiJu 现有 ACP 链路可安装或发现、启用并完成真实会话，准确暴露已验证能力。

来源任务：固定参考源码 `025_sync_and_add_acp_registry_agents.sql` 中 `autohand`；完整来源路径见同目录 `差异清单.md`。

预计会话范围：只完成一个产品的纵向接入，复用现有协议客户端、目录和表单；不重建注册框架，不修复其他供应商。所需模型、权限与技能分支限于本接入真实支持的能力，不能复制其他供应商整套实现。

## 文件

- 修改：`crates/vw-acp/src/agent_registry.rs`，增加 `autohand` 的 `BuiltInAgentDefinition`；稳定 key 不增加别名。
- 修改：`crates/vw-config-types/src/acp_install.rs`；测试：`crates/vw-config-types/src/acp_install_tests.rs`，新增 `acp_onboarding_autohand_installer_contract`。
- 新建测试：`crates/vw-acp/tests/onboarding_autohand.rs`，新增 `acp_onboarding_autohand_registry_contract`、`acp_onboarding_autohand_session_lifecycle`、`acp_onboarding_autohand_permission_denial`。
- 测试：`crates/vw-agent/src/agent/catalog_tests.rs`，新增 `acp_onboarding_autohand_catalog_binding`。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/model_source/drivers/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/model_source/drivers/autohand.rs`、`autohand_tests.rs`，仅在 CLI 支持会话模型绑定时实现和注册。
- 按实证必要修改：`crates/vw-agent/src/session/llm/acp/skill_mounts/mod.rs`；新增 `crates/vw-agent/src/session/llm/acp/skill_mounts/autohand.rs`、`autohand_tests.rs`，只在可隔离挂载时实现。
- 按实证必要修改：`crates/vw-acp/src/access_policy.rs`、`crates/vw-acp/src/client/access_policy.rs`；新增 `crates/vw-acp/src/access_policy/autohand.rs`、`autohand_tests.rs`，只在运行时原生权限契约得到证明时增加供应商分支。
- 桌面复核入口：`crates/vw-desktop/src/app/components/system_settings_agents/catalog.rs` 与 `crates/vw-desktop/src/app/state/settings/agents.rs`；默认无需修改，目录应由网关派生。
- 新建证据：`docs/dev/evidence/acp-onboarding-autohand.md`，附必要的脱敏协议样本，不包含凭据、用户消息正文或本机私有配置。

## 上下文

- 从 `vw-acp::built_in_agent_specs` 派生官方 `acp-autohand` 预设，沿用 Codex/Claude 既有安装、协议和安全门控。先读现有实现及独立测试，不在桌面绕过底层。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。

## 来源契约与边界

- 来源启动基线：程序 `npx`，argv `["-y", "@autohandai/autohand-acp"]`；这是需验证的来源记录，不是已经通过本仓库验收的命令。
- 来源 npm 包 `@autohandai/autohand-acp`，历史核验版本 `0.2.1`。实施时核验官方发行信息并固定实际测试版本，不能把历史版本当当前推荐。
- 来源技能目录：来源未声明目录，先检查真实运行时；不能猜测目录。
- 适配器依赖独立 Autohand 主 CLI；安装桥接包不代表主程序已就绪，缺失时给出可操作错误。
- 对照 `crates/vw-acp/src/agent_registry.rs`、`src/client/protocol.rs`、`src/client/process.rs`、`src/access_policy/codex.rs`、`src/access_policy/claude.rs` 及对应测试；模型来源参考 `crates/vw-agent/src/session/llm/acp/model_source/drivers/codex.rs` 和 `claude.rs`，技能参考相邻同名模块。
- 保留官方 CLI 本地配置路径；只有本会话选中的模型可通过现有中继绑定。不能复制参考项目令牌、默认模型、Electron 逻辑、品牌或数据库迁移。
- 未证明原生完全访问时返回 ClientOnly/Unavailable 等准确结果；宿主拒绝、只读、取消和版本变化均不能提升权限。上游模式名称不能直接视为授权。
- 新/恢复会话中的模型、MCP、技能、权限和租户/项目身份保持一致；能力不足明确报错，不能静默回退其他引擎、CLI 默认模型或其他会话。

## ACP 与内置智能体双重交付

- 必须同时交付 `autohand` 的 ACP 运行链路和系统内置智能体 `acp-autohand`，与现有 Codex、Claude Code 的内置预设一致；只添加命令/安装项不算完成。
- 内置目录通过 `crates/vw-agent/src/agent/catalog.rs` 的 `merged_agent_catalog` 派生，要求 `builtin=true`、`runtime=AgentRuntime::Acp { backend: "autohand" }`，补齐显示名称、用途说明、推荐提示词和默认模型/权限/技能/MCP 继承。沿用现有默认启用规则及子代理限制，不自动扩大权限。
- 新增或修复必要的内置预设、配置持久化和会话选择连线：官方智能体目录可搜索，启用后出现在已启用目录及会话智能体选择器，可编辑保存默认项；重开设置/应用后保持配置；通过该预设新建会话确实启动 `autohand`，不能落回 main、其他 ACP 或另一同名预设。
- 必读/按需修改：`crates/vw-agent/src/agent/catalog.rs`、`crates/vw-agent/src/config/schema/config_validate.rs`、`crates/vw-desktop/src/app/state/settings/agents.rs`、`crates/vw-desktop/src/app/message/settings/agents_profile.rs`；沿用已有配置和 UI 链路，不维护第二份手写目录。
- 新增失败测试：在 `crates/vw-agent/src/agent/catalog_tests.rs` 添加 `acp_onboarding_autohand_builtin_preset_metadata`，覆盖内置标识、backend、名称/说明/推荐提示词及默认选择；在 `crates/vw-agent/src/gateway/api/handlers/session/agent_profile_acceptance_tests.rs` 添加 `acp_onboarding_autohand_builtin_profile_persists_and_runs`，通过生产配置与流入口验证保存/重读、启用/停用、新会话实际路由和身份固定。
- 失败/通过验证命令均为 `cargo test -p vw-agent --lib acp_onboarding_autohand -- --test-threads=1`；先确认新增用例因内置预设缺失或路由不完整而失败，完成后至少上述 2 个新测试实际通过，零测试无效。只替代外部 CLI/模型服务边界，不 mock 目录或持久化实现。
- 真实桌面验收必须从“官方智能体→启用→选择该内置智能体→新建会话”进入，保存并重开默认配置，检查实际后端与会话身份，完成两轮消息及现有能力验收；记录暗亮主题和证据。不得仅从 ACP 调试入口或手动指定 backend 绕过内置智能体流程。
- 完成门槛：ACP 适配与内置智能体任一缺失、配置保存失败或会话选择未打通，均不得标记 `已完成`；证据文档分别列出这两项交付结果。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

## 测试计划

- crate/features：`vw-acp`、`vw-config-types`、`vw-agent` 均使用其默认 features；仅规定的 clippy 使用 workspace `--all-targets --all-features`。已检查 workspace Cargo.toml 及这些 crate 的 Cargo.toml，不为此任务新增 feature。
- 行为：`autohand` 的官方预设解析到正确安装/启动契约，在同一个 ACP 会话完成首轮、第二轮与取消，拒绝操作确实被拒绝。
- 失败测试：先创建上述独立测试文件，以公开 `built_in_agent_specs` / `resolve_agent_spec` 验证注册和 argv，并在配置测试、目录测试中验证安装映射与 `AgentRuntime::Acp` 绑定；测试名前缀统一 `acp_onboarding_autohand`。
- 失败验证命令：`cargo test -p vw-acp --test onboarding_autohand -- --test-threads=1`；`cargo test -p vw-config-types --lib acp_onboarding_autohand -- --test-threads=1`；`cargo test -p vw-agent --lib acp_onboarding_autohand -- --test-threads=1`。
- 预期失败：新增断言因 `autohand` 尚未注册、没有安装映射或没有官方预设而失败；不是语法/依赖错误，也不能用零测试当失败验证。
- 通过验证命令：修复后重跑以上三个命令。每条命令有效测试至少 1 个，ACP 集成目标至少覆盖上述 3 个用例；模型/技能/权限新增测试也必须使用统一前缀并实际执行。
- 模拟策略：注册、目录、安装选择和 JSON-RPC 编解码使用生产代码；仅外部进程/网络通过临时可执行的 stdio ACP 对端替代，使用真实协议帧验证生命周期和拒绝回包，记录 initialize/new/prompt/cancel 形状。不得仅断言 mock 调用次数。真实 CLI 另行验证身份、实际模型请求与工具结果。

## 步骤

1. 阅读仓库 AGENTS、只读 plan.md、差异清单、本任务和相邻 Codex/Claude 实现；记录工作区现有变更，确认前一批任务已完成或已完成且仅存在不影响本接入的待回归项。源码/API 依赖仍未满足则先记录证据，不跳过依赖。
2. 从来源源码包只读提取对应迁移、运行时和能力定义；查该产品官方文档、源码与发行元数据，确认真实 CLI/适配器关系、版本、安装方式、支持平台及认证。必要时在网关隔离目录执行无凭据握手，记录来源与版本，禁止安装猜测的同名包。
3. 先添加并执行失败测试，确认缺失契约的预期失败；动态领取 Cargo 槽位，清理规则覆盖成功、失败和中断。
4. 在现有注册和安装白名单添加最小分支；本地 CLI 发现与备用启动参数均需验证。npm 包按已验收版本固定；二进制下载/安装只接受固定审核来源和平台映射，绝不能让客户端提交任意 URL、包名或 shell。上游未发行的平台返回准确不支持错误。
5. 由现有 `merged_agent_catalog` 派生 `acp-autohand`，保持默认停用及默认不允许子代理；验证启用/停用、保存/重开及实际调用的后端身份。不要为此新增平行配置目录或手写桌面供应商列表。
6. 运行真实 CLI 的 initialize、session/new、两轮 prompt、cancel 以及它真实支持的 load/resume；测试拒绝文件/命令操作、错误认证、缺失可执行文件、错误模型和进程退出。客户端对端测试证明宿主逻辑，真实 CLI 测试证明供应商行为，两者证据不可互替。
7. 对照来源和实际握手逐项核验模型目录/切换、CLI 本地模型、会话模型绑定、权限模式、MCP stdio/http/sse、技能目录和斜杠命令。仅将真实支持项接入现有分派；上游明确不支持的功能给出稳定拒绝并写证据。不能以“全部不支持”代替调查已有上游能力，也不能为了显示可用而取消现有门控。
8. 对本产品支持的会话模型来源，使用受控本地上游响应服务和既有中继证明 provider/model 到达实际请求；认证只进入该子进程，日志/持久化不含真实密钥，CLI 原全局配置不被改写。对技能使用临时隔离目录，证明只发现选中的技能；对 MCP 验证仅所选服务进入会话，取消/恢复不扩大范围。
9. 运行 `cargo fmt --all` 后审查差异，仅保留本任务格式变更；执行聚焦通过验证及规定 clippy。实际桌面检查暗/亮主题的目录、状态、启用与会话选项；复用既有表单、4px 滚动条和 toast，中文不得使用等宽字体；变更字体或动态弹窗时确认实际整形/渲染路径无 panic。
10. 记录当前证据、平台限制和来源明确不支持项。最后审查之前所有已完成接入的注册仍保留，Codex/Claude 的关键行为未变。完成状态、提交及任务文件清理由本任务“完成”契约处理。

## 验证

- 聚焦：`cargo test -p vw-acp --test onboarding_autohand -- --test-threads=1`；`cargo test -p vw-config-types --lib acp_onboarding_autohand -- --test-threads=1`；`cargo test -p vw-agent --lib acp_onboarding_autohand -- --test-threads=1`，记录每条退出码与有效测试数量，零测试无效。
- 周边注册保护：`cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1`；`cargo test -p vw-agent --lib agent::catalog_tests -- --test-threads=1`。
- 规定检查：`cargo clippy --all-targets --all-features -- -D warnings`，必须执行，修复本次引入的错误/警告；既有无关失败记录文件位置与原始错误摘要，不借机扩展范围。
- 真实验收：在隔离网关配置和临时工作目录，通过实际官方目录启用并启动 `autohand`，记录安装/发现、agentInfo 版本、两轮消息完成、工具写入临时文件、拒绝操作、取消后可继续、load/resume 支持或明确拒绝。不得仅以握手或进程退出码判定可用。
- 平台验收：为安装选择写 Unix/Windows 数据测试；不冒充已在其他 OS/架构运行。官方支持但未真实测的平台保留待回归记录。
- 模型、权限、MCP、技能：证据文档逐项标注支持/上游明确不支持/环境阻塞；能力支持结论必须对应当前版本和当前运行结果。无账号时不借用其他账号、不伪造完成。
- 所需证据：前后行为测试输出、确切命令、退出码、有效测试数、实际 Cargo 槽位及绝对路径 CARGO_TARGET_DIR、官方来源 URL/源码路径和版本、脱敏真实 CLI 回执、桌面观察结果与已审查差异。

## 完成

- 只在上述当前证据证明一个产品完整接入时标记 `已完成`；上游明确不支持的可选能力须已验证拒绝路径，不能把关键安装/会话失败算成产品完成。
- 外部依赖或缺少凭据导致核心会话无法验收：先调查合理替代路径，仍无解则仅在本文件标记 `已阻塞`，写具体原因、已尝试命令和所需输入；不得用产品登记成功替代真实接入。
- 只有无关既有编译错误等导致验证无法执行、且实现已完成时可标记 `已完成，待回归`，保留本任务文件与失败证据，不宣称已验收。
- 有实际代码/测试/配置/证据文档变更时使用 `$git-commit` 提交本任务变更，排除所有他人变更；仅任务状态或删除无需提交。最终报告完成证据，已完成则删除本任务文件；待回归/阻塞保留。不得修改 plan.md。


## 2026-11-13 恢复执行调查

- 批次 1 无前置依赖；已读取执行技能、只读 plan.md、差异清单及注册/目录代码。工作区有大量他人修改，未修改或提交这些文件。
- 来源变更：官方 https://github.com/autohandai/autohand-acp/blob/main/README.md 已归档旧适配器，要求使用 >=0.9.8 的原生 `autohand --acp`，保留 autohand ID。后续实现应采用当前原生契约，不继续推广历史 npx 桥接；旧适配器权限环境变量不能迁移为授权。
- 官方发行核验：`npm view @autohandai/autohand-acp version repository dist.tarball --json`（退出 0）仍为 0.2.1；`npm view autohand-cli version engines --registry=https://registry.npmjs.org --json`（退出 0）为 0.9.11、Node >=22。
- `curl -fsSL https://api.github.com/repos/autohandai/code-cli/releases/latest`（退出 0）得到 v0.9.11。下载官方 autohand-macos-arm64.tar.gz，在 /tmp/shiju-autohand-check 隔离解压，SHA256 与官方同名 .sha256 一致：d98de36979a1a4b6442a5b0ff746a2a5f5a5408e493c93c380aa4b3ef312fe24。`autohand --version` 退出 0，报告 0.9.11 / 46a28b16c05b1984761ca2b0daa700b6fc21ef70。
- 真实运行命令：Python subprocess 启动 `/tmp/shiju-autohand-check/autohand --acp`，仅传 PATH、隔离 HOME/AUTOHAND_HOME；未读取用户私有配置或继承凭据。NDJSON initialize(protocolVersion=1, clientCapabilities={}) 与 session/new(cwd=隔离目录,mcpServers=[]) 均成功。agentInfo=autohand-cli/0.9.11；声明 load、list/resume/fork、MCP http/sse、模型列表及 default/plan/yolo/automode。能力声明仅供下一轮调查，未验收权限或模型绑定。
- 首轮 session/prompt 实测失败：stderr 及 agent_message_chunk 报 `Error (auth_failed): Authentication failed. Please verify your OpenRouter API key in ~/.autohand/config.json. No cookie auth credentials found`；响应 id=3,error.code=-32603。探测脚本退出 0（只表示成功捕获失败），子进程探测后主动终止，不能作为产品通过证据。未保留提示词或会话正文。
- 替代路径调查：官方 https://github.com/autohandai/code-cli/blob/main/docs/config-reference.md 的 customProviders 节明确要求所有 provider（含第三方/自定义/本地）先 Autohand sign-in；不能用受控响应服务代替真实账号验收。`command -v autohand` 未发现本地 CLI；`command -v ollama` 未发现，`curl -fsS --max-time 2 http://127.0.0.1:11434/api/tags` 退出 7。没有借用其他账号或变更用户全局配置。
- 核心阻塞：隔离环境没有 Autohand 登录凭据及可验收模型认证，真实首轮失败，不能完成两轮、工具拒绝/取消/恢复及官方内置智能体桌面流程。需提供经授权的隔离测试账号/模型服务，完成 CLI setup 后恢复本任务。未把握手成功标为完整接入，ACP 与 acp-autohand 双重交付均未完成。
- 本轮仅更新本任务调查和状态；无 Rust/代码/测试改动，无 Cargo 调用，因此未领取槽位，CARGO_TARGET_DIR=不适用；未运行失败/通过行为测试、clippy 或桌面验收，有效 Rust 测试数 0。后续实施仍必须先动态领取槽位并按本文件执行 TDD，不得引用本轮探测代替。
- 任务保留以恢复实施；只读 plan.md 未修改；仅状态/调查更新按授权无需提交。


## 2026-11-13 安全替代路径复核

前次“所有 provider 必须账号”的阻塞结论已被真实运行推翻，不再作为停止依据。固定 v0.9.11 源码 `src/modes/acp/adapter.ts` 的 authenticate 允许 provider API key；独立 ACP 入口未强制走交互登录。使用隔离 AUTOHAND_HOME/config.json 设置 openai provider、仅本地假令牌与环回响应服务后，同一真实原生 CLI 会话两轮 prompt 均返回 stopReason=end_turn，HTTP 捕获 /v1/chat/completions 的 model=fixture-model（3 次请求，包括运行时附加请求）。没有使用外部账号、没有绕过或修改上游认证代码。探测脚本 `/tmp/shiju-autohand-check/local_probe.py` 退出 0。隔离工作目录须使用独立 workspace 子目录，不能让工作目录包含隔离 home，否则 CLI 正确拒绝 Unsafe workspace。

继续实现注册、安装白名单与内置预设验证；禁止沿用前次认证阻塞结论。


## 2026-11-13 最终交接：磁盘空间阻塞（不需要账号）

- 已解除前次认证阻塞，安全替代路径已真实通过：原生 v0.9.11、本地 OpenAI 响应服务、隔离配置，完成两轮、load、取消后续聊、批准一次工具写入、cancelled 与 reject_once 阻止写入，以及所选 MCP stdio initialize/tools/list。无需新增外部账号或借用凭据。
- 代码已增加 autohand 原生注册与本地发现（--acp）、固定 autohand-cli@0.9.11 安装白名单；官方可选 traces/computer-control 安装开关解决 postinstall 下载超时。acp-autohand 由现有目录自动派生，默认停用、禁止子代理；增加注册、安装、目录元数据及生产网关配置/后端测试。
- 有效前置失败：槽位 5 三条规定命令均退出 101，注册 1、安装 1、agent 3 个失败用例。网关的初次失败包含测试传输开关遗漏，不能冒充业务缺失证明；已修正。
- 阶段 2 槽位 1：cargo fmt --all 退出 0；cargo clippy --all-targets --all-features -- -D warnings 退出 0。中间测试对端及测试设置问题已修正，不能引用中间失败轮作为通过证据。旧注册保护受宿主已有托管运行时影响失败，最终安排隔离配置目录复核。
- 最终阶段槽位 3：cargo fmt --all 退出 0；cargo test -p vw-acp --test onboarding_autohand -- --test-threads=1 退出 0、3/3 通过；cargo test -p vw-config-types --lib acp_onboarding_autohand -- --test-threads=1 退出 0、1/1 通过。
- 最终 cargo test -p vw-agent --lib acp_onboarding_autohand -- --test-threads=1 退出 101、有效测试 0：rustc 1.99.1 在写 incremental pre-lto.bc 时报告 No space left on device (os error 28)，随后编译器 panic。生产预设持久化与路由尚未取得通过证据，不能标记完成或待回归。
- 同阶段注册保护退出 101/0 测试（query-cache.bin 无空间），目录保护退出 101/0 测试（database or disk is full，无法写 build-script-build）。最后 clippy 重跑在日志重定向时即因无空间失败，shell 退出 1，未实际启动；保留前阶段完整 clippy 通过证据，不将本次算通过。
- 实际 CARGO_TARGET_DIR 依阶段为 /Users/xiangmin/worker/code/shiju/target/rust-slot-5、/Users/xiangmin/worker/code/shiju/target/rust-slot-1、/Users/xiangmin/worker/code/shiju/target/rust-slot-3。各阶段均通过 scripts/cargo_slot.py 原子领取并补充本计划/任务记录；进程已结束，自己的锁已释放，共享缓存保留。
- 具体外部操作：请释放构建卷空间，建议至少留出 30 GiB；本任务不删除他人产物或违反规则清空共享 Cargo 缓存。df 实测可用 117 MiB。仅清理本任务临时下载的大型原生压缩包/二进制，保留脱敏日志和脚本以恢复调查。
- 空间恢复后先重跑上述三个聚焦命令及注册/目录保护、规定 clippy；确认 vw-agent 的 3 项新测试通过后，再完成“官方目录启用→默认项保存重开→选择预设→新建会话”的暗亮主题真实桌面验收。会话模型中继与技能目录仍保持未验收的明确拒绝门控，不能宣称可用；尚需按任务第 7–8 步补完实证和必要分派。
- 当前 ACP 与内置预设交付均为部分实现，缺完整生产路由/桌面及能力隔离证据，因此保留本文件，plan.md 未修改。详见 docs/dev/evidence/acp-onboarding-autohand.md。


## 2026-11-13 磁盘恢复后续执行

用户更新规则后重查，构建卷已恢复 2.6 TiB 可用；磁盘阻塞解除，不再要求账号或空间操作。通过 cargo_slot.py 持有常驻前台 shell，动态领取槽位 1，同时继承两项绝对缓存路径，开始重新验证。
