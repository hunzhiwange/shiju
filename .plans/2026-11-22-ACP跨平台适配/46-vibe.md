# 任务 46 适配 vibe

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 vibe 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/vibe.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/vibe.rs`、`crates/libs/acp-access-policy/src/access_policy/vibe.rs`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_vibe_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/vibe.md`、`docs/dev/acp/skill-coexistence/vibe.md`。

## 上下文

审计基线版本为 3.0.4。MCP 现状｜v1 原生搜索可发现、执行及回传，同轮有官方搜索。模型来源现状｜原版有原生选模；指定模型受审 v2；通用。原生工具现状｜v2 无挂载、挂载、无名单及选中 v2 的 cli_local 都受影响，连带子代理和后台能力。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 保留原生工具发现路径，针对私有挂载追加目录。真实回执确认同轮两类调用与撤除。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_vibe_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_vibe_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

| GNU 目标 | 上游 3.0.4 制品 | 已核对证据 |
| --- | --- | --- |
| x86_64 | `cp312-abi3-manylinux_2_28_x86_64.whl`，SHA-256 `fa201f53a8f7574435c1b911218a4a537e3678831f9cbd7791e5fb32658de956` | wheel 字节、ELF x86-64、补丁前后摘要，未安装或运行 |
| aarch64 | `cp312-abi3-manylinux_2_28_aarch64.whl`，SHA-256 `bdd52e926ebdb56caabab226987873574ec481bc1564b7fd911222cacb7b3d9d` | wheel 字节、ELF AArch64、补丁前后摘要，未安装或运行 |

- 制品来自 [PyPI 3.0.4](https://pypi.org/project/mistral-vibe/3.0.4/#files)。`patches/acp-model-source/vibe/linux.json` 记录逐架构 URL 和原生 Harness、同包 CLI、补丁与启动器摘要。两份 wheel 的 `vibe-acp` 均指向 `vibe.acp.entrypoint:main`。七个现有补丁输入相同、一个待新建文件均缺失，现有 Darwin v4 补丁重放后八项已审 Python 文件摘要完全一致。
- 安装使用 GNU glibc ≥ 2.28、Python 3（安装入口）、uv、常规 CPython 3.12（私有虚拟环境）、`patch` 和 `/bin/sh`。由 `install_linux.py` 校验 wheel，再用 `uv venv --python 3.12 --relocatable` 与 `uv pip install --index-url https://pypi.org/simple --only-binary :all:` 安装固定 wheel 及其依赖，复用已有 `apply_patch.py`。wheel 的 METADATA 固定 ACP 0.11.0、MCP 2.3.0 等直接依赖及其平台条件，当前没有双架构依赖归档锁或冷安装证据。依赖缺 wheel 时应失败。安装目标仍用 `vibe-3.0.4-model-source-v4`，固定绝对入口为同一目录下的 `bin/vibe-acp`，默认参数为空。启动器使用同一 venv 的 `bin/python -I` 调供应商入口，避免从工作区及 PYTHONPATH 导入模块。
- 任务 48 注册输入。以 `all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64"))` 在 `acp-agent-registry/src/managed_runtime.rs` 注册 `mod vibe`。将 `vibe::{MANIFEST, LAUNCHER, INSTALLER}` 分别写入现有 staging assets 的 `vibe/linux.json`、`vibe/launcher.sh`、`vibe/install_linux.py`，继续写入公共合约和现有三项 Vibe 补丁素材。install 的平台守卫只为上面两个目标放行受管安装。共享 `managed_runtime/install.py` 的 Python 分支在 Linux Vibe 调 `python3 <assets>/vibe/install_linux.py <assets> <runtime> <uv>` 并退出该分支。Darwin 安装分支仍选原 wheel。
- 在共享 `verify` 的 Python 分支，对两个 GNU 目标和 `contract.package == "mistral-vibe"` 返回 `vibe::verify_linux(directory, &contract.version, &contract.files)`，用于 staging、安装回读和注册解析。Linux `verify_reviewed_agent_executable` 仅接受 `reviewed_agent_command("vibe")` 返回的规范化固定入口。比较失败直接拒绝，不能落入通用 Python 分支仅校验邻近目录。普通 `cli_local` 仍保留已有本机发现和账号路径。Apple Silicon 的安装、入口、选模、技能及审批分支均保持现行逻辑。
- 运行路径沿用 `drivers::vibe::configure` 的私有 TOML、`--session-model-config`、回环中继和当前启动引用，模型协议选择沿用 `source_protocol`。技能沿用 `skill_mounts::vibe::configure` 的已审入口及追加扫描，在卸载时传空目录。3.0.4 `mcp_servers` 用按 name 的原生 union merge，`session/new` 和 `session/load` 使用当前 ACP 声明。默认程序化目录保留 `search_tool_functions`、`run_typescript`，审批继续使用原生 effect ID。禁用 v2 直列补丁。两架构最终能力需走 `model_source/drivers/mod.rs::capabilities` 的运行平台、已审入口、协议和权限条件，本轮保留对外门控。
- 测试已放在现有 `drivers/vibe.rs::tests` 模块，任务 48 无需注册。Linux GNU 上供应当前构建的网关及经正式安装的对应架构 v4 目录，设置 `SHIJU_ACP_VIBE_GATEWAY_BIN` 与 `VIBE_COEXISTENCE_RUNTIME` 为绝对路径，执行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_vibe_contract -- --ignored`。预期命中 1 项，零失败。无 `--ignored` 会跳过这个依赖外部运行时的测试。Python 夹具通过真实网关走四个会话，核对 Chat Completions 和选模回读，区分两类技能、原生读取与会话 MCP，同调用 ID 验证续轮。挂载轮批准后服务审计新增一次，拒绝和撤除轮零新增。旧 Linux 探针在平台门控失败，现有受管安装无法提供对应 Linux 制品。此测试的撤除是新会话对照。
- 两个 GNU 目标仍有相同阻塞。私有 `VIBE_HOME` 会隔离 Vibe 原生用户 MCP 配置和供应商凭据，受信项目配置仍走上游信任判断。没有经过宿主授权且保留配置来源的原生 MCP 注入契约，官方联网搜索也缺独立供应商授权及本机替代路径。Vibe 原生默认允许部分工具，`guard_vibe_mode` 只拦截未经授权的 `auto-approve`，不能证明原生编辑、命令、子代理或后台进程均按次经过宿主。原生 MCP 与外挂同名的隔离、权限变化后的同会话恢复与撤销、以及拒绝后原生服务零新增审计仍缺验收。需要补齐原生 MCP 授权与有效权限约束再开放完整能力。当前交付只提供固定制品、安装和身份验证候选接线及对应网关测试。
- 已做 wheel 和 ELF 校验、补丁 dry-run 与应用、八项产出校验、Rust 格式及 include 路径、Python AST、Shell 语法和定向 Git 空白检查。未运行 Cargo、Docker、安装器或真实模型，Linux 双架构定向测试退出码及命中数未获取。

- 实际提交为 `8783bddbe`。

### 继续排查与 Linux 审批补丁

- x86_64、aarch64 的同一 Linux 叠加层在已审 v4 上应用。`UnifiedPermissionResolver.resolve` 保留原生拒绝，把已有原生允许中的文件写入、编辑、前台命令、后台进程启动和写入、以及 provided 工具转为 ask。子代理的 per-call resolver 与会话结果取较严值。受管 ACP 统一使用 Core 的 PROMPT 门，保留程序化发现、技能和原生允许的只读操作，后台停止沿会话原有收尾语义。ACP 回调只接受一次批准或拒绝，丢弃带持久义的响应，继续由宿主校验当前提示审批。拒绝配置及插件 hooks，命令环境排除私有绑定路径、引用和中继令牌。此过滤不提供进程级沙箱。Linux 普通启动与 Darwin 行为沿用原分支。
- 在 Harness 的 `_StdioConnection.open` 拒绝受管环境的直接 stdio，含原生配置、项目插件、ACP 声明及后续重配置。上游先通过它执行外部程序来获取 `tools/list`，到 `tools/call` 才请求审批，单独给调用加门无法约束发现阶段启动。Linux 夹具改用带鉴权的回环 Streamable HTTP，复用生产 ACP HttpMcpServer 到 `streamable-http` 的投影，核对真调用及续轮、拒绝和撤除审计。Darwin 夹具继续 stdio。Linux 测试源码目前仍覆盖两类技能、原生读取与外挂 HTTP MCP，新写入/命令/子代理的逐调用及同一会话恢复回执待补。
- 任务 48 在上述安装约定中追加物料 `vibe::APPROVAL_PATCH` → `vibe/linux-host-approval.patch`。安装器在 v4 补丁后校验输入，精确打补丁并比对输出。两个目标的运行入口验证会以 `approval_files[].after` 覆盖共享摘要并补验新增 Python 文件。原有 hooks 或直接 stdio 的受管组合必须明确拒绝。HTTP 路径保留原生工具发现和当前会话 MCP 声明，缺审批能力时须失败关闭。保持对外平台门控。
- 重新排查后仍有两类共享的实质缺口，状态为已阻塞。两个架构的 `session_model + 官方 web_search` 需要独立的 Mistral 授权输入，或兼容的本机 connector bootstrap/执行协议。上游以 `config.get_mistral_provider()` 派生独立 `connector_api_key`，通用会话中继只接受模型协议，不能用于 `/v1/connectors/bootstrap`。普通 `cli_local` 继续处理用户原有账户。两个架构的 `session_model + 原生用户/项目 MCP` 缺当前会话授权的配置来源输入。原生项目信任记录也是 `VIBE_HOME/trusted_folders.toml`，在短期私有目录不存在。复制原生配置或信任记录会引入用户密钥、hooks 与执行权限；通用 ACP 声明会丢失原生配置来源。两个架构的 `session_model + stdio MCP` 还需受限的 Linux 宿主投影。现有 `mimo_mcp::prepare` 仅允许 Darwin arm64，实际使用 `/usr/bin/sandbox-exec`。Streamable HTTP 可直接保留作为独立候选，不计作 stdio 或原生 MCP 支持。两种候选源码及受限审批路径可交任务 48 集成，统一实现缺口排除前不可发布完整能力。
- 两个归档均完成两层补丁的精确 dry-run、应用及摘要核验。更新后的五个 Python 文件通过 Python 3.12 AST 解析。Rust 定向格式和 Git 空白检查通过。未执行 Cargo、Docker、安装及运行时测试。

- 本轮审批补丁和 HTTP 验收提交为 `d91fe5efd`。
