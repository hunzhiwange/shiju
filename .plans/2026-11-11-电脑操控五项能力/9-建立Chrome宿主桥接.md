# 任务 9: 建立Chrome宿主桥接

批次：【批次 9】 依赖：批次 8；线性前置任务 8

状态：已完成，待回归

目的：让扩展请求通过本机身份校验接入宿主。

来源任务：用户要求截图五项全部支持；无外部任务清单。

预计会话范围：只处理本文件定义的行为与列明的模块；跨后端的实机验收已独立拆分。若发现新平台机制需要多会话，先记录具体阻塞，不能以界面或模拟替代执行。

## 文件

- 新建：`crates/vw-agent/src/computer_control/chrome/mod.rs`
- 新建：`crates/vw-agent/src/computer_control/chrome/bridge.rs`
- 新建：`crates/vw-agent/src/computer_control/chrome/bridge_tests.rs`
- 修改：`crates/vw-agent/src/computer_control/mod.rs`
- 新建：`scripts/computer_control/chrome_native_host.py`
- 新建：`scripts/computer_control/test_chrome_native_host.py`

## 上下文

- 仓库：`/Users/xiangmin/worker/code/shiju`。新增路径均为本计划定义的待创建产物，现有模块编辑前重新核对。
- Chrome Native Messaging host 仅负责帧传输并连接已认证 ControlHost；清单安装在用户范围，使用任务8实际扩展ID。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
- 工作区已有大量其他任务变更，只改本任务列明的文件；保留其他变更，发现无关编译失败仅记录。

## 顺序与资源

- 线性顺序：任务1→2→3→4→5→6→7→8→9→10→11→12→13→14→15→16→17→18→19→20→21→22→23→24；当前任务9，批次9。
- 可并行执行批次：无。共享注册文件、配置类型和本机原生输入/Chrome/Office验证资源，采用每批一个任务，缓存分槽不改变源文件或实机互斥。
- 前置产物以实际代码和提交/证据为准；任务文件可能在完成后删除，不能只凭文件不存在判定完成。
- 锁屏依赖阻塞不阻止不依赖锁屏执行的任务18—22；任务23、24的五项完整验收仍保持阻塞。

## Cargo 共享槽位规则

1. 同一仓库的所有计划和任务共用仓库根目录下的 `target/rust-slot-1` 至 `target/rust-slot-20`，不得为任务、会话或计划另建构建目录，也不得按任务编号取模固定分配。
2. 每次构建/验证阶段开始前，创建仓库根目录下的 `target/rust-slot-locks`；依次用原子命令 `mkdir target/rust-slot-locks/slot-N.lock` 尝试领取槽位，`N` 为 1–20。仅 `mkdir` 成功者获得该槽位；这些相对路径命令从仓库根目录执行。
3. 领取成功后，在自己的锁目录中记录计划、任务、会话标识和持有进程 PID。同一槽位同时只允许一个持有者；已占用时尝试下一个，仅全部 20 个槽位都占用时等待并重试。
4. 将 `CARGO_TARGET_DIR` 设置为领取槽位的绝对路径，例如领取槽位 3 后，从仓库根目录执行 `export CARGO_TARGET_DIR="$PWD/target/rust-slot-3"`。示例编号不代表固定分配。
5. 所有 Cargo 构建、检查、测试以及调用 Cargo 的脚本和子进程均继承该设置，不得通过 `--target-dir` 或其他配置绕过槽位；生成的二进制也从该槽位定位。
6. 该阶段的 Cargo、脚本以及使用槽位产物的子进程全部结束后，删除自己写入的占用记录，并用 `rmdir` 释放自己的锁目录；保留槽位编译缓存供后续任务复用。
7. 正常结束、失败和中断均须清理自己的锁；在后台进程或其他使用槽位产物的进程仍运行时不得提前释放。规划清理步骤时覆盖这三种退出路径。
8. 遗留锁须先核实持有会话及相关进程已结束；无法确认则保持占用，不得删除其他活跃任务的锁、进程或缓存。
9. 生成或更新计划时，统一移除与这些规则冲突的单一 Rust 共享构建空间、逐任务独立目录和 Cargo 缓存全局独占窗口要求；仍须遵守源码文件占用、真实业务依赖和其他共享验证资源的互斥，不能仅因缓存分槽就把任务判定为可并行。
10. 最终验证记录包含实际槽位、绝对路径 `CARGO_TARGET_DIR`、确切命令、退出码和有效测试数量；Rust 代码修改后先运行 `cargo fmt --all`，再运行相应验证。编译成功或零测试不能替代行为验收证据。

补充：`cargo fmt --all` 后审查差异，不提交其他模块纯格式化变更；不覆盖其他会话编辑。除本文件明确列出的聚焦测试外，不运行全量测试、wasm 检查或 Makefile。

## 测试计划

- 行为：真实长度前缀帧能往返，错误扩展来源和超大消息被拒绝。
- 失败验证测试：`crates/vw-agent/src/computer_control/chrome/bridge_tests.rs`；新增测试名称统一以 `cc_control_09_` 开头，后缀说明具体行为与结果。
- 失败验证命令：`cargo test -p vw-agent --lib cc_control_09 -- --nocapture`
- 预期失败原因：现有代码缺少本任务行为，真实子进程帧传输，覆盖拆帧、超限、断连、请求关联和错误来源。 实现前先新增测试并确认失败落在该行为；若为缺符号编译失败，集成后还须证明对应拒绝/成功断言实际运行。
- 通过验证命令：`cargo test -p vw-agent --lib cc_control_09 -- --nocapture`
- 模拟策略：本地策略、注册表与传输使用真实实现；仅OS、Chrome API、Office.js和不可稳定控制的远端界面作为外部边界模拟。模拟证据不得替代实机成功证据。

## 步骤

1. 先在独立测试文件添加上述失败用例，运行失败验证命令，保存预期失败；仅实现这些用例证明的最小变化。
2. 实现长度有界的 Native Messaging 帧和本地宿主会话绑定，不将 native host 变为通用命令执行器。
3. 清单允许来源必须来自任务8生成的实际扩展身份；安装、移除均限定 ShiJu 用户目录。
4. 接入扩展可用性检测和断线状态；Python辅助入口只桥接，不另存长期权限。
5. 运行下列通过验证及所需实机检查，记录当前证据；重构只在行为测试持续通过时进行。

## 验证

- 运行：`cargo fmt --all`
- 运行：`cargo test -p vw-agent --lib cc_control_09 -- --nocapture`
- 运行：`python3 -m unittest discover -s scripts/computer_control -p test_chrome_native_host.py`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- features：聚焦测试使用目标crate默认features；最终clippy使用workspace全部features。
- 预期：至少5个 Rust 测试及 Python 帧测试；真实临时 profile 能建立连接。
- 所需证据：失败与通过的确切命令、退出码、有效测试数量、审查后的本任务差异；涉及真实后端时补实际版本、目标身份与脱敏回读。零测试、仅编译通过、跳过实机或HTTP 200均不算行为完成。

## 完成

- 仅使用状态：`未开始`、`已完成`、`已阻塞`、`已完成，待回归`。
- 新证据齐备才可标记已完成；不可用的OS/Office依赖是功能阻塞，不能当无关构建错误略过。无关既有构建失败可标记已完成，待回归并保留本任务文件，注明确切失败与已完成的验证。
- 阻塞先调查并在已有授权内修复必要相邻依赖；无法消除时只在本任务文件记录原因、尝试、命令和所需输入，不修改plan.md或其他任务状态。
- 已完成或已完成，待回归且有实际代码、测试、配置或文档变更，使用 `$git-commit` 技能检查 staged/unstaged/untracked，仅提交本任务实际变更。只有本任务文件状态更新或删除时不调用提交技能、不单独提交。
- 最终报告文件、符号、行为及完成证据；已完成后删除本任务文件，待回归和阻塞时保留。计划与提示词不作为进度来源，plan.md始终只读。

## 本次实现与证据（2026-11-12）

- 依赖：批次8提交 `df9bfb40b`，实际扩展 ID `ogfgcnpdhnmkpcbaclbglefaclanipmg`；批次7提交 `a13ce805e`。未修改 plan.md 或其他任务状态。
- 实现：`ChromeBridge::pair/serve/is_available/enumerate`，同用户 Unix IPC、challenge/session 绑定、响应 ID/递增序列关联、有界帧、空闲 EOF 与取消断连；`serve` 消费桥接实例，控制端结束即关闭原生端口。动作经真实 ControlHost 一次性 ticket 和实时策略；读、点击、输入使用固定协议，不执行任意脚本或命令。导航、媒体解析和额外按键/滚动明确返回 Unsupported，留给后续动作/站点策略任务接入。
- Python 入口仅转换本机字节序 Native Messaging 与大端宿主帧，不持久化 ticket/权限。安装/移除限定 `~/.shiju/computer-control` 的自有脚本与独立 Chrome profile 清单；Chrome 使用 `--user-data-dir=~/.shiju/computer-control/chrome-profile`。限制真实扩展来源，目录0700、socket0600并检查连接双方UID。
- 红灯：`cargo test -p vw-agent --lib cc_control_09 -- --nocapture`，退出101，0项运行；缺少 bridge 模块（E0583），见 `/tmp/cc09-red.log`。集成后下列隔离源码测试已实际运行全部成功/拒绝断言。
- 仓库首次通过验证尝试：同一聚焦命令退出101，0项运行，其他任务文件 `gateway/api/handlers/mcp_management_resync.rs:359` E0282；其持有任务随后已补闭包类型，本任务未修改该文件。见 `/tmp/cc09-green.log`。
- 独立源码验证：临时 `/tmp/cc09-harness/Cargo.toml` 直接通过绝对 path 导入本仓库 host.rs、policy.rs、bridge.rs、bridge_tests.rs；配对比较函数逐字提取原实现，仅修正测试 fixture 的 CARGO_MANIFEST_DIR。不是模拟策略/传输。`cargo test --manifest-path /tmp/cc09-harness/Cargo.toml cc_control_09 -- --nocapture` 退出0，9项通过（真实子进程拆帧、错误扩展、超限、截断、请求关联、空闲断连、取消、ControlHost鉴权与即时撤销），实机项单独运行，见 `/tmp/cc09-harness-final.log`。
- 实机：`cargo test --manifest-path /tmp/cc09-harness/Cargo.toml cc_control_09_real_chrome_native_profile -- --ignored --nocapture` 退出0，1项通过，见 `/tmp/cc09-live-retry.log`。仓库内 Python 测试文件包含可复跑 CDP driver，不依赖临时脚本。真实 Chrome/157.0.8103.40，原始扩展ID同上，真实 connectNative→Python→Rust challenge配对与 enumerate 回读；目标 tabId=1921212470、documentId=7EAFB017EDEC8A0EDE2FA5B11FD2B83E，回环 fixture origin=http://127.0.0.1:58231，未使用模拟原生端口。初次仓库内 driver 复跑发现残留 DevToolsActivePort，已在启动前清理并通过重试。测试后已移除独立profile、宿主清单、脚本和socket，Chrome/HTTP服务退出。
- Python：`python3 -m unittest discover -s scripts/computer_control -p test_chrome_native_host.py` 退出0，8项通过，见 `/tmp/cc09-python-final.log`。
- 扩展回归：`node --test plugins/shiju-computer-control/chrome/background.test.mjs` 退出0，18项通过，见 `/tmp/cc09-extension.log`。
- `cargo fmt --all` 退出0；审查本任务差异，未暂存或提交其他任务文件及纯格式化变更。
- clippy：`cargo clippy --all-targets --all-features -- -D warnings` 退出101，构建检查不运行行为测试；失败于其他任务的 `crates/vw-desktop/src/app/message/settings/aggregation_list.rs:30`，Form.selection / Form.generation 未读取，lib与lib test均报错。本任务源码无 clippy 报错，未修改外部文件。见 `/tmp/cc09-clippy.log` 与 `/tmp/cc09-stage.log`。因此保留“已完成，待回归”和任务文件，待外部改动完成后复跑该命令。
- workspace 聚焦复跑：`cargo test -p vw-agent --lib cc_control_09 -- --nocapture` 退出0，9项通过、0失败、实机项按设计单独运行；见 `/tmp/cc09-workspace-retry.log`。完整12,306个其他测试只编译、未执行。链接器提示整个lib test的 __eh_frame 大于16MB，不属本任务源代码诊断；未改动仓库编译配置。
- 槽位：按原子mkdir动态领取，使用过2（红灯/独立源码/实机/聚焦复跑）、8（workspace验证）、11（最终独立源码）、13（fmt）。对应绝对 CARGO_TARGET_DIR 分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-8`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-11`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-13`。发现仓库单独配置build-dir后，后续阶段同步设置 CARGO_BUILD_BUILD_DIR=$CARGO_TARGET_DIR，未删除共享缓存或其他任务锁。
- 协作事件：曾误向另一任务的 `cargo test -p vw-desktop --bin shi-ju aggregation_t56_ -- --nocapture` 发送SIGINT，已向用户说明；随后观察原持有任务已自行启动 `--lib aggregation_t56_` 验证。本任务未修改该任务源码或锁。

- 最终实机补证：在重新动态领取的槽位2中复用刚生成的真实仓库测试产物，`/Users/xiangmin/worker/code/shiju/target/rust-slot-2/debug/deps/vw_agent-44fc2a0c36b13a17 --ignored --exact computer_control::chrome::bridge_tests::cc_control_09_real_chrome_native_profile --nocapture` 退出0，1项通过（无重编译、无模拟传输），见 `/tmp/cc09-native-binary-final.log`。最终回读 tabId=352644054、documentId=CC23D49E87D2F40192E014AFCDCBBCD9、fixture origin=http://127.0.0.1:61634；真实 Chrome 与扩展版本身份同前。二进制、浏览器、HTTP服务退出后，已移除本次安装的profile/清单/脚本/socket；本会话所有槽位锁均已释放，缓存保留。
- 下一步：其他任务修复 aggregation_list.rs 未使用字段后，复跑 workspace clippy；通过后可删除本任务文件并进入批次10。当前保留“已完成，待回归”，未以拒绝或模拟替代真实Chrome证据。
