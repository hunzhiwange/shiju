# DimCode 接入证据

日期：2026-11-13；实机 CLI 为 macOS arm64，固定 npm 发行物 `dimcode@0.5.28`。

## 交付

- stable backend `dimcode`；本地发现命令 `dim acp`，备用启动 `npx -y dimcode@0.5.28 acp`，安装白名单固定同版本。Unix/Windows 安装选择有数据测试；其他系统未声称实机通过。
- 生产 `merged_agent_catalog` 派生 `acp-dimcode`，builtin=true、backend=dimcode，默认停用、不允许子代理，沿用名称、说明、推荐提示词及模型/权限/技能/MCP 默认项继承。官方目录、启用、配置保存重读和会话路由共用既有实现。
- 注册、安装、目录绑定、生产配置保存/重读、启用/停用与流入口路由均有聚焦测试。生产流只替换外部 ACP stdio 对端，不替换配置或目录实现。
- 当前协议拒绝 DimCode 的 stdio MCP 配置，覆盖 new/load/resume，避免上游仅警告后静默忽略；HTTP/SSE 使用现有传输。生产流夹具为 DimCode 选择 HTTP MCP，其他后端夹具保持原样。

## 官方发行物与协议

来源：[npm](https://www.npmjs.com/package/dimcode)、[官方 registry](https://registry.npmjs.org/dimcode)、[官方 ACP 文档入口](https://www.dimcode.dev/en/docs/acp/)。当前文档入口重定向到营销页面，功能结论以固定发行物的实际协议和内嵌源码为依据。固定参考快照 025 迁移的 binary_name=dim、npx argv 已只读核对。

隔离安装命令：`npm install --prefix /tmp/shiju-dimcode-probe --ignore-scripts --registry=https://registry.npmjs.org dimcode@0.5.28`，退出 0。实际 bin 同时提供 dim/dimcode，`dim --version` 为 0.5.28。Darwin arm64 原生文件 SHA256：`0d6298cc8881b15ae8cdf1901b6f39674b74dbf5c14caedadfe509ab5ae114b0`。

脱敏 initialize：protocolVersion=1；agentInfo.name=dimcode、version=0.5.28；loadSession=true；image/embeddedContext=true、audio=false；MCP http/sse=true。声明能力与实际验证分别记录，不由握手推导全部功能可用。

## 真实 CLI 结果

全部使用临时 HOME/workspace 和回环模型服务的虚拟令牌，没有使用真实模型账号或修改官方全局配置。原始夹具和日志位于 `/tmp/shiju-dimcode-probe`，未提交原始消息、凭据或供应商二进制。

- `two-turns.log`：同一会话两轮均 end_turn，同进程 session/load 成功；实际模型请求 `/v1/responses`、model=gpt-4o。
- `cancel.log`：活动轮次 cancelled，下一轮仍 end_turn。
- `host-denial.log`：真实 fs/write_text_file 回调被宿主拒绝，不产生目标文件；`write-allowed.log`：宿主明确允许后临时文件内容为 fixture。
- `exec-denial.log`：真实 session/request_permission 返回 cancelled，未生成命令写入目标文件。
- `auth-failure.log`、`model-failure.log`：回环上游 HTTP401/404 导致 refusal；404 明确为 PROVIDER_MODEL_NOT_FOUND，未切换模型。404 是受控服务拒绝 gpt-4o，不声称已验证 CLI 接受未知模型选项。
- `mcp-stdio.log`：上游明确警告 “ACP MCP stdio transport is unsupported”，测试服务没有启动。发行物内嵌 src/acp/mcp-bridge.ts 同样明确拒绝 stdio，因此 ShiJu 在协议调用前返回错误。
- `mcp-http.log`：真实 HTTP MCP 完成 initialize、notifications/initialized、tools/list、tools/call，返回夹具工具结果；仅传入 selected 服务。

## 能力边界

| 能力 | 当前结论 |
| --- | --- |
| CLI 本地模型 | 已验证 provider 配置和实际 gpt-4o 请求；保留 CLI 本地配置路径。 |
| ShiJu session_model | 未开放，既有 UnsupportedCli 门控拒绝。上游 set_model/模型配置会调用 providerManager.switchProvider 并写入 cwd 默认选择，不据此声称已经完成会话隔离中继。 |
| 权限 | 保持 ClientOnly/宿主策略，不发布 Native 完全访问支持。上游 read-only/workspace-write/full-access 名称不构成宿主授权。 |
| 技能 | 未发布所选技能隔离挂载，既有门控明确拒绝选中技能。内嵌源码 discoverHostUserSkills 会合并注册插件、市场和用户目录，尚无已交付的隔离分支；不将此写成“上游不支持技能”。 |
| MCP | stdio 已明确拒绝；HTTP 实测工具调用成功；SSE 有上游源码和握手声明，沿用通用传输，本轮未实测 SSE 服务。 |
| 恢复 | 同进程 load、取消后继续已实测；跨进程 resume 未另行验收。 |
| 平台 | macOS arm64 CLI 实测，Unix/Windows 安装契约数据测试；其他平台未实测。 |

## 红绿与原生检查

所有 Cargo 经 `python3 scripts/cargo_slot.py`，未手工占锁或修改 target。早期红灯槽位 3；其余检查及恢复阶段槽位 2。CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 同为对应的绝对目录：

- `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`
- `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`

vw-agent lib-test 沿用前一批已验证的命令行 `--config profile.test.package.vw-agent.opt-level=0`，features 仍为默认，没有修改仓库 profile。

| 命令（槽位入口内部） | 已取得结果 |
| --- | --- |
| cargo test -p vw-acp --test onboarding_dimcode -- --test-threads=1 | 注册红灯 2 通过/1 失败；初始绿灯 3 通过；新增 stdio 拒绝先 0/1 红灯，修复后 4/4 通过、退出 0。 |
| cargo test -p vw-config-types --lib acp_onboarding_dimcode -- --test-threads=1 | 缺安装映射红灯 0/1；绿灯 1/1、退出 0。 |
| cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_dimcode -- --test-threads=1 | metadata/binding 缺预设红灯 0/2；生产持久化/路由另有 0/1 红灯；初始实现最终 3/3 通过、退出 0。 |
| cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1 | 隔离 VIBEWINDOW_CONFIG_DIR 后 24/24、退出 0。 |
| cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1 | 总数更新为 25 后 10/10、退出 0，默认启用仍为 6。 |
| cargo clippy --all-targets --all-features -- -D warnings | 初始接入实现退出 0，6m19s。 |
| cargo build -p vw-desktop --bin shi-ju | 恢复阶段退出 0，2m16s。 |

初始日志 red.log、red-profile.log、green.log、final.log 位于 `/tmp/shiju-dimcode-probe`；stdio 红绿及恢复构建日志位于 `/tmp/shiju-dimcode-stage`。本机既有受管 Codex/OpenCode 导致首次注册保护失败，隔离配置后通过；未修改其他后端迎合环境。

## 用户调整与未执行项

用户明确取消独立窗口验证及额外验证后，已停止本会话创建的独立窗口和临时网关，删除新增窗口夹具、模块连线和窗口回环脚本；同步移除计划中的人工桌面、明暗主题验收门槛。窗口尝试只观察到目录启用与配置保存重开，未完成真实桌面两轮，不能宣称已通过。没有修改用户正常应用或其他任务的窗口夹具。

按用户最新要求不再追加窗口或其他验证。stdio 修复后的 ACP 4 项已经通过；HTTP MCP 生产流夹具调整后未重跑 vw-agent 和 Clippy，以上历史通过结果不冒称为这次调整后的新结果。SSE、技能隔离、跨进程恢复和其他 OS 的边界保留为未实测，不制造支持或失败结论。
