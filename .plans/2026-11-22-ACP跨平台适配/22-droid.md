# 任务 22 适配 droid

先阅读并遵守 [公共执行规则](plan.md)，再执行本任务。

批次：【批次 3】 依赖：批次 1、2

状态：已阻塞

来源任务：无

目的：完成 droid 的 Linux x64 和 ARM64 的运行契约。

## 文件与范围

- 候选落点为 `crates/libs/llm-runtime/src/acp/model_source/drivers/droid.rs`、`crates/libs/llm-runtime/src/acp/skill_mounts/droid.rs`、`crates/libs/acp-access-policy/src/access_policy/droid.rs`、`patches/acp-model-source/droid`。只改该 key 独占文件、受管补丁和独立测试。宿主共享注册、父模块、安装脚本和能力接线由任务 48 写入。
- 新建或复用该 key 的独立测试文件。若驱动没有独占测试入口，在 `crates/libs/llm-runtime/src/acp/acceptance_support/` 新建 `platform_droid_tests.rs`，由任务 48 负责测试模块注册。现有证据见 `docs/dev/acp/mcp-acceptance/droid.md`、`docs/dev/acp/skill-coexistence/droid.md`。

## 上下文

审计基线版本为 1.9.0。MCP 现状｜原版 ToolSearch 延迟发现，已证批准、拒绝、执行及回传。模型来源现状｜原版 Airgap/BYOK 可选模，受审；通用，其他平台原版入口待验。原生工具现状｜普通原生搜索仍在，挂载保留非 MCP 延迟发现；旧枚举提示未形成目录门禁。详见审计矩阵。 [MCP 策略审计](../../docs/dev/acp/mcp-policy-audit/README.md)

## 实施

先读取任务 7–9 中适用的固定制品、SDK 隔离或已有受管移植做法。选择与该适配器相匹配的路径，沿用相同的接线交付字段。

1. 优先定位审计行中的现有 MCP 断点及模型来源限制。保留已审的实现方法（v2 直接公布 MCP，改写发现提醒；缺 v2 时可回到受审 v1），按目标核实可复用程度。
2. 对两个 Linux 架构核对上游制品、固定入口、安装依赖与可验证身份。优先用原生入口，其次移植该 key 的现有受管补丁或已有宿主桥。
3. 通过该适配器实际入口实现指定本机模型、原生工具和 MCP、会话外挂 MCP、两类技能及宿主审批，按公共规则处理撤销及恢复。复用已完成的宿主模块。保留普通 cli_local 路径。发现上游能力或发行物缺失时继续其它组合，记明失败点，维持拒绝。
4. 向任务 48 给出逐目标版本及制品、确切注册点、能力判定、安装参数、测试入口与未解决限制，写入本编号文件的交付记录。只读审查现有 Apple Silicon 分支，保留其已支持行为。任务 49 使用这份结果入账。

## 验证

- 在实现前写 `acp_platform_droid_contract`，通过真实宿主入口记录选中的制品及模型协议。断言原生和外挂工具、技能来源可区分，获准调用实际执行并续轮，拒绝或撤除后服务审计无新增。旧路由应暴露平台门控或本任务列出的适配器断点。
- 后续在 Linux Docker 容器内执行的红灯和绿灯命令为 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_droid_contract`。若需任务 48 接线，在该接线就绪的同一测试位置复跑，记录实际命中且零失败。
- 本轮只审查代码、固定依赖与断言，两个 Linux 架构分别登记源码证据或具体阻塞。

## 交付记录

代码提交 6549b97d3e、b2e5b93d33。

- GNU x64 选 1.9.0 的 `linux/x64-baseline/droid`，无需 AVX2。GNU ARM64 选同版 `linux/arm64/droid`。分别静态确认 ELF machine 62 和 183、对应加载器 `/lib64/ld-linux-x86-64.so.2` 和 `/lib/ld-linux-aarch64.so.1`，最高 GLIBC 符号版本均为 2.17。两个完整原版文件的 SHA-256 匹配各自供应商校验文件。制品 URL、原版及补丁 SHA-256、Bun 模块偏移见 [Linux 清单](../../patches/acp-model-source/droid/linux.json)。[Factory 安装入口与架构分支](https://app.factory.ai/cli)
- 受管补丁保留原生非 MCP 延迟发现及权限处理，直接公布 MCP 工具，Linux 提示只陈述当前已加载目录。两架构的源和 bytecode 位置分别固定。补丁命令为 `python3 apply.py --linux-arch <x86_64|aarch64> <已下载原版> <私有目录>/droid`。验证原版、补丁输出和内嵌模块布局后才发布 0700 文件。无需 Node 或 codesign 运行 Linux Droid。Darwin arm64 保留原补丁内容和 codesign 流程，未变更 v2 缺失才使用已审 v1 的选择。保留 Linux 普通 `cli_local` 官方安装及原生发现。
- Linux 受管修订使用 `droid-1.9.0-model-source-linux-v2`。安装、发现、覆写和启动的接线见下文续查。启动参数严格为 `exec --output-format acp`，模型驱动追加私有 `--settings`、`--model` 和 `--spec-model`。复用现有 `drivers::droid::configure` 的 Airgap、私有 HOME 及回环 bearer，原生线协议为 Chat Completions。
- 任务 48 保持两架构的受限能力门控。待具备隔离和代理再接通 `request.rs::do_stream_request_acp` 的 restricted_droid 分支、`skill_mounts/mod.rs::validate`、`process.rs::spawn_child_at` 和 `verify_droid_restricted_executable`。现有 `skill_mounts::droid::sandbox` 的路径投影适用于 Unix，工作区原生为 `.factory/skills`，当前外挂租约投影到私有个人根，并核验源及禁止路径重叠。该投影依赖客户端 `DroidSandbox::command` 限制工作区读、私有根写、进程执行和网络仅限本次回环 TCP 端口；当前实现只接受 Darwin arm64。单独放开技能 cfg 会绕过所需边界。
- x64 和 ARM64 均缺等价的进程与网络隔离。现有 Bubblewrap 包装隔离全部网络，无法直接访问宿主请求的回环中继，且未提供 Droid 的执行约束。Landlock 逐命令包装返回 Unsupported。另有独立阻塞在 `vw-agent/src/session/llm/acp/model_source/mimo_mcp.rs::prepare`，其平台门控和 stdio 子进程均依赖 Darwin sandbox-exec。Droid 批准关联位于 `acp-client/src/client/event_client.rs`，需现行 prompt 所有权、原始调用 ID、Factory MCP 元数据及当前中继，不能用通用提示或 full_access 替代。继续阻断原生/外挂技能和 MCP、批准后执行及撤销恢复的完整 Linux 能力。现有受限入口拒绝恢复和 fork，当前网关夹具的撤除轮使用新会话。同会话撤除、恢复、取消清理、原生静态 MCP 执行及 HTTP/SSE 尚缺证据或安全入口。
- 任务 48 在 `acceptance_support/mod.rs` 以 Linux GNU x86_64/aarch64 和 test 条件注册 `platform_droid_tests`。测试经独占安装器、网关选模和 `/chat/stream`，检查固定版本/摘要、Chat 路径与模型续轮、原生和外挂技能目录及随机附件、stdio 工具批准和同 ID 结果，拒绝及撤除后服务审计无新增。当前夹具绕过公共安装注册准备固定制品，网关仍会遇到受限平台门控。夹具的 Linux stdio 端使用共享验收服务，后续桥接必须让该测试服务在隔离边界内真实执行。

## 本轮验证

- 两种 Linux 制品的原版和产出双 SHA-256、ELF 架构及加载依赖、补丁所选的 Bun 模块与清零后的 bytecode 指针已静态核对。相应 JavaScript 均通过 `node --check`。Rust 修改文件通过 rustfmt 检查，Python AST、JSON 与限定 Git 空白检查通过。未执行上游二进制、安装、Cargo 或 Docker。
- 待授权并补齐隔离后，分别在 `linux/amd64` 和 `linux/arm64` 的 GNU 容器中以非 root 用户运行。设置 `SHIJU_ACP_DROID_GATEWAY_BIN` 为当前源码构建的网关绝对路径，再运行 `python3 scripts/cargo_slot.py cargo test -j 1 -p llm-runtime --lib acp_platform_droid_contract -- --ignored`，预期命中 1 项且零失败。当前无测试退出码、命中数及运行结果。

## 续查与接线更新

- Linux 固定安装的独占实现已交付。任务 48 按 Linux GNU x86_64/aarch64 注册 `managed_runtime/droid.rs`，在 `managed_runtime::install` 调用 `droid::install().await`，在 `reviewed_agent_command` 调用 `droid::reviewed_command()`，在 `verify_reviewed_agent_executable` 调用 `droid::verify_command(command)`。这些分支在 Darwin 合约和原版回退之前返回。模块已嵌入 Linux 清单、安装器及补丁，负责私有 Stage、HTTPS 固定下载、尺寸与双摘要校验、原子发布和发布后回读，公共 Python 安装器无需另增 Droid 分支。安装使用 Python 3、`/usr/bin/curl` 和对应 GNU 加载器。受限验证器和能力列表沿原门控。当前 `agent_registry.rs::built_in_agent_command` 通用分支会先尝试 `reviewed_agent_command`。任务 48 接入 Linux Droid 时令普通发现跳过该受管优先项，继续使用当前本机账号和 `droid` 路径。受管选模分支通过独立的 `reviewed_agent_command` 选择固定制品。网关夹具用同一个独占安装器生成制品再进入生产网关，因此可将制品准备和受限运行门控分别诊断。网关安装 API 的端到端回读留给任务 48。
- 复核批次 1 的 MCP 和技能底座，它们可以让已接入的目录和租约失效。Droid 的原生工具仍在供应商子进程内执行。Darwin 历史探针已证 `Skill` 和 `Read` 可在无 ACP 审批回调时执行，且禁止派生连 `session/new` 都会失败。Linux 需一个可用的隔离后端，允许重启同一受审 Droid、约束所读工作区和当前技能租约、控制写入且只允许本次回环中继。默认 Bubblewrap 模板无法接入该中继，放开共享网络会超过原边界。通用 Landlock 包装入口未实现。Airgap 和历史 `--only-tools` / `--remove-tools` 探针无法提供执行约束。纯文本或仅指定模型的降级也缺一个确认可关闭原生工具的入口。
- 复核 HTTP/SSE 替代和现有宿主 MCP 桥。1.9.0 的 ACP 声明支持 HTTP/SSE、load/resume，历史 HTTP/SSE 证据只到建会话。Linux 制品内可定位 ACP 权限和恢复标记，未发现 Droid 对宿主 `shiju.droid.toolPolicy` 元数据的实现。现有宿主 Droid 授权消费由 Droid 受限分支使用的 stdio 代理完成，仍绑定 Darwin 沙箱。即使把当前桥换成 HTTP，`VtcodeApproval::consume` 也只按工具和实参消费一次许可，执行端没有将 Factory 权限事件的原始调用 ID 与 MCP 请求双端绑定。目录过滤、用户批准和实际分发分别需要证据，避免相同工具和实参的另一请求消费许可。直接把外部 URL 或 stdio 命令交给供应商只绕开宿主执行租约，因此保留拒绝。
- x64 和 ARM64 均交付固定安装、校验、模型驱动参数、会话级验收入口。两种目标的完整受限会话仍为已阻塞，影响有无外挂时的原生工具、原生 MCP、外挂 stdio/HTTP/SSE、两类技能以及撤权之后的恢复。最小缺失输入为可用的 Linux 请求级隔离入口和经过审查的逐调用执行协议或等价代理，后者要在实际执行前核对会话、调用 ID、实参和租约，并能重新建立恢复时的目录。可沿现有 `DroidSandbox::command`、`prepare_stdio_mcp_launch` 和 approval/relay 所有权接入。保持现有公开门控，后续的红绿验证命令同上。
- 本轮通过新安装器 Python AST、清单尺寸和本地已下载制品的静态比对，以及限定 Rust 格式和 Git 空白检查。未执行安装器、上游二进制、Cargo 或 Docker。
