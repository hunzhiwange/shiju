# Grok Build 接入证据

日期：2026-11-13至2026-11-14；macOS arm64。前置GLM提交9e1d492c5存在；未修改只读plan.md，保留其他任务工作区改动。

## 来源与固定版本

官方来源：[文档](https://docs.x.ai/build/overview)、[源码](https://github.com/xai-org/grok-build)、[npm元数据](https://registry.npmjs.org/@xai-official%2fgrok)。固定参考tar.gz的025迁移已只读提取，入口npx -y @xai-official/grok agent stdio。默认npm镜像返回0.1.4，显式官方源返回latest=1.0.77、alpha=1.0.84；采用1.0.77。官方六平台包为darwin/linux/win32 × arm64/x64；其他平台未经本机验收。

隔离安装：`npm install --prefix /tmp/shiju-grok-probe --ignore-scripts --registry=https://registry.npmjs.org @xai-official/grok@1.0.77`，退出0。审查包内bootstrap后，用临时HOME/GROK_HOME启动，版本`grok 1.0.77 (50ca579fa996)`；未修改全局安装。本机另有1.0.24本地命令，注册优先发现该入口；另用local-version-probe.py对本地1.0.24执行同一隔离环境探针，两轮end_turn、工具写入及load成功，模型请求均fixture-model；分别记录两个已验证版本，不将旧本地版本当成1.0.77的证据。

## 双重交付

注册grok，本地参数agent stdio；备用及安装白名单固定@xai-official/grok@1.0.77。acp-grok通过merged_agent_catalog派生，默认停用、不允许子代理，builtin及backend身份固定；显示名称、说明、推荐提示词及默认项沿用统一生产目录。生产配置及stream入口测试覆盖保存重读、启用/停用、选择和后端身份，外部CLI边界使用stdio对端。没有第二份桌面目录或UI绕过。

会话模型驱动使用Responses协议，0700租户/项目/会话/provider/model/generation私有状态目录；config.toml原子生成、恢复时核对模型绑定并重写受控设置，令牌只进环境，不写argv或文件。GROK_HOME与GROK_DEFAULT_MODEL覆盖只进入子进程；内置grok-4.6/grok-4.5及辅助模型均绑定选定中继，禁止共享leader与CLI子代理。自定义启动参数、环境冲突、不安全目录、非回环端点显式拒绝；Windows状态ACL未验证，session_model拒绝。CLI本地模式保留官方本地配置。

## 真实1.0.77协议

原始协议、临时服务及虚拟令牌仅在本机/tmp/shiju-grok-probe，不提交原始载荷。测试使用隔离目录、官方真实二进制和本地受控模型服务；不等于公网账号验收。

- initialize protocolVersion=1，版本在_meta.agentVersion；loadSession=true，http/sse MCP及resume/list/close为上游声明。无凭据session/new返回-32000 Authentication required。
- driver-contract.log：按生成驱动的完整配置（shiju-session、两个内置模型覆盖、关闭共享leader/自动升级/CLI子代理）及GROK_DEFAULT_MODEL执行，两轮end_turn、工具写入及load成功；每次请求断言Authorization匹配指定虚拟env_key，另一个虚拟XAI_API_KEY未被采用，只记录布尔结果。CLI回写后config.toml仍0600。
- env-model.log：不传--model，以GROK_DEFAULT_MODEL与私有配置选择fixture。两轮end_turn，实际所有已观察模型请求fixture-model；session/load保持fixture模型。初期辅助grok-4.6请求已通过带引号的TOML模型覆盖解决，早期未隔离日志不作通过证据。
- write-final.log与env-model.log：真实write工具触发权限请求；选上游allow_once后fs/write_text_file写入临时tool-output.txt，内容fixture；两轮完成并load。临时HTTP上游仅替代模型网络，不替代供应商工具实现。
- denial-final.log：权限cancelled，首轮cancelled，fs/write_text_file零调用；下一轮end_turn。cancel-final.log：活动请求session/cancel，首轮cancelled，下一轮end_turn并load。
- auth-final.log/model-error-final.log：受控401/404均RPC -32603，分别含Unauthorized/API error；没有静默回退默认模型。缺失可执行文件另有原生客户端测试。
- mcp-final.log：仅selected HTTP服务收到server/discover、initialize、notifications/initialized、tools/list；stdio-final.log覆盖同一个所选stdio对端。mcp-scope.log与mcp-global-scope.log分别加入项目和临时用户配置的unselected stdio fixture，new/load过程中启动标记均未生成，selected HTTP状态ready；项目fixture为未受信目录，不冒充受信项目的覆盖。SSE探针配置传统GET事件端点及POST消息端点，实际客户端以StreamableHttpClientWorker向SSE端点POST initialize，收到405并报告handshake_failed；没有获得传统SSE成功证据，不把声明当成通过。
- 模型目录/配置选项与恢复中currentModelId=fixture；额外reasoning_effort选项由上游回读，未赋予宿主新权限。
- 技能：官方当前源码文档声明skills.paths及Claude/Codex/Cursor兼容扫描；实际inspect --json在显式skills.paths后发现selected fixture（configToml来源），未指定路径时该fixture未被发现；尚未证明仅所选技能的隔离，宿主维持明确拒绝，不伪称上游不支持技能。
- 权限保持通用ClientOnly/Unavailable，不把always-approve或上游模式名称视为宿主授权，未增加Native权限分支。

## 原生验证

红灯阶段槽位6：ACP4项，3通过/1缺失注册失败，退出101；安装1项缺失映射失败，退出101；vw-agent3项因缺失预设/绑定失败，退出101。vw-agent使用`--config profile.test.package.vw-agent.opt-level=0`降低本crate首次编译开销，仍是原生测试。此前中断的编译退出130不算测试证据。

首轮通过阶段：安装1/1、vw-agent4/4退出0；ACP集成3通过/1因已安装本地grok而错误假设npx失败，已将本地发现和固定备用契约分开验证。周边注册首轮16通过/9失败，首个错误为已安装审核OpenCode入口优先于OPENCODE_BIN fixture，Codex同样被审核运行时接管，后续环境锁中毒；计划用测试进程专用空VIBEWINDOW_CONFIG_DIR复验，不修改无关生产逻辑。最终结果见下表。红灯CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为/Users/xiangmin/worker/code/shiju/target/rust-slot-6；通过阶段自动领取槽位3，两目录均为/Users/xiangmin/worker/code/shiju/target/rust-slot-3。fmt阶段另领取槽位7，无构建产物承诺。缓存保留，入口退出后自动释放锁。


## 最终原生复验与待回归原因

2026-11-14，统一入口 `python3 scripts/cargo_slot.py python3 /tmp/shiju-grok-probe/final-checks.py`，实际槽位3；CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`。逐项执行并记录独立退出码，收集器自身退出0不代表全部检查通过：

|入口内部命令|有效测试数|退出码|
|---|---:|---:|
|`cargo test -p vw-acp --test onboarding_grok -- --test-threads=1`|4通过|0|
|`cargo test -p vw-config-types --lib acp_onboarding_grok -- --test-threads=1`|1通过|0|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_grok -- --test-threads=1`|5通过|0|
|`cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1`，仅本测试进程设置空的`VIBEWINDOW_CONFIG_DIR=/tmp/shiju-grok-probe/registry-config`|25通过|0|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1`|16通过|0|
|`cargo clippy --all-targets --all-features -- -D warnings`|非测试|101|

最终日志final-acp/install/agent/registry/catalog/clippy.log与final-status.json位于上述私有临时目录。恢复测试验证CLI添加的UI/yolo设置被生成配置替换、模型绑定被篡改则拒绝；模型配置覆盖和私有环境冲突测试均实际执行。两处目录计数已从27更新为28。注册回归的环境污染通过独立空审核运行时目录消除，没有修改Codex/OpenCode逻辑或用户安装。

Clippy两轮均因与本次Grok无关的编辑器API编译失败：`crates/vw-desktop/src/app/components/preview_panel/view/editor_options.rs:34`调用缺失的`wrap_column`，`crates/vw-desktop/src/app/message/editor.rs:276`调用缺失的`set_wrap_column`；编辑器测试153/154/159/163/166/171/176行同类错误，库测试共9条诊断。当前iced-code-editor来源为Cargo Git checkout `64c5895`，其可见API提示为`with_wrap_column`。这些文件及Cargo.lock不属于本次变更，按仓库要求不修复无关编辑器任务。

结论：Grok ACP、内置预设、会话模型驱动及全部规定聚焦/周边测试已完成；整仓Clippy尚未通过，任务状态为待回归，保留编号任务文件。下一步先由编辑器任务修复依赖/API一致性，再通过cargo_slot入口重跑整仓Clippy；其他OS、传统SSE、完整生产网关公网账号/Rue.js产物验收仍属于已说明的独立覆盖边界，不伪称已通过。
