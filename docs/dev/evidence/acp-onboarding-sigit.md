# siGit Code ACP 接入证据

日期：2026-11-14。真实平台：macOS arm64。固定官方源码1.7.10，不修改原CLI安装或全局配置。Linux/Windows仅安装参数数据测试，不代表实机通过；受审源码安装目前明确限制macOS arm64。

## 来源和安全适配

- 官方源码：https://github.com/getsigit/sigit/tree/v1.7.10
- 官方发行：https://github.com/getsigit/sigit/releases/tag/v1.7.10
- 许可证：固定源码包的LICENSE为Apache-2.0，NOTICE保留上游版权与商标声明；来源 https://github.com/getsigit/sigit/blob/v1.7.10/LICENSE 与 https://github.com/getsigit/sigit/blob/v1.7.10/NOTICE 。本产物为经修改的本地受管构建，不冒充未修改官方发行物；agentInfo的1.7.10仅表示上游基线，源码/补丁/二进制摘要另行区分衍生产物。再分发衍生产物时保留上游LICENSE与NOTICE。
- npm元数据：https://registry.npmjs.org/@smbcloud%2fsigit/1.7.10 ，新scope `@getsigit/sigit`与旧scope均为1.7.10；保留任务要求的稳定key=sigit。
- 固定参考025迁移只读核验：`npx -y @smbcloud/sigit`。当前发行需显式`--acp`；注册固定参数 `-y @smbcloud/sigit@1.7.10 --acp`，本地参数`--acp`。
- 官方源码归档SHA256：`088809cfc43ceda419913488390a65e830ac8ff66f1b5dd14a8d833d6b46369e`。受审补丁及校验契约见`patches/acp-model-source/sigit/host-boundary.patch`与`crates/vw-acp/src/managed_runtime/contracts.json`。

未修补1.7.10在宿主拒绝`fs/read_text_file`后回退本地读盘，人工哨兵随后出现在回环模型请求；默认deny和Plan模式均不能修复。静态沙箱对受限文件有效，但为CLI状态路径放行又会把相同读取能力授予模型工具；逐路径read_file权限规则在上游不匹配。不能把协议拒绝或HOME隔离等同内核隔离。

因此复用已有native managed runtime安装方式，从固定源码构建独立受审CLI：

- 每次工具调用均重新询问宿主，保留CLI显式拒绝；本地allow、会话allow缓存和模式名称不能扩大宿主权限。
- 文件回调拒绝、错误、超时均返回失败，不回退磁盘；终端创建失败不执行本地命令。
- 仅公布有宿主路由的read_file/create_file/edit_file/multi_edit/run_command。根外、无客户端能力、后台命令和其他原生工具明确失败；不悄然替换引擎或工具实现。
- 移除授权前文件预览、环境中的会话/工具hooks、自动项目指令加载、自动技能/子代理工具发现及环境MCP发现；终端命令结束后不再绕过宿主自动修改Git提交。
- 进程启动强制校验源码/补丁/二进制收据并要求argv恰为`--acp`。未修补本地CLI、npx后备入口、不同版本或headless参数不能绕过此边界。
- 原CLI保持原位置；注册/安装/会话都沿用已有目录和网关入口，没有第二份桌面目录。

源码包中仅跳过三项已核对的文档符号链接，不提取任意链接。使用上游支持的`MISTRALRS_METAL_PRECOMPILE=0`绕过缺失Metal构建工具，保留运行时Metal路径；没有修改依赖或关闭安全检查。受审源码构建仅保留上游debug二进制过大产生的ld unwind诊断，禁用功能留下的Rust dead_code/unused警告已明确处理。仓库Clippy单独按原命令验证。

## 真实能力边界

| 项目 | 实证结果 |
|---|---|
| 身份 | initialize.agentInfo.name=sigit、version=1.7.10；包元数据与源码校验共同确定版本，不依赖历史skip_version_probe |
| 基本会话 | 同一session两轮prompt；取消后原进程继续；重启load及resume后仍请求同一个选定模型 |
| 文件与命令 | 宿主拒绝读/写及terminal/create均不触发本地替代；授权读取使用宿主内容，授权写入真实临时文件 |
| 模型 | CLI本地源与SessionModel；私有loopback relay通过OPENAI_BASE_URL/OPENAI_API_KEY/SIGIT_MODEL绑定，真实请求到选定provider/model，配置不含真实凭据 |
| 模型生命周期 | 同会话第二轮、取消、重启恢复保持绑定；已启动会话拒绝替换模型；新会话选择第二模型成功；401/404及未知目录模型不回退 |
| 权限 | 通用ClientOnly/Unavailable等既有判定；不宣称Native完全访问。原生Manual/Auto/Plan名称不是宿主授权 |
| MCP | 上游声明HTTP、无SSE；受审host-only运行时尚未接入所选MCP工具，new/load前非空选择明确拒绝，环境MCP不连接；不冒称上游不支持MCP |
| 技能与子代理 | 上游扫描.sigit/.claude及全局技能；尚无选定目录隔离验收，宿主保持Unavailable，自动技能/子代理发现关闭，不悄然挂载全部目录 |
| 平台 | 实机macOS arm64；其他平台保留待回归，安装数据测试不作实机证据 |

本地模型服务仅返回人工OpenAI兼容SSE，无真实账户、用户对话或密钥。原始HTTP载荷不进入本文；证据只记录模型名、端点、认证是否匹配、工具结果数量和布尔结果。

## ACP 与内置预设

`sigit`由注册表派生`acp-sigit`：builtin、名称siGit Code、说明/推荐提示词、backend=sigit，默认停用、默认禁止子代理、继承模型/权限/技能/MCP默认项。配置验证、官方目录、启用管理和会话选择沿用merged_agent_catalog与既有桌面状态入口。

生产预设验收覆盖保存/重读、启用/停用、固定身份、后端路由及默认拒绝。常规测试只替代外部进程边界；另有显式原生预设验收，通过真实受审CLI和回环模型服务验证ACP_ACCEPTANCE_RULE到达实际请求并返回生产chat.done。

发现并修复必要相邻依赖：`vwacp.desired_config_options`的键是外部协议ID，不是本地字段名；原校验拒绝sigit-model/sigit-permission-mode。只将该精确路径标记为标识符映射，仍检查嵌套值和相邻字段。新增红灯测试先证明失败，再验证修复。

## 验证记录

原始红灯：ACP4项3通过/1注册失败，安装映射1项失败，vw-agent预设/绑定/路由3项失败，均退出101；没有把语法错误或零测试当成红灯。协议ID测试1项因持久化拒绝失败，退出101。

原生验收脚本：

```sh
SHIJU_SIGIT_TEST_ENTRY=/tmp/shiju-sigit-probe/runtime-closure/sigit python3 crates/vw-acp/tests/fixtures/sigit_real_probe.py
python3 scripts/acp_onboarding_sigit_acceptance.py --runtime /tmp/shiju-sigit-probe/runtime-closure --test-binary <本槽位vw-agent测试二进制> --mode session-model
python3 scripts/acp_onboarding_sigit_acceptance.py --runtime /tmp/shiju-sigit-probe/runtime-closure --test-binary <本槽位vw-agent测试二进制> --mode preset
```

`RUST_MIN_STACK`仅增加未优化测试线程栈；初次通用中继测试默认线程栈溢出，无模型请求，不计通过。增加栈后真实生产中继9次请求认证均匹配，主模型acp-fixture-primary、换会话模型acp-fixture-switched，全部走/v1/chat/completions，errors=0；new/second_turn/cancel/restart/switch均true。最终结果见下方完成记录。

首轮注册保护受开发机既有受审Codex/OpenCode安装影响，预期npx的老用例失败并毒化环境锁；不修改这些用例，以空VIBEWINDOW_CONFIG_DIR隔离重跑。目录保护已通过20项，30个唯一ACP后端、原默认启用数6保持不变。


## 最终原生证据

最终固定补丁SHA256：`4d775d40748eb3a4fc5656034d591052c4d8e7d99cd58e238ea9f6230f9f9256`。安装产物收据的source_sha256/patch_sha256及二进制sha256均已独立回读匹配。官方源码+受审补丁通过生产安装脚本实际构建，未用伪造产物收据替代真实CLI。

- `real-boundaries.log`：13项真实CLI检查通过，额外覆盖符号链接越界、缺少客户端文件能力、开启MCP环境开关后仍不启动环境配置服务器；最终runtime-closure实际写入临时文件，拒绝路径无哨兵泄露。
- `production-preset.log`：1项通过，实际model=preset-selected-model，has_preset_rule=true；生产保存/重读、启停、会话选择及chat.done均通过。这是独立于协议fixture的第二项交付证据。
- `production-relay.log`：1项通过，9次受控上游请求、认证匹配、errors=0；主模型与新会话切换模型均有实际请求；new/second_turn/cancel/restart/switch/handshake全部true。原会话替换与禁用模型在触碰CLI前失败且保留恢复锚点。
- `final-acp.log`：6项通过；`final-config.log`：1项通过；`final-key-policy.log`：3项通过；隔离后的`final-registry.log`：25项通过；`protect-catalog.log`：20项通过。

上述日志均位于`/tmp/shiju-sigit-probe/`。早期调试失败已区分：原版权限绕过、协议ID持久化拒绝、环境注册保护干扰、测试栈溢出，以及模拟进程未遵守严格--acp参数；这些失败不计入最终通过数量。

Cargo命令均通过`python3 scripts/cargo_slot.py`入口；源码构建使用槽位4/6，ACP/配置中间阶段使用槽位8，最终顺序检查使用槽位4。所有槽位的两个构建目录相同：

- 槽位4：CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=`/Users/xiangmin/worker/code/shiju/target/rust-slot-4`
- 槽位6：CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=`/Users/xiangmin/worker/code/shiju/target/rust-slot-6`
- 槽位8：CARGO_TARGET_DIR=CARGO_BUILD_BUILD_DIR=`/Users/xiangmin/worker/code/shiju/target/rust-slot-8`

确切原生检查命令（同阶段置于一次槽位入口内顺序执行）：

```sh
cargo test -p vw-acp --test onboarding_sigit -- --test-threads=1
cargo test -p vw-acp --lib acp_onboarding_sigit -- --test-threads=1
cargo test -p vw-acp --test persisted_key_policy -- --test-threads=1
VIBEWINDOW_CONFIG_DIR=/tmp/shiju-sigit-probe/empty-registry cargo test -p vw-acp --lib agent_registry_tests -- --test-threads=1
cargo test -p vw-config-types --lib acp_onboarding_sigit -- --test-threads=1
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_onboarding_sigit -- --test-threads=1
cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib agent::catalog_tests -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
```

源码安装实际命令：`python3 scripts/cargo_slot.py python3 crates/vw-acp/src/managed_runtime/install.py /tmp/shiju-sigit-probe/closure-assets /tmp/shiju-sigit-probe/runtime-closure sigit`，退出0。安装脚本以已核验绝对CARGO_TARGET_DIR构建；不覆盖槽位、不clean、不在Cargo进程组之外遗留构建任务。`cargo fmt --all`退出0，后续修改文件单独rustfmt；未运行全量单元测试、fmt check、wasm check或Makefile命令。


最终收口：`closure-options.log`退出0（1项），`closure-agent.log`退出0（4项通过、1项需要原生环境的用例另行执行），`closure-clippy.log`退出0。实际原生预设用例在`closure-preset.log`独立执行1项，不能把ignored计入通过。所有已要求的当前验证均有非零有效测试数量。

安装前提：受审源码安装需要Rust工具链和仓库共享槽位环境；缺少前提时明确报错，不回退到未修补npm二进制。核心读写/前台命令走宿主，MCP与所选技能挂载保持明确不可用。首次启用可选择已验证的CLI本地源或支持工具的会话模型，MCP显式置空、技能挂载置空。其他OS/架构需独立回归。

原生通用校验器中原有的Goose固定二进制例外已限定为agent=goose，避免新增native契约误接受另一个供应商的产物。此调整仅收紧既有身份边界。

自有槽位入口均正常收尾（早期主动中断除外亦由入口清理），保留所有缓存，不清理其他任务锁/进程。补丁文件的空白上下文行是unified diff必需语法；普通源文件与文档的staged whitespace检查通过，不通过改写补丁上下文改变已验证哈希。
