# Nova ACP 接入证据

日期：2026-11-14。发行版本固定 `@compass-ai/nova@1.1.65`；测试平台 macOS arm64、Node 26.13.0。其他平台只有安装参数测试，不代表实机通过。

## 官方来源与安装

- 官方文档：https://github.com/Compass-Agentic-Platform/nova
- 官方发行：https://registry.npmjs.org/@compass-ai%2fnova/1.1.65
- 固定参考快照025迁移：`nova`、`npx -y @compass-ai/nova acp`、旧技能目录 `.compass/skills`。
- 实际固定入口 `npx -y @compass-ai/nova@1.1.65 acp`，本地优先发现 `nova acp`。官方包内入口为 scripts/bin-wrapper.mjs，声明 Node>=22.13；官方平台映射 Windows x64、macOS arm64/x64、Linux arm64/x64。
- 官方源首次安装 EIDLETIMEOUT；官方 npm pack 后从同一固定 tarball隔离安装成功（507个包，ignore-scripts）。未修改全局安装或读取用户全局凭据。

## 最小同意与受控模型

用户明确授权只在隔离临时目录保存人工测试会话，全部可选上报关闭。当前交互向导的 errorReporting 默认 true 且无独立选择，因此未保存该向导。通过发行包公开导出 `getConsentManager` / `createMinimalConsent` 调用官方 grantConsent API；返回 dataCollection=true、telemetry=false、analytics=false、errorReporting=false、isValid=true。没有伪造同意文件或复制用户配置。

独立回环 HTTP 服务只返回人工构造的 Anthropic SSE，不使用真实模型账号、真实用户消息或凭据。真实 CLI 同会话两轮 `turnOutcome.status=completed`，实际 `/v1/messages` 请求两次、模型 `claude-sonnet-4-6`。这证明 CLI 本地模型路径，不证明任意 ShiJu provider/model 绑定。

## 必要的供应商协议适配

默认 CLI 原生 edit_file 会直接写入文件，因此不能直接沿用默认启动行为。实际供应商协议：

- initialize.clientCapabilities._meta[`compass.owner-approval.v1`]=true；要求回执 capability 同名 true 且 `_meta.ownerApprovalRequired=true`。
- session/new 与 load 同时携带该协商，关闭 peer 能力；恢复经 load 重新应用当前约束。
- 原生审批回调使用 `permission/request`，而 ACP 标准为 `session/request_permission`。仅 Nova 的入站 JSON-RPC 请求转换此方法，保留 id、params 和其他消息；复用原权限分类、宿主回调、拒绝策略与会话身份验证。不改全局 ACP 方法分派。
- 非法 JSON、截断帧、超过16MiB的缓冲明确失败；能力缺失的旧/变化版本在 initialize 拒绝。
- 拒绝测试实际文件保持 before，放行测试应写入 after；拒绝后第二轮仍可完成。不能把 queued 或 end_turn 单独视为成功。

## 能力边界

| 能力 | 当前结论 |
|---|---|
| ACP 会话 | 官方 CLI initialize、新建、两轮受控模型请求已验证；生产客户端工具/取消/恢复见最终验证 |
| 版本身份 | npm 1.1.65 实际握手 agentInfo 为 kore-cli/1.0.0，不能混淆两者 |
| 权限 | 强制宿主审批协商；不声明原生完全访问，不把default/code/plan模式当授权 |
| MCP | 上游 owner approval 模式明确拒绝非空MCP；宿主在new/load前明确拒绝，不丢弃所选服务 |
| 模型 | CLI返回内置Anthropic模型目录。任意宿主provider/model绑定尚未验证，保留既有 SessionModel 拒绝门控，不回落CLI默认模型 |
| 技能 | 官方支持 `.nova` 与旧 `.compass`，当前newSession实现有skillScope但stdio分派没有传入该字段；未宣称选定技能隔离挂载可用，保留既有拒绝路径 |
| 斜杠命令 | 握手后通知含loop/tag/peers/marketplace等；不因此开放宿主外的peer或marketplace授权 |
| 认证失败 | 空凭据session/new RPC -32000要求setup；受控测试仅子进程虚拟凭据 |

## 双重交付

1. ACP注册、固定版本安装、强制审批协商与方法转换由vw-acp/vw-config-types承担。
2. `acp-nova` 通过生产 merged_agent_catalog 自动派生：builtin、Nova名称、说明/推荐提示词、backend=nova，默认停用、禁止子代理、继承默认项。生产配置入口测试保存/重读、启用/停用、会话选择及固定身份路由，不另写桌面目录。

## 验证记录

首次红灯（槽位2）：ACP4项3过1注册失败、安装1项白名单缺失失败、vw-agent3项预设/绑定/路由缺失失败，全部退出101。不是编译错误或零测试。

最终所有下列检查退出0；ACP适配和系统内置预设两项交付均通过。

| 检查 | 有效结果 | 日志 |
|---|---|---|
| vw-acp onboarding_nova | 6通过，3项需要本地官方CLI的测试单独执行 | /tmp/shiju-nova-probe/closure-acp.log |
| 真实CLI（官方1.1.65） | 3通过：文件拒绝/只读/放行、两轮、取消后原进程继续、重启load保持会话及所选模型；命令拒绝/只读且无标记文件，拒绝后继续；HTTP401/404明确错误且实际模型请求不回退 | /tmp/shiju-nova-probe/closure-real.log |
| 生产预设→真实CLI | 1通过：生产保存/重读、启停、预设身份与后端路由，真实Nova回包经生产流返回 | /tmp/shiju-nova-probe/delivery-profile_real.log |
| 安装映射 | Unix/Windows 1通过 | /tmp/shiju-nova-probe/final-config.log |
| vw-agent Nova聚焦 | 3通过，真实CLI预设测试另行执行 | /tmp/shiju-nova-probe/delivery-agent.log |
| 注册保护 | 25通过，既有Codex/Claude及此前接入保留 | /tmp/shiju-nova-probe/final-registry.log |
| 内置目录保护 | 18通过，29个唯一ACP后端、原默认启用数6不变 | /tmp/shiju-nova-probe/delivery-catalog.log |
| 子进程启动回归 | 3通过，空命令/缺失进程/参数与环境合并 | /tmp/shiju-nova-probe/closure-process.log |
| workspace Clippy | all-targets/all-features，-D warnings通过 | /tmp/shiju-nova-probe/closure-clippy.log |

最后入口 `python3 scripts/cargo_slot.py python3 /tmp/shiju-nova-probe/closure-checks.py`，槽位4：

- CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4
- CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-4

生产预设、安装、目录与首次Clippy使用槽位2，两目录均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。前序一次槽位3构建退出130、一次重复红灯入口中断退出1，均不计为通过；缓存保留。最终两个入口正常结束，释放各自锁，未清理其他会话资源。

确切原生命令（均置于上述槽位入口内）：

```sh
cargo test -p vw-acp --test onboarding_nova -- --test-threads=1
cargo test -p vw-acp --test onboarding_nova acp_onboarding_nova_real_cli -- --ignored --test-threads=1
cargo test -p vw-config-types --lib acp_onboarding_nova -- --test-threads=1
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_nova -- --test-threads=1
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_nova_real_builtin_profile_persists_and_runs -- --ignored --test-threads=1
cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1
cargo test -p vw-acp --lib client::process_tests::spawn_child -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
```

vw-agent默认优化测试构建持续十余分钟后中止，聚焦命令仅将该包的test opt-level改为0，保持默认features、真实配置/协议/进程实现和断言不变。整仓Clippy保持原命令。`cargo fmt --all`退出0，后续修改块使用`rustfmt --edition 2024`；Python外部模型夹具语法检查通过。未运行全量单元测试、fmt check、wasm check或Makefile命令。

真实测试显式传入 `SHIJU_NOVA_TEST_HOME`（获授权的临时HOME）、`SHIJU_NOVA_TEST_ENTRY`（固定官方包scripts/bin-wrapper.mjs）。真实生产预设测试额外设置 `SHIJU_NOVA_REAL_PROFILE=1`；只替换外部进程/模型边界，配置与目录、租户、会话流均为生产实现。测试夹具使用最小子进程环境和Node `--no-addons`阻断系统钥匙串插件，全部模型请求为回环人工响应，不借用真实账户。

## 最终边界与防回退证据

- 真实CLI选中 `anthropic/claude-haiku-5-5` 后，受控上游每次收到 `model=claude-haiku-5-5`，包含恢复及401/404错误路径；实际生产预设使用CLI本地默认模型，实际请求为 `claude-sonnet-4-6`。没有把虚拟响应当成公网供应商模型质量验收。
- 原始CLI的 `session/set_model` 接受未知ID，但prompt返回selected execution model unavailable、turnOutcome=failed，HTTP请求0。宿主现在提前检查当前模型目录；任意宿主provider/model中继绑定仍保持UnsupportedCli，不能静默回落。真实CLI原始探针：`python3 /tmp/shiju-nova-probe/unknown-model.py`退出0仅代表探针完成，其未知模型推理明确失败。
- 原始CLI owner模式传入HTTP MCP，RPC -32602 `MCP servers are unavailable in owner approval mode`；探针 `python3 /tmp/shiju-nova-probe/mcp-rejection.py`退出0仅代表探针结束。宿主对任何非空MCP选择明确拒绝；不宣称该模式支持stdio/http/sse。
- 新会话必须再次确认owner approval；原生failed回执转换为RPC错误，queued会关闭错误连接，禁止把end_turn/queued冒充完成。load缺少非空历史时明确拒绝供应商的空会话重建回退。
- Nova启动时会在首个RPC前处理项目状态，因此子进程从当前请求工作目录启动；旧供应商仍走原启动路径。真实/协议夹具与生产预设均验证当前目录和会话身份。
- 技能所选挂载保持Unavailable；当前发行版stdio入口丢弃skillScope参数，未通过不安全配置复制伪造支持。用户现有CLI本地配置路径保持原样。
- 默认停用的acp-nova保留模型/权限/技能/MCP默认继承，启停和默认项沿用原配置持久化；选用当前不支持的宿主模型或技能挂载会明确拒绝。启用时应选择CLI本地模型源，MCP显式置空。
- Linux和Windows尚未实机测试，保留平台待回归记录；安装参数数据验证不替代这些平台验收。未修改只读plan.md，也未提交其他任务变更。
