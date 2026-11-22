# 任务 16 适配 corust-agent

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 corust-agent 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/skill_mounts/corust_agent.rs`、`patches/acp-model-source/corust-agent`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_corust_agent_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/corust-agent.md`、`docs/dev/acp/skill-coexistence/corust-agent.md`。

## 上下文

审计基线版本为 0.6.0。MCP 现状｜三种会话传输均未进模型目录，强制调用 ToolNotFoundError。模型来源现状｜原包选模探针卡认证；受审私有 workspace_models/Chat；通用。原生工具现状｜无挂载保留四项中继放行的官方文件工具；挂载替换为图谱，桥只认识图谱调用。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v3 用请求私有图谱快照装配固定三工具、按服务过滤并代执行续轮），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_corust_agent_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_corust_agent_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| Linux GNU 目标 | 制品与入口 | 能力边界 |
| --- | --- | --- |
| x86_64 | 固定 0.6.0，官方 asset 419147802。唯一归档成员 `corust-agent-acp`，普通启动无参数。归档及 ELF 摘要见 [清单](../../patches/acp-model-source/corust-agent/linux-x64.json) | 可接普通 `cli_local` 安装及发现，尚无启动实测。受管模型、MCP、技能和审批保持关闭 |
| aarch64 | 0.6.0 官方资产列有 Darwin 两架构、Linux x64、Windows x64，未列 Linux ARM64。该 tag 的公开发行树只有工作流、许可、扩展清单和图标，无可移植程序源码 | 固定安装与受管能力均阻塞。需上游可验证 Linux ARM64 制品或对应可审源码 |

- 上游证据为 [0.6.0 发布](https://github.com/rustingai/corust-agent-release/releases/tag/v0.6.0)、[发布资产 API](https://api.github.com/repos/rustingai/corust-agent-release/releases/tags/v0.6.0) 和 [tag 树](https://api.github.com/repos/rustingai/corust-agent-release/git/trees/v0.6.0?recursive=1)。静态读取官方 x64 归档，核对归档 11,540,116 字节及 SHA-256、唯一普通文件、原始 0755、文件摘要及 ELF 机器类型。入口要求 `/lib64/ld-linux-x86-64.so.2`、glibc 2.39 及 `libgcc_s.so.1`，动态依赖另含 `libc.so.6`、`libm.so.6`。仅检查制品字节，未安装或执行。
- 任务 48 安装在 `acp_agent_registry::managed_runtime` 的 `corust-agent` 安装路径仅按 Linux GNU x86_64 选择 `corust-agent-0.6.0-linux-x64-native` 独立目录。将 `linux-x64.json` 与 `install_linux.py` 嵌入并写到同一临时素材目录。复用 Corust 现行 `/usr/bin/curl --disable --fail --silent --show-error --location --proto =https --proto-redir =https --retry 2 --max-time 60 --header 'Accept: application/octet-stream' --output <临时归档> <清单 asset URL>`，再调用 `install_native(archive, staging)`。需要 Python 3、curl 和上述 GNU 运行依赖。安装器在发布前校验归档和入口摘要，专用暂存目录及入口为 0700。由现有安装流程发布，保留非 Linux 安装路径。
- 任务 48 在 `config-acp-install/src/lib.rs` 的受管安装选择分支加入 x86_64 GNU 条件。普通发现由 `agent_registry.rs::built_in_agent_command` 优先选上述目录中固定绝对入口，参数和额外环境为空。发现时查验独立目录与入口没有路径重定向、入口为可执行普通文件并匹配清单 SHA-256。保留现有按 PATH 查找的普通 `corust-agent-acp`。原包入口只接普通 `cli_local`，不要纳入 `verify_reviewed_agent_executable` 的受管模型允许列表。Linux ARM64 保留现有普通发现，安装须报告缺制品。
- 受管断点。上游 0.6.0 的会话 stdio、HTTP、SSE 经现有审计均未进入模型工具目录。现有 v4 的 Chat 中继、`workspace_models`、四个原生文件工具与三图谱工具合批续轮、会话服务过滤、请求私有图谱快照和单次 ACP 审批依赖 `launcher.py::native_command` 与 `mcp_graph.py::Graph.catalog` 的两套 Seatbelt 边界。Linux 现有通用 Bubblewrap 包装直接隔离网络，无法提供只允许指定宿主回环中继的原生进程网络；Landlock 包装亦未在子进程 pre-exec 强制执行。直接运行会失去凭据、工作区写入、执行和网络限制。另外，`mcp_bridge.py::invoke` 只消费 ACP 应答，未接任务 4 按调用身份及请求租约的宿主执行前消费。普通入口不能承接会话模型或 MCP 请求。Linux 驱动有显式拒绝，技能挂载继续由 `skill_mounts/mod.rs::validate` 门控。启用前需 Linux 等价隔离、受管审批消费及撤权联动、原生操作授权审查，以及非图谱 MCP（含远端传输）和供应商原生 MCP 的实现证据。Apple Silicon 仍复用原 v4。
- 任务 48 在 `acceptance_support/mod.rs` 按 `all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64"))` 注册 `platform_corust_agent_tests`。当前 x64 红灯探针使用固定原包覆盖项，通过网关真实会话入口遇到平台或受管模型门控；ARM64 在制品阶段明确失败。后续改由已审 Linux 受管入口跑绿灯，记录所选原生文件摘要及受管修订。现有模型探针要求实际 `POST /v1/chat/completions`、指定模型、四项原生目录、原生及选中技能正文和附件、MCP 同调用 ID 续轮、批准 1 次、拒绝或撤除 0 次。计数目前来自桥的配对完成事件，撤除场景是新会话。还需服务侧独立调度审计和同会话撤除、恢复、取消覆盖。
- 本轮通过 Python 语法、JSON、独立 Rust 测试源码格式检查及限定差异检查。未运行 Cargo、Docker 或上游程序。获准且接线后，在非 root Linux Docker 中提供 `SHIJU_ACP_CORUST_GATEWAY_BIN`、`ACP_CORUST_LINUX_ARCHIVE`、`ACP_CORUST_GRAPH`，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_corust_agent_contract -- --ignored`。当前无退出码或测试数。任务 49 逐架构登记上述阻塞。
- 独占实现及验收源码已提交 `6c23cfcac`。本任务卡保留供任务 48、49 接线与登记。

## 共享接线后的复核

| 目标与运行方式 | 状态 | 当前证据或所需输入 |
| --- | --- | --- |
| GNU x86_64 普通 cli_local | 待回归 | 正式固定安装、发现和启动校验已接。新增验收调用正式安装器、注册表和 ACP 客户端，在临时工作区检查 initialize 和 session/new。无需真实账号或提示。仅缺 Linux 执行证据 |
| GNU x86_64 受管模型、原生 MCP、会话 MCP、两类技能和审批 | 已阻塞 | 需要受审 Corust 源码或上游协议，使原生操作能逐次走宿主审批，并提供可撤销的模型和 MCP 路径。也可采用满足下述执行边界的等价适配 |
| GNU aarch64 固定安装及受管能力 | 已阻塞 | 需上游实际 ARM64 ACP 制品及可验证身份，或对应可重现构建源码。受管能力还需 x64 同样的前置输入 |

- 任务 48 已接入 `managed_runtime/linux_cli.rs::{contract,install,discover,verify_launch}`、`managed_runtime/install.py::install_linux_cli` 和 `acceptance_support/linux_platform_tests.rs`。正式安装下载超时为 240 秒，Rust 外层为 300 秒。普通网关或桌面启动仍须持有当前全访问授权。向普通入口提供托管引用、技能选择或会话 MCP 都无法进入受管分支。
- 再核对 latest、固定 tag 的 [Zed 扩展目标清单](https://github.com/rustingai/corust-agent-release/blob/v0.6.0/extension.toml) 及发布工作流。Linux ACP 只列 x86_64。工作流另列 OneRouter 下载，与 ACP 制品用途不同。QEMU 用户态运行已有 x64 制品仍需要一整套异架构运行依赖，无法满足本轮 ARM64 GNU 原生制品契约。
- 仅覆盖原包 OneRouter 私有配置会保留原生工具直接访问。仅裁剪 Chat 工具声明无法约束供应商进程自身的文件、网络或执行行为，也无法把上游原生 MCP 和会话 HTTP/SSE 接到宿主逐次授权。现有 `ManagedMcpApproval` 记录 ACP 回调的授权及有效性，供应商直接工具调用缺少执行前消费点。现行图谱桥明确拒绝 `session/load`、`session/resume` 和 `session/fork`，不能靠宿主恢复自动重建图谱授权。
- Linux 通用 Bubblewrap 断网会切断模型中继。保留主机网命名空间会给原生进程开放其他地址和端口。单独按端口限制也不足以绑定目的地址。隔离网命名空间加单一端点转接可进一步评审，但仓库无现成启动入口，该方案也无法补原生工具审批和上游 MCP。现有 Landlock 包装缺子进程执行前强制边界。维持现行 Linux 门控。
- 获准后可单独执行普通子契约 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_corust_agent_native_contract -- --ignored`。在非 root GNU 容器执行。x64 需 glibc 2.39、Python 3、curl 和固定资产访问，预期安装并创建无提示会话。ARM64 预期明确拒绝且无安装目录。各目标预期命中 1 项。测试子进程使用清空后的环境及临时 HOME、配置和工作区。原受管验收仍是阻塞探针，待受管实现后接新的制品。当前未运行 Cargo、Docker、普通子契约或受管探针。
- 普通安装与会话验收源码已提交 `63ab52566`。定向 Rust 格式检查与差异检查通过。
