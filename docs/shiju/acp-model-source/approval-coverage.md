# ACP 双档权限验收

范围为生产 `/v1/global/config/acp/defaults` 返回的 22 个后端。宿主协议矩阵另外核对兼容别名、旧命令和同名命令覆盖。真实适配器证据来自 Darwin arm64 和临时项目。

宿主接到 `fs/write_text_file` 时，会核对当前会话、工作区和文件边界，再执行普通文件编辑。未明确授予完全访问时，单独传入 `ApproveAll` 会回落为普通审批。工具和终端回调同样需要当前授权，宿主批准只选择单次许可。

| 后端 | 固定版本 | 模型来源 | 控制与覆盖 |
| --- | --- | --- | --- |
| auggie | 0.36.0 | 原生 | 已验 npm 包摘要，固定版无可用离线模型注入 |
| claude | 0.26.0，SDK 0.2.96，CLI 2.1.96 | 两种 | 两种来源均实测，原生写入走单次确认 |
| codebuddy | 2.179.3 | 两种 | 两种来源均实测，宿主单次审批 |
| codex | 2.5.0，CLI 0.165.0 | 两种 | 两种来源均实测，审批档的原生写入与命令由只读模式拒绝 |
| copilot | 1.0.105 | 两种 | 两种来源均实测，敏感操作由宿主单次审批处理 |
| cursor | 2026.10.22-27dcf5f | 原生 | 固定包 GET 403，缺离线模型配方 |
| droid | 1.3.0 | 原生 | 已验二进制摘要，ACP 创建会话要求平台登录 |
| gemini | 0.66.0 | 两种 | 两种来源均实测，维持 `default`，宿主单次审批与原生工作区限制共同生效 |
| goose | 1.57.0 | 两种 | 两种来源均实测，原生 `approve` 与宿主单次审批 |
| hermes | 0.19.0 | 两种 | 两种来源均实测，桌面原生执行要求当前完全访问授权 |
| iflow | 0.5.19 | 两种 | 两种来源均实测，维持原生越界写入限制 |
| kilocode | 7.8.18 | 两种 | 两种来源均实测，桌面原生执行要求当前完全访问授权 |
| kimi | 2.5.0 | 两种 | 两种来源均实测，宿主单次审批 |
| kiro | 3.0.0 | 两种 | 两种来源均实测，宿主单次审批 |
| openclaw | 2026.10.10 | 两种 | 两种来源均实测，桌面原生执行要求当前完全访问授权 |
| opencode | 1.19.5 | 两种 | 两种来源均实测，桌面原生执行要求当前完全访问授权 |
| pi | 0.0.22，CLI 0.73.1 | 两种 | 两种来源均实测，桌面原生执行要求当前完全访问授权 |
| qoder | 1.1.82 | 原生 | 已验 npm 包摘要，缺平台登录与自定义模型权限 |
| qwen | 0.26.7 | 两种 | 两种来源均实测，原生覆盖写入前先读取 |
| snow | 0.9.14 | 两种 | 两种来源均实测，宿主单次审批 |
| trae | 0.121.3 | 原生 | 固定包 HEAD 200，缺核验摘要及离线模型配方 |
| vibe | 3.0.4 | 两种 | 两种来源均实测，宿主单次审批 |

“两种”指智能体原生模型和客户端模型。五个仅支持原生模型的客户端路径标记为不支持。版本与离线配置取自[固定版本清单](cli-versions.json)。

## 固定版运行时准备

所有新增运行时在 Darwin arm64 上准备于临时目录。npm 包通过仓库管理安装器核对官方 registry 的 SHA-512 和合同中记录的运行文件 SHA-256。Python wheel、Goose 源码和补丁、Kiro 压缩包及运行文件使用管理合同的 SHA-256。Kimi 原生二进制使用固定版官方清单的 Darwin arm64 SHA-256。 [管理运行时合同](../../../crates/vw-acp/src/managed_runtime/contracts.json) · [固定版本清单](cli-versions.json)

| 后端 | 已核验来源 | 原生模型隔离入口 |
| --- | --- | --- |
| CodeBuddy | npm `@tencent-ai/codebuddy-code` | `CODEBUDDY_CONFIG_DIR` |
| Goose | 官方 v1.57.0 源码及仓库补丁 | `GOOSE_PATH_ROOT` |
| Hermes | PyPI `hermes-agent` wheel 及仓库补丁 | `HERMES_HOME` |
| iFlow | npm `@iflow-ai/iflow-cli` | `IFLOW_HOME` |
| Kilocode | npm `@kilocode/cli` 及平台原生程序 | `XDG_CONFIG_HOME` |
| Kimi | 官方 Darwin arm64 二进制 | `KIMI_CODE_HOME` |
| Kiro | 官方 Darwin 通用 ZIP | `KIRO_HOME` 及回环 KAS 控制面 |
| OpenClaw | npm `openclaw` 及仓库补丁 | `OPENCLAW_STATE_DIR` 及隔离的回环 Gateway |
| Pi | npm `pi-acp` 及固定 0.73.1 引擎 | `PI_CODING_AGENT_DIR`、`PI_ACP_STATE_DIR` |
| Qwen | npm `@qwen-code/qwen-code` | `QWEN_HOME` |
| Snow | npm `snow-ai` | `SNOW_CONFIG_DIR` |
| Vibe | PyPI `mistral-vibe` wheel 及仓库补丁 | `VIBE_HOME` |

- Auggie 0.36.0 和 Qoder 1.1.82 的 npm 包已按固定版本清单核对 SHA-512。Auggie 0.36.0 的 ACP 参数拒绝加载扩展，Qoder 的自定义模型仍依赖平台登录和账号权限。两者缺经核验的离线模型配方。 [Auggie 上游门槛证据](evidence/auggie.md) · [Qoder ACP](https://docs.qoder.com/cli/acp) · [Qoder 自定义模型](https://docs.qoder.com/cli/custom-models)
- Droid 1.3.0 的 Darwin arm64 二进制与官方摘要及既有记录一致。隔离探针的 `initialize` 成功，`session/new` 返回登录错误 `-32000`，回环模型收到 0 次请求。 [Droid 固定包](https://downloads.factory.ai/factory-cli/releases/1.3.0/darwin/arm64/droid) · [登录门槛探针](evidence/probe_upstream_35.py)
- Cursor 固定版 CDN 的 GET 返回 403，既有可执行文件摘要无法用于本轮新下载。Trae 0.121.3 固定包 HEAD 返回 200，仓库尚无该包的核验摘要和离线配置配方。 [Cursor 固定包](https://downloads.cursor.com/lab/2026.10.22-27dcf5f/darwin/arm64/agent-cli-package.tar.gz) · [Trae 固定包](https://lf-cdn.trae.com.cn/obj/trae-com-cn/trae-cli/trae-cli_0.121.3_darwin_arm64.tar.gz)

共享工作区回归重新检查了以下外部前提。探针使用临时目录和隔离配置，未使用真实凭据，结果不计入权限通过数。

| 后端 | 本轮探针 | 补验所需条件 |
| --- | --- | --- |
| Auggie 0.36.0 | npm 包及入口摘要匹配。禁网运行确认固定版拒绝 `--extension` | 上游提供与固定版本匹配的 ACP 离线模型注入，或提供经授权的隔离测试账号及可控模型 |
| Cursor 2026.10.22-27dcf5f | 固定包 GET 仍为 403 | 可下载且能匹配已审查摘要的固定包，以及经核验的隔离模型配方或获授权的测试登录与可控模型 |
| Droid 1.3.0 | 官方 SHA-256 和仓库摘要均匹配。回环 BYOK 配置下 `initialize` 成功，`session/new` 要求认证，模型请求为 0 | 允许使用专用 Factory 测试账号，并准许回环自定义模型 |
| Qoder 1.1.82 | npm 包摘要匹配。禁网执行 `--acp` 时 `initialize` 成功，`session/new` 返回认证错误 `-32000` | 具备 CLI 自定义模型权限的专用测试账号，以及账号允许的可控模型配置 |
| Trae 0.121.3 | 固定包 HEAD 为 200，响应无可核验摘要，`.sha256` 和 `.sha256sum` 均为 404 | 可核验的 0.121.3 产物摘要，以及与该版本匹配的隔离 ACP 登录与可控模型配置 |

当前官方文档为后续配置提供入口，Trae 的现行 2.0 文档与验收固定版存在代际差异。 [Auggie CLI](https://docs.augmentcode.com/cli/reference) · [Cursor ACP](https://prod.cursor.com/docs/cli/acp) · [Droid 自定义模型](https://docs.factory.com/model-independence/byok) · [Qoder ACP](https://docs.qoder.com/cli/acp) · [Qoder 自定义模型](https://docs.qoder.com/cli/custom-models) · [Trae 配置](https://docs.trae.cn/cli_config-file)

Claude 通过桌面网关先后接收两次工作区写入、越界写入、命令和删除。审批档只对两次工作区写入答复单次允许，两次均落盘。完全访问期间五项操作均生效。撤销后的五次新请求均被拒绝。原生模型路径的模式回读为 `default → bypassPermissions → default`。客户端模型在轮次结束后释放进程，证据来自审批事件与实际文件状态。固定版 ACP 回退测试另行核对原生工具请求和组织拒绝规则。

Codex 的 `agent` 与 `workspace-write` 可绕过宿主文件回调写入系统临时目录。桌面审批档改用 `read-only`，恢复和显式设置也遵守这一限制。两种模型来源都观测到原生审批档拒绝五项操作、完全访问允许操作、撤销后再次拒绝。原生模型路径回读为 `read-only → agent-full-access → read-only`。

Copilot 的两种模型来源都通过单次批准后执行工作区写入命令，敏感操作在审批档与撤销后被拒绝。Gemini 的两种模型来源都在审批档确认并完成两次工作区覆盖写入。完全访问允许工作区编辑及测试命令，原生工作区限制继续拒绝越界 Write。Gemini 客户端模型验收使用原生 Google 协议中继。

OpenCode 实测在审批档直接执行了创建标记和删除文件的 Bash 命令。全局权限环境变量之后仍会叠加代理专属规则，无法用全局覆盖约束任意现有配置。桌面审批档在启动前拒绝这条原生执行路径。 [固定版配置合并](https://raw.githubusercontent.com/anomalyco/opencode/v1.19.5/packages/opencode/src/config/config.ts) · [固定版代理权限规则](https://raw.githubusercontent.com/anomalyco/opencode/v1.19.5/packages/opencode/src/agent/agent.ts)

Hermes 和 Kilocode 在审批档与撤销后能执行创建标记的命令。OpenClaw 和 Pi 的原生编辑、越界写入、命令和删除均绕过宿主审批。桌面入口统一要求这些后端持有当前完全访问授权，并在准备 OpenClaw 托管 Gateway 前检查授权。部分适配器会重写模型工具调用 ID，报告同时记录请求数与实际文件副作用。

同名 Codex 替换夹具只通过 ACP 回调操作文件。网关验证了无需批准的工作区创建和编辑、完全访问后允许越界写入、撤销并加载后恢复拒绝。两个并行工作区使用相同的远端会话 ID，同时等待独立审批，网关拒绝六次跨工作区答复和六次重复答复。未知自定义 key 由宿主协议矩阵覆盖，网关的智能体配置入口会拒绝该 key。

## 后台入口

| 入口 | 授权来源与无交互行为 |
| --- | --- |
| 计划运行 | 规划、各批次及续轮每次读取同租户、同工作区、同 ACP 绑定的父会话已保存权限。无匹配授权使用审批档。ACP 计划产物写入工作区内的 `shiju-plans/`，宿主拒绝目录外的清单文件引用与链接。 |
| 本地调度 | 任务及 worktree 租约没有独立的完全访问授权。网关解析出 ACP 后使用审批档，拒绝需要人工确认的操作。 |
| 桌面任务执行 | 后台请求在网关解析出 ACP 后使用审批档，显式指定后端或由代理配置选中后端均适用。 |
| 定时任务 | 直接执行采用当前任务记录的 `full_access`，授权仅用于本轮。投递到任务池后按本地调度权限执行。 |
| 标题生成 | 使用独立 ACP 会话，权限模式为 `deny-all`，无交互策略为 `deny`。 |
| `shiju acp` | 入站 stdio 服务通过外部客户端接收 ACP 请求。它不提供本地后台任务的授权来源。独立 ACP 客户端仍使用调用者给定的权限及无交互策略。 |

审批档继续允许符合宿主边界的普通文件写入。需要一次性人工确认的原生写入、命令和删除在后台拒绝，显式 `fail` 策略仍返回错误。上表列明要求当前完全访问授权的原生适配器在后台同样适用。缺少固定运行时边界证据的五个供应商仍受前述限制。

Darwin arm64 的 Vibe 3.0.4 以真实计划入口运行两种模型来源。每轮完成 9 个工具请求，普通文件创建和覆盖落盘，越界写入、终端命令和删除均无副作用，计划完成时没有待审批请求。本地调度通过真实网关和受控 ACP 对端复验了工作区创建及覆盖、三次拒绝和正常结束。

## 复现

按编号任务的共享槽位规则设置 `CARGO_TARGET_DIR` 和 `CARGO_BUILD_BUILD_DIR`。管理运行时在临时目录调用仓库 `managed_runtime/install.py`，传入该运行时的合同及补丁素材目录、待创建的运行时路径和后端 key。将已核验的固定版适配器加入 `PATH`。验收网关与模型端点只使用回环地址，运行时使用隔离配置和临时工作区。

```sh
cargo test -p vw-acp --lib acp_desktop_approval_backend_matrix -- --test-threads=1
cargo test -p vw-acp --lib acp_full_access_ -- --test-threads=1
cargo build -p vw-cli --bin shiju
python3 -m unittest discover -s scripts -p acp_approval_model_tests.py
python3 scripts/verify_acp_approval.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --output target/acp-approval-evidence/matrix.json
cargo test -p vw-agent --lib acp_background_approval_ -- --test-threads=1
VW_RECOVERY_WINDOW_HTTP=1 cargo test -p vw-agent --lib acp_background_approval_scheduler_writes_only_inside_worktree -- --ignored --test-threads=1
python3 scripts/verify_acp_background.py --gateway "$CARGO_TARGET_DIR/debug/shiju"
python3 scripts/verify_acp_background.py --gateway "$CARGO_TARGET_DIR/debug/shiju" --session-model-runtime "$VIBE_RUNTIME"
```

`VIBE_RUNTIME` 指向已核验的 Vibe 3.0.4 运行时目录。

JSON 分别记录宿主夹具与真实适配器结果。Darwin arm64 共享工作区回归仍为 44 条，34 条真实路径通过，5 条客户端模型路径为生产不支持，Auggie、Cursor、Droid、Qoder 和 Trae 的 5 条原生路径受上述外部条件阻塞。宿主并行夹具通过。全矩阵因 5 条外部阻塞返回 1。使用 `--agent <key>` 可单独重跑某个供应商，该运行不作为完整矩阵。
