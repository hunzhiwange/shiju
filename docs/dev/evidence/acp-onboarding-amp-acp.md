# Amp ACP 受管本地运行时证据

## 2026-11-14 再次回归

执行 `python3 scripts/cargo_slot.py python3 scripts/acp_onboarding_amp_acp_acceptance.py --runtime /tmp/shiju-amp-probe/ready --mode preset`，外层退出 1，内部 Cargo 编译退出 101。槽位 2，两个缓存变量均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。

当前仍被 `crates/vw-gateway-client/src/client/agent_apps_skills/repos_api.rs:7` 的 E0599 阻断：`ProjectId` 未实现 Display，不能调用 `to_string()`。有效测试数为 0，模型请求数为 0，不计通过；未继续使用旧测试程序运行 session-model。日志在 `/tmp/shiju-amp-probe/recheck-preset.log`。已确认本轮 Cargo/rustc 退出、槽位锁释放。未改无关模块，生产预设与模型中继仍待回归。

实机：macOS arm64。目标是使用会话选中的模型，无需 Amp 账号。本文所述产物是固定官方版本的本地补丁构建，未覆盖用户的 CLI 或全局配置。

## 来源与路线

- ACP 官方目录：https://github.com/agentclientprotocol/registry/blob/main/amp-acp/agent.json 。桥接器官方源码：https://github.com/tao12345666333/amp-acp/tree/v0.12.0 。固定 npm 包 `amp-acp@0.12.0`，不是 `@edlsh/amp-acp` 分叉。
- 官方 CLI npm 元数据：https://registry.npmjs.org/@sourcegraph%2famp 。固定 `@sourcegraph/amp@0.0.1778732631-gc5875c`，Node >=20；这是保留本地 ThreadWorker 的历史 JS 发行版，不能冒充当前原生 CLI。
- CLI 包 SRI：`sha512-+DY8uzhwq7i9BlSwfTwIDaPX6UdsTocOjdP2unf3TOEkO8WV6vP5I/xYklJHCrcyX7wMUhwwLJCCVH+wX4AvnQ==`。原始 `dist/main.js` SHA256：`458435712c9c4e222822847fc9e70080cdf023ffb3c820e959a30425a22a5836`。
- 受管输入、输出文件摘要见 `crates/vw-acp/src/managed_runtime/contracts.json`。安装核对固定包身份/SRI、原始 CLI 文件摘要和补丁唯一匹配，完成后逐一回读两个包的执行文件及本地控制模块摘要。运行时再次验证这些摘要及引擎版本，拒绝未修补本地程序和任意启动参数。
- 上游 CLI 的许可证为 Amp Commercial License，安装保留其 LICENSE；桥接器为 Apache-2.0。仓库只保存补丁和自身适配代码，不提交官方下载包或 bundle。

当前原生 CLI `0.0.1794614453-g43b36a` 及桥接器文档提及的 `0.0.1793433646-gfaad62` 默认路径依赖 `/api/thread-actors`，官方架构说明：https://ampcode.com/security 。单独更换模型 URL 不能使这些路径脱离 Amp Server。历史 JS 包仍包含 `--take-me-back` 对应的真实本地 ThreadWorker、推理客户端和工具编排；补丁固定选择此分支，保留原代理循环。没有替换为 ShiJu 或另一个代理引擎。

参考项目 `new-shiju-agent` 的 025/033 迁移仅登记 Amp 与 `amp-acp --setup` 认证；其 `cli-models/aioncore.patch` 只允许 Claude/Codex/Qwen/OpenCode，未提供 Amp 无账号模型通道。

## 运行边界

- `amp-acp` 派生官方预设 `acp-amp-acp`；目录、配置保存和会话路由沿用现有生产入口。默认停用、禁止子代理，模型/权限/技能/MCP 默认项继承既有规则。
- 本产物仅提供 `session_model`，不公布 CLI 本地账号源。所有主推理及标题请求经私有回环控制桥转入现有模型中继，统一绑定所选模型。控制桥拒绝重定向，原生引擎的 fetch 只允许该控制桥来源；这不是操作系统网络沙箱声明。
- 本地身份仅用于自身控制接口，不发送 Amp 服务、不伪造远程账号。删除桥接器启动时读取原账号、原生程序修复、SDK/Orb 自动选择和 setup 入口。子进程不继承其它供应商凭据、Node 预加载或 shell 启动脚本；只获得本会话控制令牌。模型中继令牌留在桥接进程中。
- 会话状态目录 0700、原子写入的线程文件 0600；保存模型及不含令牌的绑定引用。恢复校验工作目录、模型和绑定引用，控制令牌与 ACP 会话及原生线程固定对应。流式完整回复发布前等待线程落盘，避免退出后恢复到旧状态。
- 原生工具执行入口在 hooks、参数预处理和本地工具正文之前转交宿主；无响应、拒绝、错误均失败，不运行本地后备。Read/create_file/edit_file/Bash 通过宿主文件或终端接口执行。Read 仅支持 UTF-8 文本和有限行范围；编辑要求唯一匹配；目录/二进制读取、后台及未接通工具明确拒绝。
- 关闭环境技能、项目/全局插件、MCP、指导文件扫描、`@文件` 内容展开、文件历史追踪/备份恢复与原生 git 环境采集。不得将这些原本支持的能力描述为“上游不支持”：受管运行时尚未验证所选技能/MCP 的隔离，故明确不可用。
- 历史引擎的输出上限按所选模型限额收敛，不支持的 temperature/thinking 不发送；移除旧 Opus 的默认 effort。保留现有模型中继的能力校验，不因旧引擎默认值放宽中继。
- 原生模型/权限/Orb/思考选项与隐式继续最近线程均关闭；宿主权限不能被 bypass 模式提升。模型错误和认证错误返回失败，修复了上游桥接器把非认证执行错误返回为正常结束的行为。
- MCP stdio/http/sse、技能与扩展原生工具不在本次可用范围；非空所选 MCP 在启动会话前拒绝。取消终止原生进程与进行中的模型请求；已创建的宿主终端在取消时请求终止并释放。

## 已取得的验证

红灯：ACP 目标 3 项中 2 通过/1 因未注册失败；安装 1 项因无映射失败；内置元数据、绑定、生产预设 3 项均因预设缺失失败。各红灯退出 101，未把初次配置测试的缺少 import 编译错误计作红灯。

直接原生探针使用 `crates/vw-acp/tests/fixtures/amp_acp_real_probe.py`，只模拟模型与宿主协议边界，不替换真实桥接器或 CLI。已验证同一线程多轮、真实宿主写入、拒绝无落盘、重启 load、取消后继续、错误认证/模型失败，以及宿主读取内容与磁盘哨兵区分。模型请求只记录路径、模型及认证匹配布尔值；没有在本文保存请求正文或凭据。

新增生产预设原生验收用例需要已安装受审运行时和受控模型服务，使用 `scripts/acp_onboarding_amp_acp_acceptance.py --mode preset` 提供前提并实际执行 ignored 用例；不能把普通聚焦输出中的 ignored 计作通过。`--mode session-model --test-binary <当前槽位测试程序>` 验证真正的生产模型中继、取消/恢复及换会话模型。

本机未测试其它 OS/架构。安装数据选择不等于跨平台实机通过；受管安装对未验平台明确报错。任务最终结果应以追加的当前原生验证记录为准。

## 2026-11-14 当前收尾验证

本次通过 `python3 scripts/cargo_slot.py python3 /tmp/shiju-amp-probe/verify-proof.py` 顺序执行。入口领取槽位 2，`CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；此前开发使用过槽位 3、6，两个变量同样指向对应绝对目录。均由入口管理锁和退出清理，没有手工占用他人槽位。

- 使用仓库 `install.py` 与当前 contracts/patch assets 全新安装 `/tmp/shiju-amp-probe/ready`，退出 0；输出文件摘要校验全部通过，未复用旧补丁产物。
- `cargo test -p vw-acp --test onboarding_amp_acp -- --test-threads=1`：4 通过，0 失败。
- `cargo test -p vw-config-types --lib acp_onboarding_amp_acp -- --test-threads=1`：1 通过，0 失败。
- 真实桥接器及 CLI：write/read/edit/command 的允许、拒绝共 8 组全部退出 0；每组 4 个完成回合，含进程重启 load/history、取消后继续、401/404 失败。edit 额外检查 `$&` 为字面替换；read 检查模型只收到宿主标记，未收到磁盘哨兵。原始日志 `/tmp/shiju-amp-probe/proof-native-*.log` 仅作为本机运行记录。
- 生产预设 `python3 scripts/acp_onboarding_amp_acp_acceptance.py --runtime /tmp/shiju-amp-probe/ready --mode preset`：未执行测试。`vw-gateway-client/src/client/agent_apps_skills/repos_api.rs:7` 的无关新增代码对 `ProjectId` 调用 `to_string()`，产生 E0599；同轮还有无关技能 API 编译错误。未修改这些模块。
- 当前 lib-test 无法构建，因此自动挑选的既有 `vw_agent-d5b8f137b7c9d9e7` 不能证明当前补丁的生产集成；其 session-model 探针失败、模型请求为 0，明确不计通过。此前 production receipt/默认推理预算的失败亦不计完成。
- 新增内置预设、绑定和生产验收用例尚需在当前源码编译恢复后真正执行。原先目录保护 22 项通过不替代本轮编译；本轮 catalog 保护被同样的无关依赖错误阻断。

结论：ACP 原生工具与生命周期补丁已取得实测；内置智能体生产链路仍待回归。仅在 macOS arm64 验证，不声明其它平台通过，也不声明真实云模型质量或性能。

补充检查：`cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1` 在空配置目录下 25 通过；Node 模块语法检查和 3 个 Python AST 检查通过，`git diff --check` 无输出。受影响包 Clippy 与要求的 `cargo clippy --all-targets --all-features -- -D warnings` 均退出 101，阻断于未修改模块：content-guard 的 collapsible_if/should_implement_trait，memory-algorithms 的 chunks_exact_to_as_chunks/module_inception，domain-matcher 的 module_inception。不能据此声明新增 vw-agent 模块已通过 Clippy；修复无关问题后仍需回归。
