# 任务 8 适配 claude

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 2】 依赖：批次 1

状态：已阻塞

来源任务：无

目的：完成 claude 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/claude.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/claude.rs`、`crates/libs/acp-access-policy/src/access_policy/claude.rs`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_claude_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/claude.md`、`docs/dev/acp/skill-coexistence/claude.md`。

## 上下文

审计基线版本为 0.26.0 / SDK 0.2.96 / Code 2.1.96；原生 Messages 另审 0.31.0。MCP 现状｜stdio 已启动，旧首轮 Messages 遇 context_management 422；兼容修复后同 ID 回传。模型来源现状｜原版可选模；受审版本依协议，0.31.0 不用于转换；通用。原生工具现状｜原生权限生效；私有项目 deny 才隐藏官方目录，Claude 会话元数据是独立的显式名单入口。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

1. 分开审查 0.26.0 的模型转换与 0.31.0 的原生 Messages。复用任务 3 的 Linux 私有状态目录校验，核对内嵌 SDK 的 Linux 二进制、终端入口与原生技能和插件视图。在本编号文件记录可供批次 3 复用的实现与接线模板，覆盖 SDK 与持久状态。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_claude_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_claude_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

- GNU x64 和 ARM64 的转换入口均为 `@agentclientprotocol/claude-agent-acp@0.26.0` 的 `dist/index.js`，SDK 固定 `@anthropic-ai/claude-agent-sdk@0.2.96`，其 `cli.js` 为同一份 Code 2.1.96 JavaScript 制品。相同入口也可配置原生 Messages。只有转换保留 `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`。外部 Node 使用已有审计版本 24.22.3。
- 0.31.0 原生 Messages 使用同名 npm bin，SDK 固定 0.2.119。GNU x64 选择 `@anthropic-ai/claude-agent-sdk-linux-x64@0.2.119` 的 ELF `claude`，静态检查 ELF machine 为 62。GNU ARM64 使用 `@anthropic-ai/claude-agent-sdk-linux-arm64@0.2.119`，ELF machine 为 183。两包声明 Linux 和 glibc，Code 版本 2.1.119。逐包用 npm 元数据的 SHA-512 校验原始 tarball，再对清单、JS 入口和 ELF 计算源码中的 SHA-256。仅审读取制品，未安装或执行。源为 npm 的 [适配器 0.26.0](https://registry.npmjs.org/@agentclientprotocol%2fclaude-agent-acp/0.26.0)、[适配器 0.31.0](https://registry.npmjs.org/@agentclientprotocol%2fclaude-agent-acp/0.31.0)、[SDK 0.2.96](https://registry.npmjs.org/@anthropic-ai%2fclaude-agent-sdk/0.2.96)、[SDK 0.2.119](https://registry.npmjs.org/@anthropic-ai%2fclaude-agent-sdk/0.2.119)、[GNU x64](https://registry.npmjs.org/@anthropic-ai%2fclaude-agent-sdk-linux-x64/0.2.119) 和 [GNU ARM64](https://registry.npmjs.org/@anthropic-ai%2fclaude-agent-sdk-linux-arm64/0.2.119)。
- 任务 48 安装输入。按版本分离宿主拥有的 npm 前缀，使用 `--ignore-scripts --include=optional --save-exact` 生成固定顶层包和完整锁，正式安装使用审查后的锁及 `npm ci --ignore-scripts --include=optional`。两个前缀分别显式选择适配器和对应 SDK，0.31.0 再显式固定本机 GNU 引擎包。0.26.0 只使用内置 `cli.js`，不得混装其他 Claude SDK 引擎。0.31.0 的 SDK 会先探测 musl，GNU 安装不得混装 musl 引擎。入口是各前缀绝对路径 `node_modules/.bin/claude-agent-acp`，参数为空。路由确定为转换后选择 0.26.0，原生 Messages 可选择 0.31.0 或 0.26.0。普通 `cli_local` 的发现、npm 范围回退和原生账号继续由注册表处理。
- 任务 48 启动输入。Linux 限 GNU x86_64/aarch64。在 `acp-client/src/client/process.rs::spawn_child_at` 的 Claude managed 分支调用 `access_policy::claude::verified_linux_engine(command, args, native_messages)`。该入口拒绝相对路径和 CLI 参数。转换只能用 0.26.0，原生 Messages 接受经过校验的 0.26.0 或 0.31.0。从校验返回值派生子进程的 `CLAUDE_CODE_EXECUTABLE`，禁止配置或外部环境指定它。上游 0.26.0 和 0.31.0 都接受该变量作为 SDK 显式引擎路径。仅验证本地 npm 依赖不足以排除 Node 的其它解析位置。保留 0.31.0 的现有 Darwin 运行门控，Linux 新校验入口在共享启动接线前不启用新路由。非 GNU 和其他架构没有新增能力。共享权限入口还需审查 `acp-client/src/client/protocol.rs::initialize_connection` 当前只认 Darwin arm64/0.26.0 的权限身份条件。不要照搬成 Linux 或 0.31.0 的免审批条件。
- SDK 与持久状态模板。沿用 `prepare_profile -> configure_profile -> drivers::claude::configure`，通过 `claude_state_directory` 验证 0700 的租户会话历史目录，再在本次私有启动目录写入 `settings.json`。会话 new/load/resume 通过 `session_meta.rs` 发送当前模型、私有设置及活动插件。项目原生技能目录为 `.claude/skills`，外挂使用 `skill_mounts::claude::prepare` 的 `.claude-plugin/plugin.json`，由当前会话投影提供。原生 `.mcp.json` 由 SDK 的 project settings 来源发现，外挂 MCP 由当前 ACP 消息传入。会话恢复必须重建当前 MCP 和插件元数据并更新授权。启动回收与令牌撤销沿用任务 3 的生命周期。源码沿用 Apple Silicon 的选模、软权限与插件路径。
- 具体缺口。两种 GNU 架构均缺正常网关的固定引擎接线，以及真实 Claude 的 readback、原生预允规则与宿主审批的优先级验证。SDK 的 `canUseTool` 只在 Claude 要求审批时回调，不能仅凭回调存在保证每个原生调用受当轮宿主授权。对外挂 stdio MCP 若不能证明直接原生调用受控，任务 48 需经现有 `Host::prepare_stdio_mcp_launch` 和 `managed_mcp` 审批代理，或保持该组合关闭。当前测试覆盖制品身份、转换判定、私有状态及 SDK 设置，不覆盖工具成功续轮或拒绝、撤除后的服务审计。后续增补真实宿主 fixture 时，原生和外挂 MCP 应使用不同服务和调用审计，原生和外挂 Skill 应断言不同命名空间，分别采集批准、拒绝、撤除后恢复的工具执行和同 ID 结果。现有 Darwin coexistence 测试可提供模型和 MCP 外部替身。
- 定向测试已通过既有 `host_migration_21_tests::claude` 模块挂入，任务 48 无需重复注册。两种 GNU 架构均需将 `SHIJU_TEST_CLAUDE_026_ENTRY` 和 `SHIJU_TEST_CLAUDE_031_ENTRY` 指向相应隔离前缀的绝对 bin，再运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_claude_contract -- --ignored --nocapture`，预期命中一个制品测试。该测试显式忽略以免普通测试依赖外部安装，`--nocapture` 输出选中的版本与协议。原定过滤命令不带 `--ignored` 时预期运行一个企业配置测试。旧 Linux 断点为 0.31.0 仅接受 Darwin 引擎。已审查固定制品、调用点和定向差异。Linux x64 和 ARM64 的编译、测试、安装与运行均未执行。

## 复查与独立交付

- 两种 GNU 架构使用同一个按协议选引擎入口。测试覆盖 0.31.0 拒绝转换、拒绝附带参数，以及私有配置和路由标记。0.26.0 捆绑 Code 静态源码确认 Linux 加载 `/etc/claude-code/managed-settings.d`。驱动在写私有模型设置前检查基础文件和全部可加载 JSON 片段。允许组织 deny 等现有规则生效，遇路由覆盖或不可核验片段直接拒绝。独占测试覆盖权限规则、路由冲突、隐藏文件及失效的符号链接。
- 当前仓库没有可移植的 Claude 受管补丁。Codex 补丁只固定本机进程入口，Autohand 补丁的事件和授权协议与 Claude 不同。两版 Claude 使用 `canUseTool` 向客户端传工具 ID 和原始参数，但 SDK 的原生预允与权限模式可避开该回调。SDK 有 `PreToolUse` 扩展点，当前没有两个固定版本对组织策略、外挂插件、子代理及恢复后继续执行的强制回调证据，因此不以仅添加 hook 或 ask 规则开放权限。原生文件和 Bash 可能由 SDK 在本地执行，宿主的 ACP 文件及终端回调只能控制确实发到宿主的调用。相关优先级参见 [Claude Code 设置文档](https://code.claude.com/docs/en/settings)。
- 现有 `mimo_mcp::Projection::approval_config` 可复用逐次授权、工具名与参数比对、提示轮撤权和回环 HTTP 合同。其 `mimo_mcp::prepare` 平台断言为 Darwin arm64，启动固定依赖 `/usr/bin/sandbox-exec`，且宿主只给既有后端赋予审批投影视图。`mcp_client::open_isolated_stdio_peer` 仅提供无宿主回调和进程生命周期的 IO，隔离由传入命令负责。Bubblewrap 通用包装没有本投影的显式工作目录、私有 scratch 和进程执行边界，且会丢弃传入命令的环境及 cwd，不能直接替换 Darwin 包装。GNU x64 和 ARM64 的宿主选定 stdio 外挂 MCP 均卡在同一 Linux 隔离实现及共享接线。仅转发 ACP stdio 参数会让供应商自行启动进程。
- 可独立交付的是固定的双架构制品、版本及协议判定、企业模型配置拒绝和已接入的独占静态契约测试。任务 48 接通前保持现有能力门控。两架构的原生工具及 MCP、外挂 MCP、工具续轮和拒绝/撤除后零新增审计仍缺生产保护或完整验收源码，状态保持已阻塞。后续需先使宿主控制每次受管调用并接通隔离，再获 Linux 运行授权完成验收。
- 实际提交为 `a0006e3e8f`（双架构制品身份）和 `bd08e23b5`（协议选择与企业配置片段）。定向差异检查和 Rust 语法解析通过，未编译或执行测试。

## GNU 隔离落地后的候选分档

- 纯模型档已有 0.26.0 独占候选，GNU x64 与 ARM64 共用补丁及 SDK 0.2.96 内置 Code。补丁沿用 ACP 会话入口，在最终 query 处用构造的 SDK options 取代透传，固定原生 Messages 和模型，并以私有配置目录作为运行 cwd。引擎采用 `--bare`、空 tools、空 settingSources 和 strict MCP。启动回读 MCP 必须为空。拒绝挂载、账号认证、模式及模型修改、slash、非文本与历史恢复。本档没有工具、MCP 或技能。普通账号继续走原版。候选保留独立身份，现有生产验收和外部门控保持现状。
- 受控读取档优先用一个宿主审定 GNU stdio 服务。共享 GNU 隔离、目录和一次消费已可复用，剩余断点在 Claude SDK。固定 0.2.96 的 `PreToolUseHookInput` 包含 `tool_use_id`、工具名和输入，`SDKControlMcpMessageRequest` 只有 `server_name` 与内层 JSON-RPC。SDK 转发内层消息，原生 HTTP MCP 请求同样缺 `shijuCallId`。使用参数或队列猜配会把不同调用混为同一授权。本档需固定引擎最终 MCP 派发点，将实际调用 ID 传给现有投影，或实现等价可核验接口。当前拒绝执行该档，未加入未经控制的本机命令或供应商自行发现。
- 任务 48 接线输入为独立前缀、`patches/acp-model-source/claude/apply-model-only.py`、`verified_linux_model_only_engine(command, args)` 和现有私有 Messages profile。需要在实际网关候选路径中改用返回的固定引擎并验收一次纯文本请求。制品测试变量为 `SHIJU_TEST_CLAUDE_MODEL_ONLY_ENTRY`，独占测试过滤为 `acp_platform_claude_contract_model_only_identity -- --ignored`。候选可独立交付给两架构，原生 Messages 回环验收仍待执行。原任务范围还含受控读取、原生工具、MCP、技能与恢复，状态仍为已阻塞。
- 静态核对 SDK 的 CLI 参数生成、Code 2.1.96 的 bare/strict MCP 实现、hook 和 MCP 数据类型以及实际补丁选项。用固定 npm 源文件在临时目录重建补丁并检查 JS 语法及输出摘要。未执行编译、单元测试、SDK、Claude 或真实模型。
- 该轮候选、独立身份及测试源码已提交 `2d9a3171e`。
