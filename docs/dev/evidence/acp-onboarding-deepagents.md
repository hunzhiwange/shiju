# DeepAgents 接入证据

日期：2026-11-13；实机：macOS arm64。最终验证状态见下表，未将登记或握手代替真实会话。

## 双重交付

- ACP stable key：`deepagents`；由生产 `merged_agent_catalog` 派生官方 `acp-deepagents`，默认停用、不允许子代理，沿用名称、说明、推荐提示词及默认项继承。没有第二份桌面目录。
- 来源启动基线固定为 `npx -y deepagents-acp@0.1.42`。本地候选为 `deepagents-acp`；发现、备用 npx、显式命令覆盖都必须经过审核摘要校验。未审核的 npx 在 spawn 前拒绝，网关安装入口安装独立受管副本。
- 官方目录可搜索、启用、保存并重开配置，固定引擎显示 deepagents，强制会话模型选择不可用。桌面两轮最终结果：当前构建实际窗口连续两轮成功，亮暗主题均观察到两条回复，无错误。
- 生产配置/目录/流入口两轮最终结果：同一会话连续两轮通过，实际模型 HTTP 请求 2 次，均为 gpt-4o。

## 官方来源和安全合同

官方项目为 [langchain-ai/deepagentsjs](https://github.com/langchain-ai/deepagentsjs)，接口来源为 [DeepAgents ACP 参考](https://reference.langchain.com/javascript/deepagents-acp)。官方 npm metadata、`npm pack deepagents-acp@0.1.42 --registry=https://registry.npmjs.org` 确认当前验收的 JS 桥接发行物，bin 为 dist/cli.js。Python 同名产品未替代本入口。

- tar.gz SHA256：`df98cb054af600568086fc91726501480443b3248e73f09cc1a754745a28617e`。
- 原始 CLI SHA256：`6574016c7212831f66c79a2412e7de56c75363293c182fdc311749b8bb251238`。
- 最终修补 CLI SHA256：`d65ca9295356f90c11402e3ec9f39856324329a8a7acbdfe79592444e471abce`。
- 最终 guard SHA256：`b491916bf936fc6691dea6de7eb215ad8101eaf7fa8234b26aaadbd289e88df9`。

官方 0.1.42 在权限 RPC 异常和未知 optionId 时放行，宿主文件调用失败后回退本地文件；历史 0.1.7 同样存在这些路径。复现及包保存在本机 `/tmp/shiju-deepagents-probe`，没有全局安装或读取账号。

修补复用现有 managed_runtime：apply.py 核验固定原始摘要与每个替换上下文的唯一次数；CLI/guard 输出摘要登记在 contracts.json；deepagents-lock.json 固定所有依赖和完整性，安装使用固定官方 registry、npm ci --ignore-scripts。OpenAI/Anthropic provider 分别固定为 1.8.4/1.6.4。OpenAI 显式使用 ChatOpenAI 的 Chat API，避免自动选择 Responses 改变已验证传输。

实际授权在 LangChain wrapToolCall 内、handler 执行前进行。异常、取消、未知选项均拒绝；授权返回后再次检查取消。上游永久授权缓存已移除，每次重新询问宿主。仅开放已验证的 read_file/write_file；继承的 edit_file、ls、glob、grep、execute、task 及未知工具在 handler 前返回明确不可用，避免父类直接访问本地文件。宿主读写异常直接报错，无本地回退。

默认不扫描项目 skills 或 AGENTS.md；工作目录在启动和 new/load 时都比较真实路径；每进程仅一个会话身份，未知会话取消不影响当前会话；非空 MCP 在 new/load 中明确拒绝，不静默忽略。

## 真实 CLI 验证

`scripts/acp_onboarding_deepagents_acceptance.py` 启动实际修补 CLI，仅模型 HTTP 边界使用回环服务和虚拟令牌。子进程临时 HOME/workspace 独立；正常请求到 `/v1/chat/completions`，模型 gpt-4o；错误模型负例保持 invalid-model 并报错。脚本不保存原始消息或密钥。

当前脚本退出 0，日志 `/tmp/shiju-deepagents-probe/acceptance-final.log`：

- 六轮覆盖宿主拒绝、权限 RPC 异常、取消、未知选项、允许工具但拒绝宿主写入、同时允许工具和写入。前五种不产生越权文件；最后真实 write_file 写入临时文件，内容准确为 fixture。
- 真实模型随后请求 edit_file，守卫拒绝，文件保持 fixture，未执行继承的本地编辑路径。
- 非空 MCP 和工作目录扩张明确报错；同一进程的 session/load 成功。
- 活动 prompt 取消返回 cancelled，后续 prompt 可继续。
- 错误认证和错误模型分别收到真实 HTTP 401/404，CLI prompt 报错，没有切换模型或提供商。
- 真实 session/set_mode 和 /status 成功。

`node patches/acp-model-source/deepagents/guard_tests.mjs <审核包目录>` 退出 0：5 类权限结果、5 个取消检查、allow-always 后宿主撤销，以及 7 类未审核工具的 handler 不执行均通过。

握手脱敏摘要：protocolVersion=1；agentInfo.name=deepagents-acp、version=0.0.1；loadSession=true；image/embeddedContext=true、audio=false；MCP http/sse=false；modes/commands=true。上游 CLI --version 实际为 0.1.42，安装身份以包元数据和摘要为准。

## 能力与平台边界

| 项目 | 当前交付与证据 |
| --- | --- |
| CLI 本地模型 | OpenAI --model 与环境认证已由实际请求验证；Anthropic 原生入口保留，未声称本次已验证 Messages 回执。 |
| ShiJu session_model | 本接入未发布中继绑定；既有 UnsupportedCli 门控保持，不能用 CLI 默认模型冒充。 |
| 工具与权限 | 宿主授权下的 read_file/write_file；其他工具明确不可用。未增加 Native 完全访问分支，沿用 ClientOnly/宿主权限；模式名称不构成授权。 |
| 模式/斜杠命令 | 实际验证 agent/plan 切换和 /status；源码还提供 ask、/clear，未将源码存在等同独立验收。未提供 session/set_model/模型目录处理器。 |
| 技能 | 未发布所选技能隔离挂载；默认扫描关闭，显式挂载由既有门控拒绝。 |
| MCP | 上游 CLI 未消费所传 MCP 服务；审核入口明确拒绝非空配置，未发布 stdio/http/sse 支持。 |
| 恢复 | 同一进程 load 已通过；上游仅内存检查点，进程重启后恢复明确失败，不静默创建替代会话。 |
| 平台 | macOS arm64 实机通过基础 CLI；Unix/Windows 安装选择有数据测试。Linux 未实机回归；Windows 受管包装未验证，安装明确拒绝，不能回落未修补全局包。 |

## 原生红绿验证

所有 Cargo 均经 `python3 scripts/cargo_slot.py`。早期红灯槽位 3、主验证槽位 2；它们两个缓存变量分别同为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-3`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。默认 vw-agent lib-test 优化级别 1 的尝试被 SIGKILL，未计测试结果；仅以命令行覆盖本包测试优化级别为 0 后取得有效红绿，不修改仓库 profile。

| 入口内命令 | 结果 |
| --- | --- |
| cargo test -p vw-acp --test onboarding_deepagents -- --test-threads=1 | 红灯注册缺失：2 通过、1 失败；绿灯 4 通过、退出 0，含未审核 npx 拒绝。 |
| cargo test -p vw-config-types --lib acp_onboarding_deepagents -- --test-threads=1 | 红灯安装缺失：1 失败；绿灯 1 通过、退出 0。 |
| cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_deepagents -- --test-threads=1 | 暂时移除注册取得预设缺失红灯：2 失败；恢复后 metadata/binding 2 通过，外部依赖用例显式忽略并单独执行。 |
| cargo test -p vw-acp --lib acp_onboarding_deepagents_reviewed_install_and_discovery -- --ignored --nocapture --test-threads=1 | 当前审核合同下 1 通过、退出 0，安装与发现路径在 /tmp/shiju-deepagents-final-install-20261113。 |
| cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1 | 隔离配置后 24 通过、退出 0；保留其他产品注册。 |
| cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1 | 8 通过、退出 0。 |
| cargo clippy --all-targets --all-features -- -D warnings | 最终变更后退出 0（30.68s），全部目标和特性，warnings 为错误。 |

早期日志 `/tmp/deepagents-red.log`、`/tmp/deepagents-red-lowopt.log`、`/tmp/deepagents-green2.log`、`/tmp/deepagents-final-checks.log`。首次注册保护受本机已有受管 Codex/OpenCode 安装影响，隔离配置后通过，未改其他产品迎合环境。曾修复 /var 与 /private/var 的启动目录比较，随后真实 CLI 与生产首轮通过。原生 lib-test 有既有 __eh_frame 链接容量警告，不冒称 Clippy 失败。已执行 cargo fmt --all 并检查差异，后续只格式化修改文件。

## 两轮问题与最终实机

生产流和桌面均复现“首轮成功，第二轮 session/load 找不到内存会话”的红灯。逐字段 HMAC 指纹证明命令、环境、租户、路径和权限不变，唯一变化是同样 20 项工具的排列顺序；排序后指纹一致。缓存键将工具集合当成有序列表，导致旧 CLI 退役并启动新进程。最终只规范化缓存键中的工具顺序，保留原请求内容，权限集合变化仍得到不同缓存键。已加入独立回归测试与真实生产流同会话两轮测试。运行时和空模型选项的试改未解决问题，均已撤回；临时诊断已移除。

最终原生阶段保持一个前台入口，自动领取槽位 4，CARGO_TARGET_DIR 与 CARGO_BUILD_BUILD_DIR 均为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-4`，日志 `/tmp/deepagents-final-stage.log`。测试命令、退出码和有效数量直接记录在该日志；前台驱动仅用于同阶段复用槽位，未启动脱离进程组的后台任务。早期槽位 5/6 的冷构建及审核摘要更新后的旧二进制拒绝均未计通过，缓存保留。

当前前端经 cargo build -p vw-desktop --bin shi-ju 构建成功（槽位 4），SHA256 `7b02e67df0f19aaf9569f07616441ff98ad1be68b7e17cb20df88230107e5377`。独立应用副本、HOME、桌面配置、网关配置及工作目录位于 `/tmp/shiju-deepagents-desktop-20261113`。窗口夹具使用独立数据库根目录与既有真实传输开关；旧前端和 scripted stream 的初次错误不作为最终验收证据。

已实际观察亮/暗主题官方目录与配置弹窗，成功 toast、保存后重开说明仍保留，固定 backend 正确，中文渲染无 panic。最终当前构建已从官方目录启用、开始会话并连续发送两轮，亮暗主题均显示两条“验收回复”，无 chat.error；网关夹具 1 通过、退出 0，真实模型 HTTP 请求 2 次。服务端 summary 显示 agent=acp-deepagents、acp_agent=deepagents、full_access=false；acp-options 显示 connected=true、cli_local，session_model=unsupported_cli，未切换其他引擎或模型。脱敏身份文件在隔离根目录 identity-passed-final.json。


最终入口与可复现命令（以下命令作为同一个 cargo_slot.py 前台驱动的子命令顺序执行）：

```bash
# 环境：VIBEWINDOW_CONFIG_DIR=/tmp/shiju-deepagents-desktop-20261113/gateway
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_deepagents_cache_key_preserves_tool_set_identity -- --test-threads=1
python3 scripts/acp_onboarding_deepagents_acceptance.py /tmp/shiju-deepagents-final-install-20261113/acp-runtime/deepagents-0.1.42-model-source-v1/node_modules/.bin/deepagents-acp --gateway cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_deepagents_builtin_profile_persists_and_runs -- --ignored --nocapture --test-threads=1
# 窗口夹具另设 VW_DEEPAGENTS_WINDOW_ROOT=/tmp/shiju-deepagents-desktop-20261113；窗口操作结束创建该目录的 stop 文件。
python3 scripts/acp_onboarding_deepagents_acceptance.py /tmp/shiju-deepagents-final-install-20261113/acp-runtime/deepagents-0.1.42-model-source-v1/node_modules/.bin/deepagents-acp --gateway cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_deepagents_desktop_gateway_fixture -- --ignored --nocapture --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib cache_key_is_ -- --test-threads=1
```

缓存集合回归 1 通过；生产两轮 1 通过；桌面夹具 1 通过；Clippy 退出 0。一次旧缓存测试过滤串误写为 session::llm::acp::session::tests::cache_key，实际 0 测试，不计通过，随后用 cache_key_is_ 重跑：有效 2 测试通过，退出 0。用户正常应用与配置未用于最终窗口验收，独立窗口进程验收后退出。
