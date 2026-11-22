# 任务 47 适配 vtcode

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 vtcode 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/vtcode.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/vtcode.rs`、`patches/acp-model-source/vtcode`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_vtcode_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/vtcode.md`、`docs/dev/acp/skill-coexistence/vtcode.md`。

## 上下文

审计基线版本为 0.183.0。MCP 现状｜原版 new 忽略列表，模型缺定义。模型来源现状｜原版 TOML 可选模；空挂载核受审原包，挂载核 v2 相邻制品；通用。原生工具现状｜普通/挂载均保留沙箱支持四项，挂载追加；历史同轮 17 次官方和 22 次 MCP；挂载 DenyAll 拒绝。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 回环 HTTP 桥、服务名单、调用名与参数绑定、单次授权；挂载强制新会话），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_vtcode_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_vtcode_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。


## 交付记录

| Linux GNU 目标 | 0.183.0 原版制品身份 | 受管阻塞 |
| --- | --- | --- |
| x86_64 | 官方 GNU 归档和 `./vtcode` ELF 已核对，SHA-256 见清单。动态依赖有 glibc、libstdc++、libgcc_s，非弱 GLIBC 符号最高 2.38，另有弱 2.39。 | Linux VT Code 与 stdio 子进程隔离缺失，v3 制品尚未构建及固定。 |
| aarch64 | 独立的官方 GNU 归档和 `./vtcode` ELF 已核对，SHA-256 见清单。动态依赖有 glibc、libstdc++、libgcc_s，非弱 GLIBC 符号最高 2.18。 | Linux VT Code 与 stdio 子进程隔离缺失，v3 制品尚未构建及固定。 |

- 原版固定制品定义在 [linux.json](../../patches/acp-model-source/vtcode/linux.json)。两个归档的实际 SHA-256 同时匹配 GitHub release API 与上游同名校验文件。用 ELF 头及版本依赖核对了目标架构和所需 ABI。安装需要 HTTPS 下载、Python 3 的 tar 读取、目标的动态链接器及上述库。普通入口为 `vtcode acp`，只给子进程设置 `VT_ACP_ENABLED=1`、`VT_ACP_ZED_ENABLED=1`。[上游固定发行](https://github.com/vinhnx/VTCode/releases/tag/0.183.0)
- 任务 48 在 Linux GNU x86_64/aarch64 独占条件下为 `managed_runtime.rs` 增加 `#[path = "managed_runtime/vtcode.rs"] mod vtcode;`。在 `agent_registry.rs::built_in_agent_command` 的普通 CLI 优先级中使用 `vtcode::cli_local_command`（从注册表跨模块调用时可经 `managed_runtime` 转发），传入 `root("vtcode")`，发现经校验的绝对入口和 `["acp"]`。保留本机 PATH 发现和现有原生账号环境。普通 Linux 校验入口保持与 `reviewed_agent_command` 和 `verify_reviewed_agent_executable` 独立，以免通过后者的已安装路径相等判断赋予受管能力。
- 任务 48 在显式 `install_builtin_agent` 及 Python 安装素材传递中按目标选取 `vtcode::LINUX_CLI_CONTRACT` 的固定 URL、归档和 ELF 摘要。复用独立暂存、HTTPS、校验后安装及回读。只提取唯一且为普通文件的 `./vtcode`，检查归档与入口摘要后设权限 0700，发布到 `acp-runtime/vtcode-0.183.0-linux-cli-v1/vtcode`。Linux 原版不消费应用同级 `vtcode-acp`。Darwin arm64 按现有 v1/v2/v3 合约发现及安装。
- 固定源码提交为 `4fd8e47a885c1224f122bde8b52f293071240525`，tag 源归档 SHA-256 为 `80c598c0086a2ad4fe4b3ea5cfdade9651cace6dc2542990be982a56dc617831`，`Cargo.lock` 为 `6ba0d9c829a09f0cd88f6f2ccaf966b4d26e2f6a1cadc00d26f64121625bad45`。在临时源码副本按 MCP、只读技能顺序使用 `git apply --check` 并应用，两层均可应用。对应补丁 SHA-256 为 `5f880b25dfb3832ca4881f93602d5489c0c1f5bd3e1776920eaa7028e634b799`、`ab657c6e25a73a940cd54e6b80ef70425b44631c80ccf80871b9cfd266e3fa6b`。后续构建可从两架构原生 GNU 环境复用 Rust 1.100.0、`--locked -p vtcode --bin vtcode` 和相应 GNU 目标，原版哈希不适用于补丁构建。
- 现有模型驱动生成私有 TOML，固定 `/v1/chat/completions`、选定模型和回环令牌。原版的 `session/new` 缺会话 MCP 目录，原生 CLI 能发现技能但 ACP 入口缺只读技能工具。两层补丁可提供同轮调用 ID 回传、原生与外挂技能分源及只读资源。现有宿主 MCP 路线由 `mimo_mcp::prepare` 通过私有回环 HTTP 暴露 stdio 服务，校验服务名单并用 `VtcodeApproval` 按工具名和参数消费当轮的一次授权。 `request.rs` 对挂载强制新会话。原生静态 MCP 仍缺宿主逐次执行与撤除证据。外挂 stdio 后端和 VT Code 本身分别依赖 Darwin 隔离，浏览目录与编辑路径、冻结配置、禁止子进程、限制回环端口及保护技能快照都需要 Linux 等价实现。通用 Bubblewrap 接口不提供这些逐请求约束及端口级隔离，Landlock 的文件约束也无法单独覆盖整个边界。任务 48/49 对两个目标保持 `session_model`、受管工具、外挂 MCP、外挂技能及审批禁用。
- 任务 48 在 `acceptance_support/mod.rs` 以 `cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))` 注册 `platform_vtcode_tests`。断言覆盖普通 CLI 能力、原版制品发现、摘要、实际宿主 ACP 握手和受管拒绝。旧注册路径无法从隔离安装目录选出 Linux ELF，受管路径在构建私有状态前失败。后续获准后分别在非 root Docker 目标执行任务原定命令，准备隔离安装和 `VIBEWINDOW_CONFIG_DIR` 再以同一过滤追加 `-- --include-ignored` 运行握手。受管工具正例仍需 Linux 私有环境和已审摘要。届时可复用 `scripts/acp_skill_coexistence/vtcode.py` 的模型请求、同 ID 续轮、批准、拒绝及卸载服务计数夹具，扩展原生工具和原生 MCP 的审计断言。
- 独占提交 `e91cdd692`。本轮仅完成归档、ELF、补丁应用、JSON、Rust 格式和定向空白检查。未安装或执行供应商二进制，未运行 Cargo 或 Docker。暂无红绿退出码及命中数。

### 续查与追加交付

- 任务 48 已通过 `managed_runtime::linux_cli` 接通双架构固定原版安装与发现，并在普通进程启动复核固定路径和参数。注册侧复用本卡独占验证器，`acceptance_support/linux_platform_tests.rs` 已注册验收。增加固定入口接受 `acp`、拒绝改为 `chat` 的断言。双架构普通 CLI 子集仅待回归。
- 受管候选使用清单 `managed_source_candidate` 的固定源、锁文件、Rust 1.100.0 及三层补丁。安装候选先校验源码和锁文件，第一层 `git apply --unidiff-zero`，后两层普通 `git apply`；经授权后在两种 GNU 目标分别以 `--locked -p vtcode --bin vtcode` 构建、锁定产物并校验目标 ELF 与摘要。第三层只在 Linux 且有宿主 launch reference 时收紧。ACP 只接受单次放行，外挂 MCP URL 必须匹配宿主私有 `SHIJU_VTCODE_MCP_ORIGIN` 下的 `/shiju-vtcode-mcp/<服务>`。MCP 请求以 `params._meta.shijuCallId` 传递与 ACP 相同的调用 ID，保留审批原参数，发送一次原生 `tools/call` 并拒绝需要后续输入的响应。原生结果和调用 ID 继续沿 ACP 回传。Apple Silicon 和普通 CLI 保留原路径。
- 共享投影还不能消费这个 ID。当前 `ManagedMcpApproval` 与 `VtcodeApproval` 只绑定名称、参数和有效期，同名同参调用无法区分。后续共享实现需同时接收 ACP 请求的 `toolCallId` 与执行请求的 `shijuCallId`，按会话、提示、服务、调用 ID、名称及原参数校验后一次消费，转发前剥离专用元数据。只有隔离启动完成后才由宿主注入 origin 和私有配置，禁止从用户配置授权该环境。GNU x86_64/aarch64 均需等价的 VT Code 和独立 stdio 后端隔离。共享 Bubblewrap 会挂载宿主 `/tmp` 并切断宿主回环；Landlock 现有实现不支持逐命令包装；直接 HTTP/stdio 与原生工具策略都不能替代受控进程、私有文件范围和回环约束。上游原版还缺会话 MCP 与 ACP 技能目录。需要独立审定本机补丁制品与共享执行边界，并补原生静态 MCP、同会话取消/撤权及重复同实参不同 ID 的零执行审计。完整受管组合保留已阻塞与对外拒绝。
- 追加提交 `90c353141`。第三层在两层已应用的固定源码上通过正向与反向差异核对，修改后的 Rust 文件经 rustfmt 解析，补丁和锁文件摘要已比对。未运行 Cargo、Docker 或供应商二进制。后续在固定上游源可用 `cargo test -p vtcode-acp managed_permission_accepts_only_the_single_call_option` 定位供应商审批断言，ShiJu 普通入口仍用原定过滤。

### Unix socket 候选与任务 48 接线

- 本节更新上节的共享依赖。任务 4 的 `prepare_linux_stdio` 已对 `LinuxMcpOwner::VtCode` 校验单个显式 stdio 服务的 SHA/argv 和工具名单，并在中继消费匹配调用 ID 的批准。该执行进程可复用任务 2 的禁止 socket 启动。原生静态 MCP 未进入该合同。
- 同一固定源码在三层补丁后再应用清单第四层。0.183.0 锁定的 reqwest 0.12.28 与 rmcp 使用的 reqwest 0.13.4 均有 `ClientBuilder::unix_socket`。补丁将受管 OpenAI Chat 与 Streamable HTTP MCP 导向 `/run/shiju/relay.sock`，禁用代理和重定向，不加依赖。普通 CLI 和 Darwin 不走该分支。只为 Linux 受管模式保留供应商进程内的 `list_files` 与 `apply_patch` 候选并保留逐次批准。定义和执行都拒绝其他原生工具。 `read_file` 依赖 ACP 宿主回调，现有受管分支拒绝该回调；`code_search` 与原生命令保留关闭，待成本更低且可审的独立实现。原生和外挂只读技能走第三层之前的原有技能视图。new/load/resume 的 cwd 必须为固定工作区，new 禁止额外挂入目录，挂载仍须新会话。
- 任务 48 的中继接线。 `model-relay-http::RelayServer::start_with_routes_and_vtcode_catalog` 当前仍只绑定 TCP。抽用现有 router，以锁定 Axum 0.8.9 的 `axum::serve(UnixListener, router)` 在会话独占 0700 目录中绑定 0600 socket，仅给该主进程单文件挂载。保持现有模型/MCP 两套会话令牌、catalog、调用 ID 与生命周期，撤销时终止服务并清理 socket。逻辑 origin 可固定为 `http://127.0.0.1:1`（不监听 TCP），给 `Projection::approval_config` 同一 origin，TOML 使用该 origin 的 `/v1` 并由宿主设置 `SHIJU_VTCODE_MCP_ORIGIN`。为 `vtcode::configure` 和 `skill_mounts::vtcode::configure` 增加 Linux 的私有配置/技能挂载分支，避免生成 Seatbelt 文件。 `acp-client::spawn_child_at` 只用确定的 Linux 主进程隔离入口；验明固定 ELF 和参数后启动，不回退 Darwin 或原版 CLI。
- 主进程仍有独立隔离缺口。任务 2 紧凑策略禁止 `socket` 和 `socketpair`，不挂工作区，也没有 VT Code ELF 需要的 libstdc++。可复用固定 ELF、最小动态库视图、干净环境和进程回收。主进程需要新合同允许 AF_UNIX stream、禁止其他 socket 族和子命名空间，隔离任何可达宿主套接字。绑定实时宿主工作区会暴露其中现存或并发创建的 pathname socket；仅启动扫描或只读挂载不能阻断连接。一个安全候选是仅复制常规文件和目录到请求私有视图，在独立边界经审批提交写入，并把配置、技能快照、库及唯一 socket 分开只读挂载。另一候选是追加经过审查的、仅向指定 socket 建连的宿主描述符代理，保持严格进程禁止创建 socket，但两个 reqwest 客户端还没有继承描述符的连接器。还需限制工作区或 scratch 内新载入其他可执行文件、隐藏原生项目配置/凭据，并证明 apply_patch 审批写入及撤销。保持对外拒绝，GNU x86_64/aarch64 均缺可审主进程启动、受管制品摘要和目标回执。
- 提交 `d4165755b`。新增真实 UnixListener 测试源码，验证模型 POST 经 socket 到达并拒绝重定向。授权后在固定上游源内按目标运行 `cargo +1.100.0 test --locked -j 1 -p vtcode-llm shiju_relay_uses_unix_socket_and_refuses_redirect`。后续宿主验收还须覆盖无 AF_INET/AF_INET6、挂载目录中其他宿主 socket 不可达、模型与 MCP 共用私有 socket、拒绝与撤销后服务调用为零。第四层已在固定三层源码上验证应用、反向检查及 Rust 解析。未构建或运行测试。

### 私有 TOML 令牌输入与任务 48 接线

- 0.183.0 的 `vtcode-config::core::CustomProviderConfig` 公开格式只有 `api_key_env` 和互斥的命令型 `auth`。`agent.custom_api_keys` 走通用迁移/凭据存储，所以不能承接会话 bearer。第五层补丁只在 Linux 宿主管理标记存在时启用，不改原版或 Darwin 格式。
- 任务 48 将 `# shiju-relay-bearer = <64 位小写十六进制令牌>\n` 作为请求 TOML 的第一行，再拼接现有配置。令牌取当前模型 `RelayBinding::bearer_token()`，与 MCP 投影令牌保持独立。Linux 删除子进程 `SHIJU_VTCODE_RELAY_TOKEN` 及 Linux TOML 的 `api_key_env`；请勿将令牌加到参数、环境或启动日志。按任务 2 使用只读的 `/run/shiju/vtcode.toml`，启动固定 argv `--config /run/shiju/vtcode.toml acp`，继续用宿主设置的 launch reference。路径和格式固定，空缺、格式错、携带不同模型或供应商时失败关闭。
- 普通 TOML 解析入口 `ConfigManager::parse_layer_value` 先剥离该行，避免解析错误的输入源码片段包含令牌。受管 `build_session_provider` 在通用凭据和供应商工厂之前按固定模型创建专用 Chat 客户端。客户端只读取固定文件的首行，输入限 128 字节，转成敏感 Authorization HeaderValue，网络目的地始终是唯一 Unix socket 与固定的 `http://127.0.0.1:1/v1` 逻辑地址。禁用代理和重定向。令牌未存入供应商 api_key、通用云设置或命令凭据。MCP 授权沿请求头与单次调用 ID 的独立路径。
- 提交 `04f073107`。上游验收过滤为 `cargo +1.100.0 test --locked -j 1 -p vtcode-llm shiju_relay_`。夹具只用合成值，覆盖授权头经过真实 UnixListener，敏感 Debug 隐去值、拒绝重定向、缺失或非法输入拒绝。五层补丁、固定源码和锁文件摘要见清单。本轮未编译或执行测试，公开门控保持。Linux 双架构制品、运行时校验和完整权限证据仍待取得。
