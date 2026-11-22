# 任务 44 适配 stakpak

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 stakpak 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/stakpak.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/stakpak.rs`、`patches/acp-model-source/stakpak`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_stakpak_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/stakpak.md`、`docs/dev/acp/skill-coexistence/stakpak.md`。

## 上下文

审计基线版本为 0.3.88。MCP 现状｜new 忽略三种声明；启动 HTTP 可发现、批准、执行并回传。模型来源现状｜原版私有 Provider 可选模；受审 TOML 与中继；通用。原生工具现状｜v2 挂载仅 MCP；v3 均保留官方 view/create/str_replace，挂载加当前目录，其他原生工具仍受限。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v3 校验 cwd、重启原生入口、单次批准和标题去冲突），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_stakpak_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_stakpak_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 宿主 | 上游 0.3.88 制品 | 静态身份核对 | Linux 运行 |
| --- | --- | --- | --- |
| x86_64 | `stakpak-linux-x86_64.tar.gz`，唯一成员 `stakpak` | 归档和文件 SHA-256 匹配，ELF64 x86-64，静态 PIE，无 ELF 解释器 | 未运行 |
| aarch64 | `stakpak-linux-aarch64.tar.gz`，唯一成员 `stakpak` | 归档和文件 SHA-256 匹配，ELF64 AArch64，静态链接，无 ELF 解释器 | 未运行 |

- 核对了两份实际归档、GitHub release API 公布的 SHA-256、单一可执行普通文件成员以及 ELF 架构。固定 URL、归档和成员摘要、目录及参数见 [Linux 普通 CLI 清单](../../patches/acp-model-source/stakpak/linux-cli.json)。上游两个发行目标均为静态 musl，可以作为 GNU 宿主的普通 CLI 制品。制品不需要 Node.js、Python 包依赖或构建工具。固定安装器本身使用现有 Python 3 与 HTTPS 下载工具。[0.3.88 发行页](https://github.com/stakpak/agent/releases/tag/v0.3.88) · [固定版构建工作流](https://github.com/stakpak/agent/blob/v0.3.88/.github/workflows/build-and-release.yml)
- 任务 48 按 Linux GNU x86_64/aarch64 条件在 `managed_runtime.rs` 声明 `#[path = "managed_runtime/stakpak.rs"] mod stakpak;`，公开 crate 内的普通 CLI 查找入口，调用 `stakpak::cli_local_command(&root("stakpak")?)`。在 `agent_registry.rs::built_in_agent_command` 的 Linux Stakpak 分支使用该查找入口，未安装时保留原有系统 `stakpak acp` 发现。已安装入口使用绝对路径，参数仅 `acp`。校验器拒绝重定向的修订目录、非普通文件、缺执行位和不匹配的文件摘要。受管 `reviewed_agent_command`、`verify_reviewed_agent_executable` 不应将这个原版制品认作受管启动器。
- 任务 48 在 `install_builtin_agent` 和 `managed_runtime/install.py` 为 Linux 普通安装接入 `LINUX_CLI_CONTRACT`。按宿主架构选唯一 URL，仅用 HTTPS 下载，核对归档摘要，仅复制唯一预期普通成员到宿主管理的临时安装目录，固定文件权限 0700，核对成员摘要后按现有安装事务发布到 `acp-runtime/stakpak-0.3.88-linux-cli-local/stakpak`。发布前后的校验均调用普通 CLI 校验，沿用已有配置 API 的安装路径。Linux 安装能力仅表示普通 CLI 可装。Apple Silicon 的 `stakpak-0.3.88-model-source-v4`、构建回执和 launcher 摘要继续沿用原合约。
- 指定模型的原生协议是 OpenAI Chat Completions。现有 `drivers::stakpak::configure` 用受审私有 TOML、模型缓存及回环令牌固定工具模型，受管工具目录包含 `view/create/str_replace/load_skill`。原版 `session/new` 不消费 stdio/HTTP/SSE 声明。受管 v4 复用 v3 的 cwd 校验、重启及重新握手、启动期 HTTP MCP、官方工具保留、标题去冲突和单次批准。已核对 v0.3.88 源归档 SHA-256 `7ffae3233a6d97d4bf142392ce747a720b6c94717259ee6527deed91664eeb00`，SHA-256 为 `4873acd8bbdbcb0224db458256821e2cde986cee2298b7bc7433ec5bb616d7c4` 的 v4 补丁可在该源码干应用。源码路径复用锁文件及 Rust 1.94.1。Linux 受管构建的固定产物和运行证据仍缺失。
- 两种 Linux 目标继续拒绝 `session_model`、受管技能和 MCP。现有 launcher 用 Seatbelt 限制工作区直接写入、快照写入、进程执行和网络，只允许回环连接。启动期 HTTP 桥的 stdio 后端还经 Cortex Seatbelt 约束项目读取、临时写入、进程派生和网络。宿主通用 Bubblewrap 隔离整个网络命名空间，缺少受控的模型及 MCP 回环接入。现有 Landlock 包装没有对应的网络限制，也没有等价的路径和执行边界。任务 48 保留 `drivers::capabilities`、`skill_mounts::validate` 及请求门控，独占 Stakpak 驱动新增 Linux 拒绝，避免把普通制品或 Darwin 布局接进私有模型路线。需要两条等价的 Linux 进程隔离路径后再启用。
- 受管当前拒绝项目静态 MCP 覆盖。现有桥可处理新会话的选中 stdio 服务，MCP 帧按本轮已观察到的 ACP 请求、服务和工具、完整参数做一次匹配，执行帧缺少 ACP 原调用 ID，不能证明相同参数重入时的身份绑定。带挂载的恢复也在 launcher 拒绝。两个目标均缺原生静态 MCP 的执行前宿主授权、同会话挂载恢复及撤权、原生写入批准与拒绝的实证。保留原有官方文件回调与技能快照策略。任务 49 分别登记双架构阻塞，Apple Silicon 历史验收独立记账。
- 任务 48 在 `llm-runtime/src/acp/acceptance_support/mod.rs` 按 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]` 注册 `platform_stakpak_tests`。普通 CLI 测试以 `VIBEWINDOW_CONFIG_DIR` 的隔离目录核对真实注册入口、制品摘要与模型路由拒绝。原路由无法解析固定目录。非 ignored 测试锁定普通能力与未审入口拒绝。注册库的独占校验测试覆盖重定向、缺文件、缺执行位和内容不匹配。
- 完整网关用例设置 `SHIJU_ACP_STAKPAK_GATEWAY_BIN` 为当前网关绝对路径，并以非 root 用户设置 `ACP_STAKPAK_REVIEWED_DIR` 为该架构的隔离源码制品目录。现有夹具接受 v4 固定源码及补丁回执，独立检查文件并交给网关做自身校验。旧 Linux 路由在网关阶段拒绝受管模型或技能。未来 Linux 受管修订形成后，将夹具中的平台安装布局、文件摘要及回执同步到正式合约。四轮断言实际 Chat Completions 请求、原生和选中技能及附件区分、批准后下游执行与对应 ID 的模型续轮、拒绝及撤除零新增审计。完整网关用例目前无可用 Linux 受管制品。
- 本轮仅执行静态审查。源补丁干应用、归档与 ELF 核对、Python AST、JSON、局部 Rust 格式和定向 Git 空白检查通过。未安装、编译或执行测试。后续按原命令在两个非 root Linux Docker 目标分别运行，普通用例预期执行 1 项、标记忽略 2 项。接线后带 `-- --ignored` 并用测试全名分别选择普通 CLI 和网关用例，分别预期执行 1 项。注册库另用 `python3 scripts/cargo_slot.py cargo test -j 1 -p acp-agent-registry --lib acp_platform_stakpak_contract`，预期 1 项。实际命中、退出码及失败数未获取。
- 提交 `7b1ca90f24` 和 `53dadcf60`。后一提交按仓库锁定的 sha2 0.11 接口分块读取并更新摘要。

### 续查与追加交付

- 当前共享工作区已把普通 Linux CLI 接入 `linux_cli::discover/install/verify_launch`、注册表、安装可用性和 ACP 启动前校验。两架构的普通 CLI 子集仅待 Linux 定向回归。受管安装与能力继续失败关闭。
- 固定版 Stakpak 的 MCP proxy 从 `RequestContext.meta` 转发到后端请求。两架构复用同一 [受管 Linux 叠加清单](../../patches/acp-model-source/stakpak/linux-managed-overlay.json)。源码叠加层把 ACP 实际会话和调用 ID 写入执行帧 `_meta`，Linux 桥消费匹配当前会话、同一调用 ID、已审核服务及工具和完整实参的 15 秒一次性批准。旧桥的同参调用串领问题可在这一边界拒绝。取消、结束和撤销继续用现有生命周期清空许可。源码还拒绝无权限通道、持久批准，以及原生文件操作绕过 ACP 宿主文件回调。普通 CLI 和 Darwin 不使用叠加层。
- 任务 48 后续受管候选安装在独立 Linux 修订目录。按原合约核对上游源码归档和 `skills.patch`，先应用技能补丁，再验证并应用 `linux-approval.patch`。在暂存目录校验原版 `mcp.py` 摘要，应用 `linux-bridge.patch` 并校验输出摘要。复用现有锁定工具链和受管回执格式，Linux 修订需将叠加层作为回执输入核验，并在实际启动之前验证 Linux launcher、native、桥及隔离执行器。Linux 构建和 SHA-256 产物尚未生成。受管新目录禁止出现在普通 CLI 发现分支。
- 进一步复核现有 Linux 隔离替代。通用 Docker 与 Bubblewrap 的无网络路径无法访问宿主模型及桥的既有回环，宿主网络模式会放宽隔离。通用 Landlock 未实现逐命令包装，Firejail 的现有通用入口也没有只连接本轮回环服务、只执行固定 native、禁止直接写项目及快照的组合。需要可审计的逐进程 Linux 隔离及两端受控回环接入，或宿主等价机制。原版项目静态 MCP 还缺能和会话选集分别经已授权服务目录解析的入口，直接放行项目自定义启动配置会跨过审批。已挂载恢复需要宿主重新发布当前选择与有效状态后再绑定旧会话。Linux x64 与 ARM64 的完整合同均仍有上述实现缺口，维持已阻塞，任务 49 按架构登记。
- 静态核对了固定源码、全部调用者、锁定的 rmcp 0.11.0 元数据序列化，以及对原版技能补丁之后的源码叠加和桥脚本叠加。应用产物的 Rust 语法解析、Python AST、JSON 和定向 Git 空白检查通过。未来经授权可在隔离安装目录以 `PYTHONPATH` 指向经验证且已应用 Linux 叠加层的桥目录，执行 `python3 -m unittest discover -s patches/acp-model-source/stakpak -p mcp_linux_test.py`。测试断言同参的不同调用 ID、不同会话、改参、重放、撤销、晚到批准和超时均拒绝。未执行任何测试、构建、安装或运行上游制品。
