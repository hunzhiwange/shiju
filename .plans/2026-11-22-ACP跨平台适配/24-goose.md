# 任务 24 适配 goose

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 goose 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/goose.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/goose.rs`、`crates/libs/acp-access-policy/src/access_policy/goose.rs`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_goose_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/goose.md`、`docs/dev/acp/skill-coexistence/goose.md`。

## 上下文

审计基线版本为 1.57.0。MCP 现状｜原生扩展接纳 stdio；受管历史目录 10/18/10，真实执行回传。模型来源现状｜原生 Provider/模型，受审；通用。原生工具现状｜普通及挂载保留官方工具，挂载实测两次 tree；三项服务限制来自转发器。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（模型锁、wire 模型及 Skill 隔离），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_goose_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_goose_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| Linux GNU 目标 | 官方普通制品 SHA-256 | 受管候选 |
| --- | --- | --- |
| x86_64 | `goose-x86_64-unknown-linux-gnu.tar.bz2`，`e54036f3b7f95d3b0d7854c09465af5ef52a28c7fa03d2497d0ea68a6382282d` | `goose-v1.57.0-model-source-v3-x86_64-unknown-linux-gnu/goose`，ELF machine 62 |
| aarch64 | `goose-aarch64-unknown-linux-gnu.tar.bz2`，`17c6b090db6acc5605da904bbe19cc2765bc7a6786019d85a6d0a361de06dbf7` | `goose-v1.57.0-model-source-v3-aarch64-unknown-linux-gnu/goose`，ELF machine 183 |

- 两个官方归档的版本与摘要来自 [v1.57.0 Release API](https://api.github.com/repos/aaif-goose/goose/releases/tags/v1.57.0)。普通 `cli_local` 继续走 `agent_registry` 的 `goose acp` 和本机配置。官方二进制缺少受管模型锁及技能视图参数，需复用 [固定源码](https://codeload.github.com/aaif-goose/goose/tar.gz/refs/tags/v1.57.0) 和现有 v2 共存补丁。已核对归档 SHA-256 `36fcab524e25ca563830757bf2da8a68716ea2a5d7ca2f055dec4647b78548d9`、补丁 SHA-256 `3b75b96dc038b60f294de02354de34150a204e5e08f51a5bd37ff9095898ebcb` 及补丁可应用性。Apple Silicon 保留原 v2 目录、回执格式、驱动和历史能力。
- 任务 48 的安装接线。在 `acp_agent_registry::managed_runtime` 按 Linux GNU x86_64/aarch64 声明 `#[path = "managed_runtime/goose.rs"] mod goose;`，在 `installed_directory` 选择表中含 `goose::TARGET` 的版本目录。以后若启用受管候选安装，在 `install` 传入目标给 `managed_runtime/install.py` 的 Goose native 分支，复用固定 v2 源码、补丁和锁文件。Python 3、patch、Cargo 和宿主 GNU 工具链为构建入口依赖。上游固定 Rust 1.99.0，GNU CI 安装 build-essential、pkg-config、libssl-dev、libdbus-1-dev、libxcb1-dev。按本机目标使用 `cargo build --target <TARGET> -p goose-cli --bin goose --no-default-features --features rustls-tls,bundled-mcp --locked`，在验证阶段用公共槽位。保存对应 `<target-dir>/<TARGET>/debug/goose`，向原回执新增 `target`。沿用现有摘要和原子安装流程。未编译、未推断已安装 ELF 的动态链接依赖或目标摘要。[固定版 GNU 构建工作流](https://github.com/aaif-goose/goose/blob/v1.57.0/.github/workflows/build-cli-linux.yml)
- 任务 48 的启动和身份接线。Linux GNU 的 Goose `verify` 分支调用 `goose::verify(directory, contract)`，核对目录和文件类型、可执行位、ELF 架构、构建回执与流式程序摘要。安装、发现和恢复都用同一验证。Linux 的 `verify_reviewed_agent_executable` 只接受 `reviewed_agent_command` 返回的版本化安装入口，阻止走普通 native 的任意目录回执和旧摘要分支。回执是本地构建回执，无上游签名；构建输入和安装根由宿主管理。入口参数固定 `["acp"]`，`model_source::prepare_profile` 验证入口后由 `drivers::goose::configure` 加 `--shiju-model-source`，私有 `GOOSE_PATH_ROOT` 和回环令牌锁定 OpenAI Chat Completions 的实际模型。现有 `skill_mounts::goose::configure` 验证 v2 回执、选择私有视图并屏蔽空选择，原生同名技能优先。保留普通路径与现有对外能力，Linux 候选存量不构成新增能力发布证据。
- 任务 48 在 `acceptance_support/mod.rs` 以 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_goose_tests;` 注册契约。旧 Python 探针在制品检查即拒绝 Linux。新探针使用实际网关、目标回执和真实 Goose，模型替身只接纳带回环认证的 `/v1/chat/completions` 请求。断言真实开发者 `tree` 批准与续轮、原生和外挂技能正文和附件、外挂 stdio 的同 ID 工具续轮，以及挂载一次调用、拒绝和撤除零新增服务审计。撤除轮使用新会话。后续在两个非 root Docker 容器分别设置 `SHIJU_ACP_GOOSE_GATEWAY_BIN` 与 `ACP_COEXISTENCE_GOOSE_BIN`，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_goose_contract -- --include-ignored`。未运行 Docker、Cargo、网关、Goose 或夹具，暂无红绿退出码和实际命中数。
- 两架构的完整运行契约都阻塞。上游 `prepare_session_for_activation` 对非空列表合并已保存扩展，空列表不清除；`handle_load_session` 还能继续处理历史中未应用的许可答复。现有 v2 补丁保留历史但未记录会话 MCP 的归属，也未更新恢复的执行授权。现有宿主 Goose 使用 `approve` 和临时答复，供应商的 `PermissionInspector` 仍允许显式 `AlwaysAllow`。技能与 MCP 共存夹具没有覆盖原生静态 MCP、在飞调用取消和同会话撤销。需要在生产恢复阶段按归属替换会话 MCP，并在调用执行前重新校验当前轮次、服务、调用 ID 和参数，验证已保存批准和已撤销服务均无法执行。Linux 两架构先保持现有拒绝，由任务 49 分别登记。[固定版会话恢复](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/server/load_session.rs) · [固定版会话扩展合并](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/acp/server.rs) · [固定版工具审批](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/permission/permission_inspector.rs)
- 本轮只做归档和补丁摘要、补丁试应用、Python AST、定向 Rust 格式以及 Git 空白检查。

## 续审交付

- Linux GNU x86_64/aarch64 受管模型候选升级为 v3。固定上游和 macOS v2 保持原值。任务 48 的 Linux 原子安装在既有 v2 补丁之后应用 `patches/acp-model-source/goose/linux-session.patch`，预先核对 `ad6eda848802728e2efadab5677d61f740e3dc64a823e18625d807cf37a63ebe`，存入回执 `linux_patch_sha256`。原 `patch_sha256` 仍为 v2 技能补丁摘要，用于原技能视图校验。Linux 验证模块同时核对两层补丁和目标回执。任务 48 接入验证模块及契约测试后，普通安装、注册表能力和公开门控沿用现状。
- Linux v3 的模型锁路径强制 `approve` 或禁工具的 `chat`，保留显式拒绝，将本机持久允许降为逐次审批。普通 `goose acp` 清除模型锁，Apple Silicon 不命中 Linux 分支。供应商只接受当前未取消运行返回的一次性答复。通用工具分发前核对同轮唯一确认的调用 ID、工具名称与精确参数，阻止同轮 ID 重放，并在进入扩展调用前检查取消。供应商清单保留有效的原生扩展，拒绝会话 MCP 名称碰撞。恢复使用当前目录与请求 MCP 重新构建扩展清单，空列表撤除历史外挂，原生扩展从当前配置重新发现。活跃期间拒绝加载，普通完成的历史可继续。包含未应用审批的中断历史失败关闭，避免自动续用已失效的批准。这些行为只在固定 v3 补丁应用后的 Linux managed ACP 路线生效。
- 新增同一 ACP 会话跨进程恢复断言，真实 Goose 先接入 stdio，再从同一临时历史库加载空 MCP，核对原生 `tree` 保留、旧服务从真实目录消失且审计零新增。之前的真实网关夹具仍验证原生工具、原生与外挂技能、外挂 MCP 获准执行后续轮及拒绝审计。后续在两个非 root 容器分别执行原定过滤命令并附 `-- --include-ignored`。两层补丁已顺序应用到摘要匹配的固定源码副本，修改后的 Rust 通过 rustfmt 解析，仓库侧 Rust 格式、Python AST、补丁摘要和定向 Git 空白检查通过。未执行测试或编译。
- 仍为实现阻塞，影响 Linux GNU 两架构的原生静态 MCP 与完整宿主 MCP 审批契约。固定版 `prepare_acp_session_agent` 在提示轮之前调用扩展租约 `start()`，仓库或插件中的原生 stdio 可以在首次工具审批前启动进程。受管宿主不能以当前 ACP 工具审批批准这条启动路径。同时，Goose 的 `mcp_client` 在支持的传输上已经能携带 `agent-tool-call-request-id`，但任务 4 的宿主 relay 尚无按该 ID 查找并消费许可的执行 API，且项目原生 MCP 未经该 relay。复用 ACP 的一次性答复可以约束 Goose 已发现工具的分发，无法证明项目配置命令的启动授权或 relay 的调用 ID 绑定。禁用原生 MCP 会丢失要求的能力，独占补丁也无法替宿主补齐跨层授权。需由公共宿主提供受管启动审批/目录投影和执行端调用 ID 消费，再接入 Goose 元数据并回归两架构。此前保持对外门控。这个缺口独立于尚未运行的 Linux 测试。
