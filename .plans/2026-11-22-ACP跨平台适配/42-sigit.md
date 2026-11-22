# 任务 42 适配 sigit

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 sigit 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/sigit.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/sigit.rs`、`patches/acp-model-source/sigit`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_sigit_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/sigit.md`、`docs/dev/acp/skill-coexistence/sigit.md`。

## 上下文

审计基线版本为 1.7.10。MCP 现状｜旧入口前置拒绝；隔离原版可发现，调用分发报错且 stdio 继承环境。模型来源现状｜原生可选模；所有启动要受审，挂载显式 macOS，通用且驱动非 Unix 拒绝。原生工具现状｜普通与挂载保留宿主受控官方执行；v2 同轮实证，v3 未有完整真实三轮；逐服务名单未投影。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v3 沙箱 stdio、单次批准、失败回传、禁止文件本地回退及只读索引准备），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_sigit_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_sigit_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

实现及验收源码提交为 `5e1897a07`。

| Linux 目标 | 上游 1.7.10 对照制品 | 受管候选入口 |
| --- | --- | --- |
| GNU x86_64 | `@smbcloud/sigit-linux-x64@1.7.10` 的 `package/bin/sigit`，ELF machine 62，SHA-256 `a748a6dc1acbe873f116f2d1837b95f5ca19c44bede012f04ae69f88269e459a` | `sigit-1.7.10-model-source-v5/sigit --acp`，本机 `x86_64-unknown-linux-gnu` 源码构建 |
| GNU aarch64 | `@smbcloud/sigit-linux-arm64@1.7.10` 的 `package/bin/sigit`，ELF machine 183，SHA-256 `4f013ef2f7df26a68fede65b25d86a1e63cd6a0dd3af553c483cec2cde16457f` | `sigit-1.7.10-model-source-v5/sigit --acp`，本机 `aarch64-unknown-linux-gnu` 源码构建 |

- 两个 npm 平台包的下载字节均匹配已发布的 SHA-512 SRI，内部 ELF 摘要与 GitHub 同架构资产匹配。上游发布脚本使用 manylinux2014，校验 glibc 2.17。官方 npm 包及 ELF 仅作对照。原版会在审批前启动 stdio 且继承模型中继环境，文件及终端回调也有本地回退，由受管源构建保留宿主边界。[上游发布](https://github.com/getsigit/sigit/releases/tag/v1.7.10) · [x64 npm 元数据](https://registry.npmjs.org/@smbcloud%2fsigit-linux-x64/1.7.10) · [ARM64 npm 元数据](https://registry.npmjs.org/@smbcloud%2fsigit-linux-arm64/1.7.10)
- 安装输入为 `patches/acp-model-source/sigit/linux.json`。两个目标共享固定源码和补丁摘要。现有安装器核验归档并安全解包，验证后先应用现有 `host-boundary.patch`，再用 `patch --batch --forward --fuzz=0 -p1` 应用 `linux-skills.patch`。沿用 `python3 scripts/cargo_slot.py` 持有槽位的安装入口，在目标架构上构建上游 `cargo build --locked -j 1 -p sigit --bin sigit`，复用槽位的两个 Cargo 缓存路径，选取本机 GNU 编译产物。沿用上游 `rust-toolchain.toml` 的 stable 工具链及锁文件。需要 Python 3、patch、Rust/C 编译工具、CMake、Perl（上游发布使用 perl-core）和 glibc 构建环境。上游发行包的 glibc 下限不适用于尚未构建的受管产物。
- 任务 48 在 `acp-agent-registry::managed_runtime` 的合同选择、`installed_directory`、`install`、`verify`、`reviewed_agent_command` 与 `verify_reviewed_agent_executable` 一致使用 Linux v5。安装后的 `build-receipt.json` 保留 `version`、`source_sha256`、`patch_sha256`（v4 基础补丁）及构建产出的 `sha256`，增加 `linux_patch_sha256` 和 `target`（Rust 目标三元组）。加载和显式路径都要核对对应收据、实际摘要、ELF 架构和固定参数，拒绝未叠加补丁的旧收据。Apple Silicon 沿用 v4 合同。普通 `cli_local` 保留原生账号、模型选路和受管入口验证。
- `model_source::prepare_profile`、`drivers::source_protocol` 和 `drivers::sigit::configure` 现有 Unix 路径支持私有回环 Chat Completions、显式选模和私有状态目录。任务 48 从 `model_source/drivers/mod.rs::capabilities` 对 Linux v5 收据建能力判定，依据目标运行证据决定对外发布。技能接线复用 `request.rs::do_stream_request_acp` 的 siGit profile 分支、`skill_mounts::sigit::configure` 与 `skill_mounts/mod.rs::validate`，保留现有挂载前 `session_model`、工具启用和 `DenyAll` 检查。Linux 扫描复用原生目录优先级、受信租约链接及有界元数据读取，逐级 `openat` 拒绝符号链接。激活、附件和原生文件/终端工具仍使用宿主回调及单次审批。撤除与恢复需要用现有会话租约重验路径。
- 双架构共同阻塞在 MCP。v4 受管 stdio 和 SQLite 辅助文件准备只用 `/usr/bin/sandbox-exec`。现有 `Host::prepare_stdio_mcp_launch` 实现的隔离服务启动也硬编码该 macOS 沙箱，且没有 siGit 分支。通用 Bubblewrap 后端不提供该合同所需的等价路径、子进程限制、逐服务名单或仅允许 SQLite 辅助文件写入的准备路径。上游原生 MCP 分发也没有受审宿主执行入口。保持 `acp-client::client::protocol::validate_mcp_contract` 及供应商沙箱门控，HTTP/SSE 继续拒绝。需补经审查的 Linux 隔离启动、只读索引准备及名单投影与消费后，才能验证原生/外挂 MCP、错误续轮、取消、撤权和恢复。
- 任务 48 在 `acceptance_support/mod.rs` 注册 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_sigit_tests;`。测试通过现有生产网关探针，验证上述受管版本、入口、构建目标和实际模型协议，要求原生/外挂技能分别激活并回传附件，单次允许使真实 stdio 夹具执行一次，拒绝及撤除零新增审计。现有路径先遇 Linux 安装/能力门控，旧 v4 的 Linux 扫描也返回空列表。启用独立技能路径后，组合用例仍会在 MCP 挂载门控失败。夹具的撤除轮是新会话，尚不覆盖同会话的 load/resume 与取消。
- Linux x64 与 ARM64 分别留待授权后执行。传入当前 Linux 网关 `SHIJU_ACP_SIGIT_GATEWAY_BIN` 和私有 v5 安装目录 `ACP_SIGIT_REVIEWED_DIR`，运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_sigit_contract -- --ignored --nocapture`，预期命中一项。已核对固定归档及两层补丁摘要并静态重放，检查了新 Rust 文件格式、Python 语法及定向差异。未执行 Cargo、Docker、运行时安装或模型请求，两种架构均无红绿结果。

## 续查与 Linux v6 投影候选

- 两个 GNU 架构的受管候选统一升级为 `sigit-1.7.10-model-source-v6/sigit --acp`。其余制品和构建参数沿用上节。任务 48 在基础及技能补丁之后叠加 `linux-mcp.patch`，安装与身份校验额外核对清单及收据的 `linux_mcp_patch_sha256`。收据必须覆盖全部三层补丁；v5 不能进入投影分支。Darwin 仍只装 v4。普通 `cli_local` 无托管启动标记。
- v6 的供应商入口在 new/load/resume/fork 分支读取本次选集，仅接受与 `OPENAI_BASE_URL` 同源的 `http://127.0.0.1:<端口>/shiju-vtcode-mcp/<服务名>`，以及与模型凭据分离的宿主 `RelayBinding` bearer。缺少标记、错误来源、原始 stdio、SSE、外部 HTTP 和重复目录身份均拒绝。整批定义先校验，随后读取宿主已过滤的 JSON-RPC `tools/list`。网络客户端关闭代理、重定向和重试。投影工具审批标题为精确的 `mcp__<服务名>__<工具名>`，原始 JSON 参数保留在 `raw_input`，仅接受单次允许。调用携带 `params._meta["shiju.sigit.toolCallId"]` 原 ACP ID。错误回到原工具续轮且无本地回退。取消中止供应商 HTTP 等待，下游终止与撤权仍由宿主租约负责。
- 任务 48 可复用 `RuntimeHost::prepare_stdio_mcp_launch`、`mimo_mcp::Projection` 的 JSON-RPC 路由与 `managed_mcp` 授权入口。新增 siGit 精确注册分支，用 `shiju.sigit.toolPolicy` 投影选中服务的 deny/allow，且下游保留原生服务名。先运行 `drivers::sigit::configure` 清除继承的 `SIGIT_MANAGED_MCP_TOKEN`，只有宿主成功建立本轮投影后才覆盖 `drivers::sigit::MANAGED_MCP_TOKEN_ENV`。在 `acp-client::client::protocol::validate_mcp_contract` 只接受经验证的 Linux v6 与宿主生成的同源投影。新会话和恢复由宿主重新解析选集及签发令牌；失败撤销旧接线和令牌。
- 两架构的外挂 MCP 已具备上述供应商半边，仍缺共享 Linux 隔离 worker 和同调用 ID 的宿主授权消费。已查 `mimo_mcp::prepare` 仍限 Darwin，`dispatch` 仅按名称、参数及租约消费批准。直接沿用会丢失调用身份。需要共享层提供具备只读工作区、敏感文件排除、无网络、子进程约束和中断清理的 Linux 服务启动，以及安全 SQLite sidecar 准备。通用 Bubblewrap 暴露宿主临时目录且未限制派生进程，Landlock 的逐命令包装返回 Unsupported，现有通用 Docker 后端也未实现该启动和授权合同。项目没有可直接接入的等价 worker；各 key 不能通过放宽门控填补该依赖。
- 两架构的供应商原生静态 MCP 独立阻塞。固定版内建官方 HTTP 路径使用供应商账号凭据，内建 smb/asc 走本地原生进程，均在受管 ACP 启动时关闭。现有供应商协议未提供受管会话内的原生服务隔离、宿主凭据交接与撤权合同。需要供应商提供该协议，或者由宿主引入经审查的静态服务迁移及独立凭据来源。当前安全候选仅覆盖会话外挂投影客户端。继续阻塞 MCP 组合并保留现有门控。没有 MCP 的选模、原生工具及技能候选仍可按上节接线并待双架构回归。
- 复用现有生产网关测试入口，将 `ACP_SIGIT_REVIEWED_DIR` 换成私有 v6 目录，验收夹具校验三层收据并识别 Linux 的单次权限标题。现场仍只执行补丁重放、源码/摘要以及目标差异静态检查，未执行 Cargo、Docker、安装或网关测试。

- v6 投影候选提交为 `fff053ed1`。
