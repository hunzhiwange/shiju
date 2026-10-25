# 运行目录安全前置检查

取证日期：2026-10-25（Asia/Shanghai）。任务12仅静态调查与只读后端探测，动态尝试0次；没有启动模型、Gateway、容器或仓库外写入实验。

批次8的0348401f4已实现每次调用独立目录、首轮独立会话和可信metadata，Node验证18/18；其任务文件仅因无关Clippy待回归。本调查可依赖该已实现行为，但不将其作为安全证明。

## 结论与适用范围

当前只可确认 `pollution-only-directory`（仅防意外污染目录）。动态写任务尚未获准：缺少绑定真实Gateway配置及全部工具能力的受限执行证据，须由任务19采集环境、任务20执行受控canary后决定。本文是已完成的前置调查，不是任何一次尝试的准入凭证。

本机现有Docker daemon可响应，选择**已存在的独立容器机制**作为任务19优先评估对象；本次不创建容器、不研发沙箱。只有整条Gateway及其工具/后代进程处于受限环境，且无宿主仓库、答案、verifier、凭据或Docker socket暴露时才可候选。Shell内部沙箱不能替代整个工具链的隔离。若现有部署不能提供该环境，动态写任务保持阻塞，另立生产权限机制计划。

## 路径、符号与配置证据

以下路径均相对仓库根目录，行号为调查时源码定位；RustCodeGraph查询后对未索引评测文件使用直接读取。

| 边界 | 来源与行为 | 对准入的含义 |
| --- | --- | --- |
| 请求目录 | `crates/vw-agent/src/gateway/instance.rs:35` `resolve_directory`：query.directory优先于header，再回退进程目录 | 监督端必须同时检查URL查询与 `X-VibeWindow-Directory`，二者解析结果等于允许的workRoot |
| stream到运行时 | `crates/vw-agent/src/gateway/api/handlers/session/stream.rs` `chat_stream`、`start_gateway_chat_producer:866`；`root_dir=dir:920`，`start_prompt(Request { root: Some(root_dir) }):988` | 目录是运行上下文，不是操作系统隔离 |
| 工具能力 | `crates/vw-agent/src/session/processor/helpers.rs:34` `allowed_tool_ids_for_request_from_specs`；无allowed_tools使用base，有列表取交集并保留builtin required工具 | 不能只检查请求列表；检查最终可执行工具、别名、必需工具与动态能力，包括AgentTool、MCP、ACP、浏览器、网络和后台任务 |
| 工具root与全权选项 | `crates/vw-agent/src/session/processor/runner.rs:457-490`，`ToolUseContext::new`、`request_full_access_enabled`、`ToolRuntimeContext::new` | `options.full_access=true`还会绕过本轮非CLI审批；受限配置必须拒绝该路径 |
| 实际工具工厂 | `crates/vw-agent/src/tools/toolset/mod.rs:175-198` `runtime_config_for_context`、`:222-272` `execution_environment_for_config` | 调用 `config::schema::load_or_init_config()`，失败回退默认配置；使用NativeRuntime，不能假设配置中的Docker runtime已覆盖该工厂 |
| 配置来源 | `crates/vw-agent/src/config/schema/config_load.rs:280-353` `load_or_init_config_for_tenant_with_environment`、`load_or_init_config` | 工具加载默认租户/无显式project_root配置，经过运行目录解析、SystemConfigStore和环境覆盖；stream另用 `get_project_for_tenant` 读取reliability。不能只记录workRoot中的配置文件，应记录实际生效配置和来源摘要，拒绝回退未核实的配置 |
| 权限构造 | `crates/vw-agent/src/tools/toolset/mod.rs:312` `security_for_tool_context`；`crates/vw-agent/src/security/policy/mod.rs:694` `SecurityPolicy::from_configs` | 来源为config.autonomy和config.security.bash；核对workspace_only、allowed_roots、forbidden_paths、命令/重定向/环境策略。full_access清空禁止路径、允许任意命令并关闭Bash检查 |
| 文件边界 | `security/policy/mod.rs:580` `is_resolved_path_allowed`；`tools/file_read/mod.rs:1091`、`tools/file_edit/mod.rs:87,103`、`tools/apply_patch/mod.rs:191-233` | 路径与解析后路径检查是应用层约束；workspace/allowed_roots在forbidden_paths前允许，不能用禁止列表覆盖已放行根目录，更不能推导整个宿主不可读写 |
| Shell开关 | `crates/vw-agent/src/tools/shell/mod.rs:132` `sandbox_config_for_security`；`tools/shell/sandbox/mod.rs:46` `should_use_sandbox` | Full且非workspace_only会禁用沙箱；禁用、override、排除命令、后端不可用均可返回use_sandbox=false。后端存在不等于每条命令受限 |
| macOS沙箱 | `crates/vw-agent/src/tools/shell/sandbox/executor.rs:115-162` `wrap_with_seatbelt`、`generate_seatbelt_profile` | 全局allow file-read，允许工作区及 `/tmp`、`/private/tmp` 写入；无法承诺防对抗读取或严格只写workRoot。同在临时目录的其他尝试也不能视为互相安全隔离 |
| 其他平台 | 同文件 `wrap_command:89`；Linux委派FirejailSandbox，不直接应用同一filesystem列表；其他平台bail | 不能把macOS配置结论迁移到Linux；平台不支持或后端不可用时不得签发受限证明 |

现有测试定位（本任务不运行Cargo，不将测试存在视为已通过）：

- `crates/vw-agent/src/gateway/instance_tests.rs`：目录优先级与空header。
- `crates/vw-agent/src/session/processor/helpers_tests.rs`：`request_filter_only_retains_requested_tools_and_dynamic_skill_loader`、`request_filter_distinguishes_missing_list_from_explicit_empty_list`。
- `crates/vw-agent/src/tools/toolset/tests.rs`：`full_access_context_disables_workspace_only_for_tool_security`。
- `crates/vw-agent/src/tools/file_read/tests.rs`：`file_read_blocks_symlink_escape`、`file_read_outside_workspace_allowed_when_workspace_only_disabled`。
- `crates/vw-agent/src/tools/shell/tests.rs`：`shell_full_access_disables_workspace_sandbox`、`shell_workspace_scoped_full_autonomy_keeps_sandbox`。
- `crates/vw-agent/src/tools/shell/sandbox/sandbox_tests.rs`：`sandbox_disabled_globally_short_circuits`、`sandbox_global_disable_takes_precedence_over_override`；`executor_tests.rs`覆盖后端探测及包装，但不能替代实际边界canary。

## fixture、答案与verifier

`evals/promptfoo/attempt-workspace.mjs:28` `prepare`仅复制可信fixtureFiles公开输入allowlist，拒绝symlink，不复制整个fixture树；workRoot由系统tmpdir下mkdtemp创建。`vibewindow-provider.mjs` runtime-accuracy分支使用可信config.manifest/taskId/fixtureRoot/fixtureFiles，而非Agent或vars身份。

`evals/promptfoo/artifact-oracle.mjs:19` `createArtifactOracle`的expectedFiles/unchangedFiles字节来自监督端Map，拒绝模型指定verifier，逐字节独立验收。答案和verifier必须留在监督进程侧；公开fixture允许Agent读，原始fixture源、答案、保留集、verifier、结果轨迹不应挂载进Agent环境。验收必须等所有写入者及后代退出，防止并发篡改。

目录外保存不等于不可访问：同宿主UID有权限时，Shell的全局读、解释器、其他工具仍可能读取仓库源码和答案。当前不能宣称防对抗读取，也没有核实实际题库答案的部署位置。任务19须逐项列出真实路径、挂载、UID及可达性；缺失或可读取私有答案时拒绝动态评测，不以oracleDigest或提示词约束替代隔离。

## 任务19/20可执行准入与退出步骤

1. 任务19从可信执行配置定位Gateway进程、租户、UID、平台、配置版本和环境覆盖的非敏感摘要。检查实际工具工厂与所有后代所在的容器/低权限边界；读取容器inspect或现有沙箱配置，确认挂载、网络、执行能力、超时和进程回收。不得输出原始环境、令牌或答案。
2. 只允许该次prepare产生并核实realpath/dev/ino的绝对workRoot；容器路径映射必须有可信对照。拒绝仓库根、HOME、共用临时目录作为允许写根，拒绝symlink、跨尝试allowed_roots和full_access。禁止写范围包括宿主仓库、fixture源、答案/verifier/保留集、监督配置/证明、其他尝试及凭据目录。运行时状态所需额外写目录须显式列明且为本次独占，不能默许整个tmp。
3. 核对完整工具能力；所有可写、可执行、可派生进程的路径均须受同一边界控制。不能仅以shell命令白名单、allowed_tools、workspace_only或Docker服务在线证明安全。若没有已有且可核实的受限环境，记录阻塞，停止，不请求扩大宿主权限。
4. 任务20在上述已有受限环境中验证受控canary：允许目录内产物成功，受控禁止目录的读/写按要求失败，检查symlink/绝对路径/后代进程边界及退出回收。使用专门可销毁canary，不触碰真实宿主敏感文件；本任务不执行该步骤。
5. 监督端绑定配置摘要、workRoot、attemptId和证据生成证明；配置、工具集、挂载、身份、二进制或workRoot变化使证明失效并重新检查。任何边界失败、证明缺失/不匹配、后台写入者未退出、超时或流异常均停止采样，保留目录与轨迹；不计算为有效安全通过。所有写入者退出后才验收，再按任务7生命周期清理。

## 准入记录字段契约

由可信监督端产生，不能由模型自报。字段与现有Provider的 `accuracyAcceptance.admit` 校验对齐；字段格式合法仍不代表安全边界已获验证。

| 字段 | 要求 |
| --- | --- |
| mode | `restricted-sandbox` 或 `pollution-only-directory`；后者只说明防污染，不能据此批准需要安全边界的动态写任务 |
| configurationDigest | 64位小写SHA-256，覆盖实际生效非敏感配置、来源摘要、工具能力、UID/平台、挂载及沙箱/容器版本；完整来源摘要存证据 |
| workRoot | 本次尝试被授权的精确绝对路径，与Provider请求和可信路径映射一致 |
| forbiddenWriteScopes | 非空绝对路径数组，列出禁止写入范围，不能与允许根重叠 |
| evidencePath | 监督端可读的绝对证据路径；Agent不可修改，不挂载到其环境 |
| verifiedAt | 带时区的ISO时间，记录实际验证时间；本文日期不能复用作canary时间 |
| digest | 64位小写SHA-256，由可信监督端绑定记录与证据计算；摘要不是认证机制，可信执行配置必须核实来源 |

证据文件另外包含attemptId、配置来源摘要、允许读写/执行根、后代回收条件、每项检查命令/退出码、有效canary数量、输入输出摘要和阻塞原因。Provider仅引用可信执行配置中的证明，不从prompt/vars/模型响应获取证明。任务19/20负责实际签发；本文不提供虚构通过样例。

## 本次验证记录

- `~/.rustcodegraph/bin/rustcodegraph status`：退出0，4240文件；使用explore与node跟踪上述符号。未同步或修改索引；当前并行channels/project未提交改动不作为已验证发布状态，任务19应冻结实际二进制版本。
- `command -v sandbox-exec`、`command -v docker`：发现现有命令。`/usr/bin/sandbox-exec -h`退出64，符合源码backend_available探测条件，但未执行沙箱命令。
- `docker info --format '{{.ServerVersion}}'`：退出0，28.1.1；仅证明daemon可用，未证明容器限制。`id -u`退出0，UID 501，无独立低权限身份的验证证据。
- 只读探测原始记录：`evals/promptfoo/results/isolation-preflight-availability.json`（忽略目录）；包含各命令退出码、时间和dynamicAttempts=0。调查没有有效动态安全实验、没有模型尝试，不存在红绿测试日志。
- 纯文档/取证任务按任务12契约不调用Cargo，也不以零测试声称运行时安全通过。提交前执行路径/符号引用检查及 `git diff --check`。
