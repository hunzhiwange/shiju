# GLM Agent 接入证据

日期：2026-11-13；实机 macOS arm64。批次4提交 `28d1d6311` 及 Dirac 证据已核验，代码依赖满足。保留工作区其他任务变更；未修改只读 plan.md。

## 来源与版本

官方 [npm 元数据](https://registry.npmjs.org/glm-acp-agent)、[源码与说明](https://github.com/stefandevo/glm-acp-agent)。`npm view glm-acp-agent version bin repository --json` 返回1.17.1、dist/index.js、stefandevo/glm-acp-agent。官方要求 Node20.19+/22.13+/24+。本次使用本机 Node，未改全局安装。

固定来源 tar.gz 中025迁移已只读核验：key=glm-acp-agent，binary同名，npx -y glm-acp-agent；没有复制 bypass_permissions 默认授权。

隔离安装命令：`npm install --prefix /tmp/shiju-glm-probe --ignore-scripts --registry=https://registry.npmjs.org glm-acp-agent@1.17.1`，退出0。发行物 dist/index.js SHA256=`d0d9237bffcbd5017a0085aa3c4625c29e2b750f87c358fa9c455d3cd4cdbd98`。

## 双重交付

注册本地命令 `glm-acp-agent`、备用 `npx -y glm-acp-agent@1.17.1`；安装白名单同版，Unix/Windows 数据测试。其他OS未实机运行。

`acp-glm-acp-agent` 从生产 merged_agent_catalog 派生：builtin=true、backend=glm-acp-agent、默认停用、不允许子代理，名称、说明、推荐提示词和默认项沿用统一目录。新增生产配置/流入口测试验证启用、保存重读、身份固定、正确后端、停用拒绝，只有外部进程使用受控 stdio 对端。桌面官方目录与会话选择沿用现有网关派生链路，无第二份手写列表或UI改动。

会话模型使用已验证的 ACP_GLM_MODEL/AVAILABLE_MODELS/BASE_URL 与 Z_AI_API_KEY 覆盖；只允许已审查参数、工具模型及回环中继。状态按租户/项目/会话/provider/model/generation隔离，目录0700并核验用户和非符号链接；拒绝环境冲突，令牌不进入argv或配置文件。非Unix私有目录ACL未验证，显式拒绝。CLI本地模式保留其官方本地配置。

## 真实 CLI 协议与能力

临时 HOME、XDG_CONFIG_HOME、ACP_GLM_SESSION_DIR、工作目录 `/tmp/shiju-glm-probe`；只用虚拟令牌及受控本地模型HTTP服务，没有借用账号或修改全局配置。临时 probe.py、mcp-probe.py、stdio-probe.py、sse-probe.py 分别运行固定发行物。

- initialize：agentInfo={name:glm-acp-agent,version:1.17.1}，protocolVersion=1，loadSession=true，image/embeddedContext=true，http MCP=true，session resume/list/close/fork已声明。声明不等于全部实测。
- two-turns.log：同会话两轮 end_turn；HTTP `/v1/chat/completions` 实际 model=fixture-model；同进程 session/load 成功并回读相同模型和default权限。未声称跨进程恢复或公网账号验收。
- write-final.log：write_file请求 session/request_permission，选择上游真实 optionId=allow 后触发 fs/write_text_file、返回success并写入fixture。早期write.log用了其他供应商的allow_once，实际被拒绝，修正后重跑，不作为写入成功证据。
- denial.log：取消权限请求，tool_call status=failed/Cancelled by user；无写入。exec-denial.log同样拒绝run_command且目标文件未生成。
- cancel.log：活动轮cancelled，下一轮end_turn，随后load成功。
- auth.log/model.log：受控上游401/404分别返回RPC -32603，details=401/404 fixture failure；没有默认模型回退。
- mcp.log：仅所选HTTP服务initialize/tools/list，fixture_mcp到达实际模型tools，load后重连；stdio.log的fixture_stdio同样到达模型tools。sse.log：new直接返回明确“SSE transport ... not supported yet”。
- MCP子进程继承代理环境是上游已声明行为；本次只使用受控服务及虚拟令牌，不宣称供应商进程隔离。
- 技能：临时 .claude/skills/selected/SKILL.md 被发现并通过 available_commands_update 公布selected；源码dist/protocol/slash-commands.js合并cwd和HOME的.claude目录。未证明仅选中技能的隔离入口，宿主保留明确拒绝；不是声称上游不支持技能。
- 权限：new回读default，另有accept_edits/bypass_permissions；不将名称视为宿主授权。保留通用ClientOnly/Unavailable判定，不新增未经完整固定运行时证明的Native授权分支。

生产网关到真实CLI及公网模型的完整用户配置验收、其他平台、跨进程resume仍需独立26号验收任务；本次生产路由与真实CLI边界分别验证，不冒称额外实机覆盖。

## 原生验证

红灯、通过阶段均使用 `python3 scripts/cargo_slot.py`，实际槽位2；CARGO_TARGET_DIR和CARGO_BUILD_BUILD_DIR均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，入口结束自动释放锁并保留缓存。

红灯 red.log：ACP退出101，2通过/1注册失败；安装退出101，0通过/1映射失败；vw-agent退出101，0通过/3失败（目录元数据、绑定、生产持久化路由）。

通过 green.log：ACP3/3、安装1/1、vw-agent4/4均退出0；整仓Clippy退出0。vw-agent包含模型私有目录/环境冲突拒绝测试。

命令：`cargo test -p vw-acp --test onboarding_glm_acp_agent -- --test-threads=1`；`cargo test -p vw-config-types --lib acp_onboarding_glm_acp_agent -- --test-threads=1`；`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_glm_acp_agent -- --test-threads=1`。仅沿用前批次vw-agent命令行测试优化调整，未修改features或仓库profile。

格式化限本次文件，使用rustfmt --edition 2024 --config skip_children=true，避免覆盖其他任务。未执行全量单测、fmt check、wasm check或Makefile。

最终final.log：ACP4/4、安装1/1、智能体4/4、注册保护24/24均退出0；目录保护首次12通过/2失败，原有总数26需随本次接入更新为27，已修复并重跑。resume.log额外验证session/set_model成功及同进程session/resume回读fixture-model/default。

最终regression.log：智能体聚焦4/4、完整目录14/14、Clippy均退出0；命令为同槽位入口中的 `cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_glm_acp_agent -- --test-threads=1`、`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1`、`cargo clippy --all-targets --all-features -- -D warnings`。所有阶段实际使用槽位2，进程已结束、锁自动释放，缓存保留。
