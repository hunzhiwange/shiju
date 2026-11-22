# 任务 39 适配 poolside

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 poolside 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/poolside.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/poolside.rs`、`patches/acp-model-source/poolside`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_poolside_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/poolside.md`、`docs/dev/acp/skill-coexistence/poolside.md`。

## 上下文

审计基线版本为 1.0.16。MCP 现状｜旧受管拒列表；原生 new 接收但未进目录，启动期 HTTP 诊断完整。模型来源现状｜原版私有配置可选模；受审、Docker 必需；通用；工作区普通入口变化见上。原生工具现状｜普通与挂载均保留 read/write，固定禁 shell 等；宿主审查暂存写，拒直接写；挂载历史 33 次 MCP 及 15 次读取。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 核实镜像架构和隔离后的宿主挂载。保留原有写入暂存及审查，按 Linux 容器内路径核对工作区绑定。验证夹具覆盖容器调用边界。后续如需真实容器集成，使用隔离的 Docker 环境。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_poolside_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_poolside_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 归档成员与静态证据 | 运行 |
| --- | --- | --- |
| x86_64 | `pool-linux-amd64`，ELF64 x86-64，Docker OCI `linux/amd64` | 未运行 |
| aarch64 | `pool-linux-arm64`，ELF64 AArch64，Docker OCI `linux/arm64/v8` | 未运行 |

- 两份上游归档的实际 SHA-256 与 ACP Registry 相符。每份只有一个普通可执行文件成员，ELF 无动态解释器、`NEEDED` 或 glibc 版本引用。候选安装素材 `patches/acp-model-source/poolside/linux.json` 固定 1.0.16 的归档 URL、成员、原生二进制摘要及 Docker 索引和两个子清单。通过 Docker Hub 镜像缓存回读该索引，响应 SHA-256 与引用一致。没有运行发行包或镜像。[供应商 1.0.16](https://github.com/poolsideai/pool/releases/tag/v1.0.16) · [ACP Registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
- **任务 48 安装接线。** 保留稳定 key `poolside` 和现有系统原生 `pool acp` 的 `cli_local`，Linux 安装不得覆盖原生命令或账户。待 Linux 隔离层受审后，将 `managed_runtime.rs::contract`、`installed_directory`、`reviewed_agent_command`、`verify_reviewed_agent_executable`、`managed_runtime/install.py` 的 Poolside 分支和 `config-acp-install/src/lib.rs` 的安装可用性统一接至按 GNU 架构选择的 v4 合约。依赖 Python 3、HTTPS 下载及可访问且能解析同路径 bind 源的本机 Docker daemon。HTTPS 下载后核对归档摘要，仅复制唯一预期成员至 `pool-native`，启动器名 `pool`，两文件模式 0700，核对全部已安装文件摘要。宿主参数仅 `acp`，受管启动器再调用 `pool-native acp --sandbox required`。v4 尚缺 Linux 启动器及 stdio 隔离实现的摘要。Apple Silicon v3 的合约和 Python 源码保持原样。
- **任务 48 运行接线。** 继续使用 `model_source::drivers::poolside::configure` 的固定 argv、私有回环、工具模型与准确选模检查，以及 `drivers::source_protocol` 的 Chat Completions 路由。受管启动同时经过 `drivers::capabilities`、`skill_mounts::validate`、`request::do_stream_request_acp` 的 Poolside 分支和 `acp-client::client::process` 的私有启动环境。Linux 当前由共享门控和本次独占 driver 拒绝 `session_model`，解决隔离阻塞前保留两者。即使将来受管 `session_model` 可用，Linux 的普通 `cli_local` 仍应选择独立的操作员原生入口。
- **隔离及授权阻塞。** `launcher.py::sandboxed_engine` 用 Darwin Seatbelt 约束宿主侧技能激活和项目链接，`mcp_bridge.py::Mounts` 加载的 `shiju-stdio.py::Stdio.start` 也固定使用 `sandbox-exec`。Docker 仅运行文件工具，无法隔离宿主发现或 stdio 子进程。现有通用 Bubblewrap 隔离宿主网络，无法直接连接模型中继及会话 HTTP 桥，且缺少 Poolside 专属文件范围、工作目录、环境与 exec 限制。现有通用 Landlock 不约束所需网络边界。需 Linux 等价宿主隔离，以及限制 stdio 读取、写入、进程和网络的执行器，之后才可替换 guard。继续复用 `Mounts.permission/decide/claim/end/revoke`、`shiju.poolside.toolPolicy` 和宿主 `DenyAll`。保留工作区与临时技能目录的只读 Docker bind、暂存 diff 审批、拒绝原生直接写。容器不接收 Docker socket 或私有模型环境。旧受管进程的挂载只能用于当前会话，挂载变化时应先废弃旧进程和批准，再按当前选择重建，不能重用旧桥。
- **尚缺路径。** 受管配置目前只写当前会话的 MCP，生产启动禁止项目 `.poolside` 配置覆盖。原版接受 ACP `mcpServers` 也不会使工具进目录。两架构均缺能保留原生 MCP 同时执行归属核验、逐次审批和恢复撤权的受审入口。真实原生写入批准/拒绝、同会话取消和恢复也仍需 Linux 证据。双架构制品和 OCI 支持不能代替这些运行证据。
- **任务 48 测试注册。** 在 `acceptance_support/mod.rs` 增加 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_poolside_tests;`。夹具借真实网关发四个会话，核对选中的二进制摘要及模型协议、原生和外挂技能正文及附件、只读同路径容器绑定、批准后服务实际执行及同 ID 续轮，拒绝和撤除后服务审计均零新增。旧 Linux 会在已核验安装的网关路径上被技能或模型门控拦截。该验收覆盖会话间撤除，未覆盖同会话恢复、原生 MCP 或宿主写入审批。
- 后续在每种架构的非 root 隔离容器中设 `SHIJU_ACP_POOLSIDE_GATEWAY_BIN` 为网关绝对路径、`ACP_POOLSIDE_REVIEWED_DIR` 为该架构私有 v4 安装目录、`ACP_POOLSIDE_DOCKER_SOCKET` 为隔离 daemon 的绝对 Unix socket。daemon 需见到夹具的同一绝对路径。运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_poolside_contract -- --ignored`，预期命中 1 项。显式忽略避免普通测试依赖网关和 Docker。原定命令不带 `--ignored` 时不执行该探针。
- 提交 `c0638a3a7`。已静态核对归档、ELF、镜像索引及验证入口。Python AST、JSON、Rust 语法和定向 Git 差异检查通过。未安装运行时，未执行 Cargo、夹具、Docker 容器或真实模型。未采集红灯或绿灯的退出码和测试命中数。

## 续查

- **两架构普通 `cli_local` 待回归。** 任务 48 已从 `managed_runtime/linux_cli.rs` 消费本任务的制品清单，走 `install_builtin_agent`、`resolve_agent_spec` 和 `AcpClient::spawn_child_at`，固定私有目录 `poolside-1.0.16-linux-cli-v1/pool` 与参数 `acp`。安装不需要账户或模型设置，沿用原版账户路径。新增验收用独立 HOME 和配置根调用安装、发现、真实 ACP 建会话及新进程重连。它验证同一固定入口带受管标记会拒绝，替换已装入口后的重连也会拒绝。现有共享注册位于 `acceptance_support/linux_platform_tests.rs`，无需重复注册。
- **两架构受管路径仍已阻塞。** MiMo 的宿主逐次审批投影可复用调用身份、消费授权和退出处理，但其 `mimo_mcp::prepare` 平台断言和 `Stdio` 启动仍依赖 Darwin；`open_isolated_stdio_peer` 仅给已隔离的命令提供 MCP IO。当前 Landlock 后端未实现子进程包装，通用 Bubblewrap 的共享临时目录、缺少专用 stdio 执行约束和断开宿主回环均不符合合同。给 Bubblewrap 分享宿主网络会扩大访问范围；将整个 ACP 引擎放入 Docker 并挂宿主 socket 会透出 daemon 权限，且不能限制嵌套容器请求。需提供受审的 Linux 原生进程边界（同路径工作区、私有状态、指定回环出口和限范围 Docker API）及独立无网络 stdio 执行边界，或供应商提供等价隔离入口。仓库现有桥无等价实现。
- 原生 MCP 仍需要上游提供可拦截的注册及执行入口，或可审计源码实现等价桥。固定版只证明配置可添加原生服务，`session/new` 的列表不会进入模型工具目录，现有许可回调只证明私有 HTTP 桥内显式下发的服务可逐次审批。直接透传供应商账户或项目配置无法保证原生 stdio/HTTP 和恢复后的调用先经宿主核验。保留模型、原生/外挂工具与技能、审批的受管门控，不把普通入口当作受管回归证据。
- 补充 Python 桥接断言。外部 ACP 的完成 diff 只能产生一次宿主写入请求，允许和拒绝均丢弃原生暂存副本，拒绝回告错误。运行时代码及 Darwin v3 摘要未改。该夹具覆盖桥接协议，真实 Linux 宿主写入和同会话撤权仍未验证。
- 后续每架构执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_poolside_contract_cli_local -- --ignored`，预计命中一项普通入口。受管探针使用上节环境与命令并在 `--ignored` 后附加 `--skip acp_platform_poolside_contract_cli_local`，预计命中一项。桥接检查使用 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s patches/acp-model-source/poolside -p launcher_test.py`。本轮仅静态检查，三条命令均未执行。
- 续查提交 `49ce934e5`。Python AST、定向 Rust 格式与 Git 空白检查通过。没有运行安装、测试或 Docker。
