# 任务 10: 实施Chrome站点权限

批次：【批次 10】 依赖：批次 9；线性前置任务 9

状态：已完成，待回归

目的：在动作实际执行时控制站点访问、跳转和文件传输边界。

来源任务：用户要求截图五项全部支持；无外部任务清单。

预计会话范围：只处理本文件定义的行为与列明的模块；跨后端的实机验收已独立拆分。若发现新平台机制需要多会话，先记录具体阻塞，不能以界面或模拟替代执行。

## 文件

- 新建：`crates/vw-agent/src/computer_control/chrome/origin_policy.rs`
- 新建：`crates/vw-agent/src/computer_control/chrome/origin_policy_tests.rs`
- 修改：`crates/vw-agent/src/computer_control/chrome/bridge.rs`
- 修改：`crates/vw-agent/src/tools/computer_control.rs`

## 上下文

- 仓库：`/Users/xiangmin/worker/code/shiju`。新增路径均为本计划定义的待创建产物，现有模块编辑前重新核对。
- 任务3策略是权威状态；插件启用、Chrome启用和网站允许是三个独立条件。不能因任意应用授权就绕过网站限制。
- 按无人值守方式执行：常规、可逆、低风险事项自行选择符合仓库约定的最佳可行方案，遇到问题先调查修复并继续；仅在关键歧义无法消除或确需新增授权时集中询问，记录决策和阻塞证据，不重复请求已有授权。
- 工作区已有大量其他任务变更，只改本任务列明的文件；保留其他变更，发现无关编译失败仅记录。

## 顺序与资源

- 线性顺序：任务1→2→3→4→5→6→7→8→9→10→11→12→13→14→15→16→17→18→19→20→21→22→23→24；当前任务10，批次10。
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

- 行为：未授权 origin 不读写；跳转及撤销后旧标签页句柄失效。
- 失败验证测试：`crates/vw-agent/src/computer_control/chrome/origin_policy_tests.rs`；新增测试名称统一以 `cc_control_10_` 开头，后缀说明具体行为与结果。
- 失败验证命令：`cargo test -p vw-agent --lib cc_control_10 -- --nocapture`
- 预期失败原因：现有代码缺少本任务行为，覆盖跨站跳转、重定向、子框架、上传下载单独批准、删除站点授权后的缓存句柄拒绝。 实现前先新增测试并确认失败落在该行为；若为缺符号编译失败，集成后还须证明对应拒绝/成功断言实际运行。
- 通过验证命令：`cargo test -p vw-agent --lib cc_control_10 -- --nocapture`
- 模拟策略：本地策略、注册表与传输使用真实实现；仅OS、Chrome API、Office.js和不可稳定控制的远端界面作为外部边界模拟。模拟证据不得替代实机成功证据。

## 步骤

1. 先在独立测试文件添加上述失败用例，运行失败验证命令，保存预期失败；仅实现这些用例证明的最小变化。
2. 规范化 scheme/host/port，逐次核对当前页面 origin，不只在打开标签页时检查。
3. 将站点批准接入现有权限请求链路；读写、上传、下载按实际能力显式检查。
4. 工具动作接入 Chrome 桥接；防止通过通用原生 computer_control 工具绕过已拒绝的 Chrome 专用权限。
5. 运行下列通过验证及所需实机检查，记录当前证据；重构只在行为测试持续通过时进行。

## 验证

- 运行：`cargo fmt --all`
- 运行：`cargo test -p vw-agent --lib cc_control_10 -- --nocapture`
- 运行：`cargo clippy --all-targets --all-features -- -D warnings`
- features：聚焦测试使用目标crate默认features；最终clippy使用workspace全部features。
- 预期：至少6个测试及一次跳转后拒绝的真实浏览器记录；批准来源与实际页面匹配。
- 所需证据：失败与通过的确切命令、退出码、有效测试数量、审查后的本任务差异；涉及真实后端时补实际版本、目标身份与脱敏回读。零测试、仅编译通过、跳过实机或HTTP 200均不算行为完成。

## 完成

- 仅使用状态：`未开始`、`已完成`、`已阻塞`、`已完成，待回归`。
- 新证据齐备才可标记已完成；不可用的OS/Office依赖是功能阻塞，不能当无关构建错误略过。无关既有构建失败可标记已完成，待回归并保留本任务文件，注明确切失败与已完成的验证。
- 阻塞先调查并在已有授权内修复必要相邻依赖；无法消除时只在本任务文件记录原因、尝试、命令和所需输入，不修改plan.md或其他任务状态。
- 已完成或已完成，待回归且有实际代码、测试、配置或文档变更，使用 `$git-commit` 技能检查 staged/unstaged/untracked，仅提交本任务实际变更。只有本任务文件状态更新或删除时不调用提交技能、不单独提交。
- 最终报告文件、符号、行为及完成证据；已完成后删除本任务文件，待回归和阻塞时保留。计划与提示词不作为进度来源，plan.md始终只读。


## 实施与证据（2026-11-12）

- 前置：批次9实际提交 `0717b4e95` 及真实Chrome配对证据存在；本会话先复跑 workspace clippy，退出0，原批次9的无关警告已消失。未修改 plan.md 或其他任务状态。
- 已补齐先前的协议/策略/扩展实现缺口；它们不再作为功能阻塞。文件通道使用现有 activeTab/scripting 能力，以 DataTransfer/File 上传批准的内联字节、同源 fetch 下载直链内容；没有新增 Chrome manifest 权限，不接受任意本地路径。传输上限256KiB，下载拒绝重定向并回传内联字节，不在宿主写入文件。
- `origin_policy.rs::canonical_origin/DocumentBinding::check` 规范化HTTP(S) scheme/host/port、拒绝凭据及其他scheme；绑定tab/document/navigationVersion/完整URL，失效后不因回到旧URL而恢复。
- `ChromeBridge::execute` 每次动作前后重查ControlHost实时授权和当前文档；Navigate独立校验目标origin并废弃旧句柄；重定向或换文档后丢弃回读。扩展只执行顶层documentId，拒绝子框架结果；普通Click/Fill不能直接操作文件输入及download链接。
- `ControlHost::recheck/authorize_destination/approve_transfer/consume_transfer` 将传输许可绑定完整请求、目标、会话、权限版本、请求ID及文件字节，方向不可互换且只用一次；停止、撤销、过期不会恢复旧许可。
- `ComputerControlTool` 的封闭Chrome动作已接入真实Native Messaging宿主，使用同用户0700目录/0600 socket、单控制端、一次性ticket和原有执行器上下文。配对等待支持取消；非macOS工具入口明确不支持。原生Chrome拒绝仍由底层权威策略执行。
- 站点、上传、下载分别调用现有HookRunner/ApprovalManager请求链；显式站点Deny不可被批准覆盖；Full Access、工具白名单及turn bypass不授予传输权限。站点批准为到撤销/网关重启的内存授权，持久设置仍由原策略配置负责。通用审批展示/审计采用脱敏元数据，文件审批显示大小/摘要，执行绑定原始请求。
- 必要相邻范围：`chrome/mod.rs`、`chrome/bridge_tests.rs` 注册与后置文档探测fixture、`computer_control/host.rs` 的即时重查/单次许可、`vw-api-types/src/computer_control.rs` 的Upload/Download/ChromeDownload闭合DTO、`tools/decision/mod.rs` 的强制作用域批准与脱敏展示、`tools/computer_control/chrome.rs`、`tools/computer_control/chrome_tests.rs` 的实际调用者、扩展background及独立策略测试、`scripts/computer_control/chrome_origin_live.mjs` 的受控实机driver。未改无关模块。

### 确切命令与结果

| 命令 | 退出码 | 有效行为测试 / 证据 |
| --- | --- | --- |
| 红灯：`cargo test -p vw-agent --lib cc_control_10 -- --nocapture` | 101 | 0；E0583缺少origin_policy模块；见 `/tmp/cc10-red.log`。后续所有拒绝/成功断言均实际运行，未将编译失败算作行为证据 |
| JS红灯：`node --test plugins/shiju-computer-control/chrome/origin_policy.test.mjs` | 1 | 新增4项失败（导航、独立传输批准、批准上传、子框架）；见 `/tmp/cc10-js-red.log` |
| `cargo fmt --all` | 0 | 多轮执行并审查本任务差异；最后见 `/tmp/cc10-final-lints.log`；未暂存其他模块格式化变更 |
| `cargo test -p vw-agent --lib cc_control_10 -- --nocapture` | 0 | 最终19 passed / 0 failed / 1 ignored（实机项另跑），12327个无关测试只编译未执行；见 `/tmp/cc10-final-confirm.log` |
| `node --test --test-name-pattern=cc_control_10 plugins/shiju-computer-control/chrome/origin_policy.test.mjs` | 0 | 4 passed；结果见本会话工具输出 |
| 槽位3测试产物 `debug/deps/vw_agent-44fc2a0c36b13a17 --exact computer_control::chrome::bridge_tests::cc_control_09_authenticated_host_and_live_revocation --nocapture` | 0 | 1 passed；仅回归因新增后置文档探测而调整的相邻fixture |
| 槽位3测试产物 `debug/deps/vw_agent-44fc2a0c36b13a17 --exact tools::computer_control::chrome::tests::cc_control_10_real_chrome_tool_transfer_and_redirect_denial --nocapture --ignored` | 0 | 1 passed；完整工具执行器→真实ControlHost→Native Messaging→真实Chrome，见 `/tmp/cc10-final-confirm.log` |
| `cargo clippy --all-targets --all-features -- -D warnings`（槽位2、槽位3阶段） | 0 | workspace全部features已通过，日志 `/tmp/cc10-final-stage.log`、`/tmp/cc10-final-confirm.log` |
| 同一clippy命令最终补充复跑（槽位12） | 101 | 无关网关高可用处理器E0599；见下述回归原因及 `/tmp/cc10-final-lints.log` |

聚焦测试为vw-agent默认features；clippy为workspace全部features。集成中出现的Arc上下文借用与测试factory返回类型错误均已修复。链接器沿用任务9已记录的 `__eh_frame > 16MB` 性能提示，测试正常通过；未更改编译配置或隐藏警告。未运行全量单元测试、wasm检查或Makefile。

### 真实浏览器回读

- 实际版本 `Chrome/157.0.8103.40`，扩展 `ogfgcnpdhnmkpcbaclbglefaclanipmg`；最后一轮 tabId=`160276103`，批准origin=`http://127.0.0.1:57095`。
- 模型工具读取受控页面、上传fixture后服务端实际收到19字节、下载回读准确等于fixture字节；driver确认downloads=1。
- Navigate访问批准origin的 `/redirect` 后，真实最终URL为 `http://localhost:57095/destination`；对旧目标发起Snapshot实际返回 `TargetChanged`，没有读取未批准origin的内容。
- CDP仅负责启动独立profile、加载扩展及点击扩展授权；页面动作和传输经本仓库真实模型工具、权限请求链和Native Messaging执行。批准响应使用受控fixture HookHandler，政策、许可、注册表、帧传输和Chrome API均为真实实现。
- 两轮实机均退出0。浏览器、Node/HTTP服务退出后，移除了本次独占创建的profile、Native Messaging注册、relay和socket，`~/.shiju/computer-control` 已不存在；未操作正常Chrome profile或生产文档。

### 待回归原因与资源交接

- 最后复跑clippy时，其他任务新增的 `crates/vw-agent/src/gateway/api/handlers/agent_apps_high_availability.rs:173,175,194` 对 `ProviderId` 调用 `.to_string()`，但该强类型未实现Display，触发3处E0599。已读取其独立变更确认是队列写入接口，与本任务Chrome DTO/策略无依赖；按AGENTS.md未修复、暂存或提交该无关文件。
- 槽位12的lib检查已确定失败后，仅停止了该失败命令尚在运行的lib-test clippy-driver（PID28458，已验证父进程19659及本会话owner PID4803），Cargo收尾退出101才释放自己的锁；没有停止其他任务进程。此轮不能算作clippy通过。前两轮通过和最终19项测试/实机证据仍如上，故按用户例外规则标记“已完成，待回归”，保留任务文件。
- 各阶段均依次原子mkdir动态领取1–20槽位，并同步设置绝对CARGO_TARGET_DIR与CARGO_BUILD_BUILD_DIR；实际使用槽位1、2、3、4、8、11、12，对应绝对路径分别为 `/Users/xiangmin/worker/code/shiju/target/rust-slot-1`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-2`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-3`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-4`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-8`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-11`、`/Users/xiangmin/worker/code/shiju/target/rust-slot-12`。最终行为/实机阶段为槽位3，最终clippy失败阶段为槽位12；各阶段进程及产物使用者退出后已释放自己的owner/锁，保留缓存。
- 下一步：由对应任务修复ProviderId转换后，按共享槽位规则复跑同一clippy命令；通过后将本任务更新为已完成并删除本文件。代码、19项行为测试、4项JS测试和真实浏览器证据已交付，不再以实现缺口阻塞后续批次。
