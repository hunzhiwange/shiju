# 任务 30 适配 kilocode

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 kilocode 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/kilocode.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/kilocode.rs`、`patches/acp-model-source/kilocode`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_kilocode_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/kilocode.md`、`docs/dev/acp/skill-coexistence/kilocode.md`。

## 上下文

审计基线版本为 7.8.18。MCP 现状｜受管沿原生 stdio 完成同 ID 续轮；原版同版尚缺对照。模型来源现状｜原生可选模；受审 --pure；通用，Windows 私有状态 ACL 拒绝。原生工具现状｜补丁保留原生工具和权限；三项范围来自转发器，缺挂载同轮官方执行回执。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（仅私有 Skill 根触发技能扫描限制及重签制品），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_kilocode_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_kilocode_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| Linux GNU 目标 | 固定来源与入口 | 补丁后 ELF SHA-256 |
| --- | --- | --- |
| x86_64 | [@kilocode/cli-linux-x64-baseline@7.8.18](https://registry.npmjs.org/@kilocode/cli-linux-x64-baseline/-/cli-linux-x64-baseline-7.8.18.tgz)，`bin/kilo`，ELF machine 62，加载器 `/lib64/ld-linux-x86-64.so.2` | `3c37b0e6a23c9ae24d8d78a6a7b25f62fce2a4c9a688e0391b8cdf69a451ad67` |
| aarch64 | [@kilocode/cli-linux-arm64@7.8.18](https://registry.npmjs.org/@kilocode/cli-linux-arm64/-/cli-linux-arm64-7.8.18.tgz)，`bin/kilo`，ELF machine 183，加载器 `/lib/ld-linux-aarch64.so.1` | `b511073bf0706e06ed93c50e02d9cd13bf0c46743f13e47d79a20bee70800da3` |

- x86_64 按上游包装器的无 AVX2 选择固定 baseline 包。此版本实际核对 baseline 与普通 x64 的 ELF 字节相同，npm 归档摘要不同。两架构入口可见的最高 GLIBC 符号版本为 2.30。归档 SRI、原始及修改后 ELF 和嵌入入口摘要位于 `patches/acp-model-source/kilocode/linux.json`。补丁复用 Darwin 四处语义修改，按架构选择精确锚点。在 Linux ELF 现有填充内各增长 2374 字节，保留 93 个非入口模块、原生指令及 program headers。仅受管环境下追加私有技能根和省略内置技能。选中来源使用 canonical skill 目录的未信任解析，原生同名优先，受限附件目录读取继续使用 Bun glob。Apple Silicon 仍使用原来的重签修订。
- 任务 48 安装输入。Linux 限 `target_env=gnu` 的 x86_64/aarch64。继续采用稳定 key `kilocode`，使用分离目录 `kilocode-7.8.18-model-source-linux-v3`，包元数据为 `@kilocode/cli@7.8.18`。在 `managed_runtime.rs::install` 暂存 `kilocode/{apply_linux.py,linux.json}`。在 `managed_runtime/install.py` 的 Kilocode 分支，将所选 GNU 包与包装包均固定为 7.8.18。npm 路径显式选定目标包和包装包，使用 `--ignore-scripts --save-exact`，核对 lock 里的两层 SRI。也可复用固定归档下载并核对 SRI。禁止包装器的安装后脚本及动态架构回退。用 Python 3 执行 `apply_linux.py <x64|arm64> <platform-package>/bin/kilo <wrapper-package>/reviewed/kilocode`。如果后续隔离设计使用上游 bwrap、seccomp、网络 relay 或其他外置资源，须固定其身份并按实际加载位置提供。暂以审查后的 Linux 锁条件为准，安装器尚未接线或运行。
- 任务 48 注册输入。候选入口是版本目录内的 `node_modules/@kilocode/cli/reviewed/kilocode acp --pure`。在 `installed_directory`、`reviewed_agent_command` 和 `verify_reviewed_agent_executable` 一致匹配 Linux 修订、包版本和目标摘要，拒绝 Darwin 摘要及普通 CLI 回退。既有 `drivers::kilocode::configure` 可复用 GNU 私有状态、管理员 `/etc/kilo` 配置拒绝、每会话模型别名、回环 Chat Completions 和令牌轮换。普通 `cli_local` 继续使用现有本机发现和 `npx -y @kilocode/cli acp` 回退。Linux 受限启动门控仍位于 `request.rs` 和 `acp-client/src/client/kilocode_sandbox.rs`，状态保持拒绝。
- 两架构共同阻塞。Darwin 的 `kilocode/launcher.py` 依赖 Seatbelt，限制工作区和选中源的读取、私有状态写入、固定 Git 及供应商入口的执行、两个回环端口。OMP bridge 可复用提示生命周期、动态挂载、按策略发现和逐次宿主审批，但它的下游 `cortex-code/mcp_bridge.py::Stdio.start` 仍硬编码 `/usr/bin/sandbox-exec`。通用 Bubblewrap 包装器隔离整个网络，无法保持拥有的回环中继，也没有该模型进程的执行白名单和挂载合同。上游 Kilo SandboxPolicy 包裹已启用的工具调用，内部工具可以走供应商自己的提权批准，并未隔离整个 ACP 进程或绑定到宿主的一次性批准。受限启动需同时补等价的 Linux 供应商进程和下游 stdio 隔离，且保持宿主回环中继。禁止以直通供应商 stdio 代替桥。上游 ACP 的 `registerMcpServers` 只增不删，宿主需要保留桥在 load/resume 时的重建和撤销机制，核对旧端点和令牌不再有效。[固定版 ACP 服务](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/acp/service.ts) · [固定版工具隔离策略](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/opencode/src/kilocode/sandbox/policy.ts)
- 原生配置 MCP 仍是独立缺口。现有受限启动禁用项目配置并使用私有 HOME；现有网关夹具验证原生 read/skill 和宿主选择的 MCP，未覆盖独立的原生配置 MCP。恢复其目录前须提供受同等宿主审批、读写及网络约束的路由，再测试两类 MCP 的不同服务审计和同会话 resume/cancel 撤权。现有夹具撤除轮另建会话。
- 任务 48 在 `acceptance_support/mod.rs` 用 `#[cfg(all(test, target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))] mod platform_kilocode_tests;` 注册新契约。在各自非 root GNU Docker 容器准备私有版本布局，并设置 `SHIJU_ACP_KILOCODE_GATEWAY_BIN`。可用 `ACP_COEXISTENCE_KILOCODE_BIN` 指定经过补丁的入口，入口须带同布局的包装包元数据。命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_kilocode_contract -- --ignored`，预计实际命中 1 项。当前生产路径应在旧 Darwin 门控失败。已补网关夹具验证固定摘要、实际 Chat Completions 请求、原生和外挂技能及附件、批准调用后该 ID 续轮、拒绝和撤除零新增下游审计。测试亦要求拒绝后有模型续轮。
- 本轮仅读取上游归档，核对 SHA-512、架构、ELF 和入口摘要，试应用补丁并比对其余模块及非 .bun 字节。Python AST、Node 静态语法、定向 Rust 格式和 Git 空白检查通过。两架构均未运行 Cargo、Docker、npm 安装、网关或供应商程序，无红绿退出码和有效测试数。

- 实际提交为 `8089f61a7`。编号任务卡保留供任务 48 和 49 接线与记账。

## 受限注册复查

- v3 在两个 GNU ELF 的原生 ACP MCP 注册入口增加整批预检。仅 `SHIJU_KILO_RESTRICTED=1` 时要求宿主设置 `SHIJU_KILO_BRIDGE_PORT`，并仅接受该端口上的字面 `127.0.0.1`、`/mcp/<43 字符桥随机路径>`、HTTP transport 和空 headers。任一条错误时整批失败，原生 MCP add 尚未执行。外层 ACP 仍由 OMP bridge 提供调用绑定、一次性批准及撤销。新增 `linux-mcp.test.mjs` 检查两个目标实际嵌入的预检，并覆盖普通路径。任务 48 使用 v3 摘要，且在 bridge 的 `configure(port)` 回调向供应商子进程发布绑定后的实际端口。用户配置及继承环境不得供给该值。
- 两个 GNU 目标分别缺少安全启动条件。DeepAgents 的投影客户端可提供协议参考，其宿主使用的 `mimo_mcp::prepare` 仍拒绝非 Darwin arm64。OMP server 和审批状态可复用，但现有 stdio peer 启动器仍要求 Seatbelt。通用 Bubblewrap 隔离了全网络且未挂载审过的只读工作区及可执行项。要复用其命名空间，需另有经审查的双向回环代理、专用目录与执行约束，以及可用于外置 stdio 子进程的隔离合同。直接使用 host 网络会给供应商进程额外网络权限，直接使用标准 Bubblewrap 会使回环模型和 MCP 失联。Poolside 的容器只包住文件工具，供应商引擎本身仍依赖 Seatbelt。Kilo 内置 sandbox 仅包住启用的工具操作，且其升级批准归上游，不能充当宿主启动边界。
- 固定版内核 MCP service 有 `remove(name)`，但 ACP 注册路径的 SDK 未提供对应动态 remove。桥清除后旧路径无法调用，供应商目录可能继续展示缓存条目。在存在挂载变化的恢复场景，应由真实宿主按新配置重启供应商进程，再从私有状态恢复同一会话并重建桥。原生配置 MCP 缺可信来源分流及逐次宿主审批协议。要解除两架构阻塞，需要可用的 Linux 受限启动与下游隔离入口，以及按来源接入原生配置 MCP 的宿主契约。当前保持平台门控。
- 更新后的两份补丁均可在原始固定 ELF 的原有段填充内应用。验证仅限摘要、非入口模块、JS 静态语法和定向差异检查。后续另行授权可执行 `node --test patches/acp-model-source/kilocode/linux-mcp.test.mjs`，以及前述实际网关定向测试。两者本轮未运行。
- v3 注册预检与测试源码提交为 `9747ce74c`。
