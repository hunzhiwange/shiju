# Gemini 完全访问：客户端批准与原生限制

日期：2026-11-06（Asia/Shanghai）；平台：macOS arm64；依赖批次 6 提交 `da4aec968`，其共享协议实现已存在，待回归的全仓问题不改变本任务的权限结论。

## 制品与源码（E2）

实际入口 `/opt/homebrew/bin/gemini` → npm `@google/gemini-cli/bundle/gemini.js`，包版本及隔离 `--version` 均为 **0.66.0**。未升级用户安装；使用临时 HOME、私有 Gemini 配置与项目、loopback Google GenerateContent 上游、仅本地有效的占位密钥，未读取或打印真实凭据。

官方来源：[配置](https://geminicli.com/docs/reference/configuration/)、固定版本 [ACP 会话](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/acp/acpSession.ts)、[模式枚举](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/acp/acpUtils.ts)、[RPC 分派](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/acp/acpRpcDispatcher.ts)、[底层配置](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/config/config.ts)。固定源码下载成功并与本地对应实现核对；源码结论与运行结果分开记账。

- Gemini 将 Plan/Communicate 工具映射为 Other，计划与问答仅提供普通 allow_once/reject_once 选项，没有结构化确认载荷；不能自动批准这类流程确认。该点为 E2，不冒充实机计划工具 E3。
- 精确 legacy 模式：`default / autoEdit / yolo / plan`，没有 `setSessionConfigOption` 实现，`resume` 未实现。
- `setMode` 返回空对象；模式事件发送 `agent_message_chunk` 文本 `[MODE_UPDATE] ...`，没有标准 `current_mode_update`。不解析模型可仿造的文本作为安全确认。
- `loadSession` 会 dispose 旧运行对象并重新 prepareSessionConfig；load 的模式响应是新配置，不能替代运行中 set_mode 的无副作用回读。
- 底层 `setApprovalMode` 拒绝在未信任目录开启特权模式；切换 yolo/plan 会刷新工具、沙箱管理器和系统指令，并清除当前序列的模型选择。不能把该切换当作不改变模型、工具、提示词的纯权限选项。
- `disableYoloMode`/安全模式、工作区信任、策略 deny 和独立沙箱有各自边界。客户端自动批准不能覆盖供应商硬拒绝，也不等于解除 OS 沙箱。

本次制品关键 SHA256：

| 文件 | SHA256 |
| --- | --- |
| package.json | `dfd44e39afc070f5fb28e26b870f980ea41bb4f3ab124a97b977d62b81e36662` |
| bundle/gemini.js | `f48ad00f929e699361d442440e89a70ab08342f9042ea22f99f0e9ff4bf60ba6` |
| bundle/gemini-B5V54E2S.js | `5e0abc1aa8df60a3a48ea03a03bf3477c52c9b6fde5561d99aedab398c7a5a5b` |
| bundle/chunk-RZ3ZVTZB.js | `19ff9f53cf4dbe81755abc4f1d8576186ba9bcd9490015f07116db134da07398` |

完整 72 个顶层 bundle JS 哈希保存在本地证据 `bundle-sha256.json`，其按 key 排序 JSON 的 SHA256 为 `4de780f783b924f965659f312973d83dfc63c15f20ba0bdbb8f77fa02c277c9c`。该哈希用于描述本次制品，不作为生产 Native 白名单。

## 真实协议与执行（E3，限定平台与入口）

真实 CLI 未打补丁，受控上游只生成固定 `run_shell_command` 或 `write_file` 和结束响应，实际工具由 CLI 执行。客户端不公布 fs/terminal，取消所有普通审批。手工指定 yolo 仅用于隔离供应商能力探针，生产不会自动启用。

| 原始 JSON-RPC / 工具 | default | yolo | default（撤销） |
| --- | --- | --- | --- |
| shell 写临时文件 | 不存在；审批 1 | 存在；审批 0 | 不存在；审批 1 |
| write_file 写临时文件 | 不存在；审批 1 | 存在；审批 0 | 不存在；审批 1 |
| 用户策略 run_shell_command deny | 不存在；审批 0 | 不存在；审批 0 | 不存在；审批 0 |

initialize：协议 1，`agentInfo=gemini-cli/0.66.0`，loadSession=true，image/audio/embeddedContext，MCP HTTP/SSE。new 公布上述模式，当前 default，模型始终 `gemini-2.5-flash`。三次 `set_mode` 均返回 `{}`；没有标准模式回读通知。load 保留同一 sessionId 并回读 default；`session/resume` 和 `session/set_config_option` 返回 -32601。普通审批提供 `proceed_always/allow_always`、`proceed_once/allow_once`、`cancel/reject_once`，探针统一返回 cancelled。客户端 fs/terminal 回调均为 0；shell 结果不等于客户端 terminal/create 验收。

单独隔离启动核验：私有 `security.disableYoloMode=true` 时，`--experimental-acp --approval-mode yolo --skip-trust` 退出 **52**，明确报管理员禁用。进一步直接 ACP set_mode 在该设置下仍能返回成功并执行本例工具，因此不能把仅启动期检查当作动态控制的可靠安全门。生产不尝试突破此限制，也不通过切换 yolo 绕过检查。用户策略 deny 的证据不是企业部署 E3；原生 OS 沙箱、网络和其他平台仍未验收。

早期 deny/secure 探针将设置放在 GEMINI_CLI_HOME/settings.json，未被实际核心配置读取，结果不计入限制验收；修正到 GEMINI_CLI_HOME/.gemini/settings.json 后重跑。入口 relaunch 的内存设置读取路径与核心设置路径不同，不据此修改模型驱动。

## 生产行为与分类

**客户端自动批准；部分支持。原生自动提权不可用。** 原始供应商 yolo 有免审批行为，但不满足本计划的标准确认、动态恢复和不隐式改变工具/系统指令的契约，所以不接入 Native。

- `client/process.rs::launch_args` 对普通、npx、托管材料及自定义 Gemini 入口均不注入 yolo 或 skip-trust；显式 bypass/trust 参数明确拒绝，避免启动后无法安全撤销。工作区信任由独立配置管理，完全访问不授予信任。
- `access_policy/gemini.rs::restricted_session` 及 `sync_native_access` 在 new/load/resume、首次 prompt 和复用会话前要求同一 session 的 default/plan 快照；高权限、未知/缺失回读及冲突 config 均明确拒绝继续，不猜测成功，不替换会话。已存在的 elevated 会话需通过供应商界面显式恢复 default 后重新加载。
- Gemini 的 other/switch_mode 或缺少 kind 的审批回调明确要求独立用户交互；当前没有结构化确认 UI，因此直接报错而不猜测标题或自动回答。普通 execute/edit/read 等结构化工具仍走原有客户端审批。
- 显式配置和 legacy 模式选择均拒绝 yolo/autoEdit；活动 turn 的受支持模式变更请求取消并要求结束后重试。不会自动退出 plan。DenyAll 在 native prompt 前拒绝执行。
- false→true→false 只改变当前宿主授权与客户端文件访问边界，原生保持 default/plan，完全访问为 true 且 ApproveAll 时仍显示 ClientOnly。关闭完全访问不会关闭独立的既有 ApproveAll 策略；回调数量区分这两个边界。
- Gemini 原生 resume 不可用；现有 ResumeOrLoad 使用同 ID load 回退，禁止把 -32601 宣称为原生 resume 成功。

必要相邻修改限于共享启动材料化、协议模式入口、prompt 屏障与 fixture；没有修改安装、模型来源实现、认证或品牌。

## 验证记录

默认 features 的聚焦回归：

| 命令 / 阶段 | 退出码 | 有效测试 |
| --- | --- | --- |
| `cargo test -p vw-acp --lib acp_full_access_gemini -- --nocapture`（红测） | 101 | 6 项中 5 项在行为断言失败，1 项通过 |
| `cargo test -p vw-acp --lib acp_full_access_gemini -- --nocapture`（最终绿测） | 0 | 10 项通过 |
| `cargo test -p vw-acp --lib acp_full_access_gemini_workflow -- --nocapture`（补充红测） | 101 | 1 项在流程确认自动批准断言失败 |
| `cargo test -p vw-acp --lib acp_full_access_launch -- --nocapture` | 0 | 6 项通过 |
| `cargo test -p vw-acp --lib client::event_client_tests -- --nocapture` | 0 | 24 项通过 |
| `cargo fmt --all`（最终源码） | 0 | 格式化完成，仅保留任务差异 |
| `cargo clippy --all-targets --all-features -- -D warnings`（最终源码） | 0 | 全仓无警告，1m07s |
| `cargo run -p vw-acp --example gemini_full_access_probe --` | 0 | 3 个真实工具结果 |
| `cargo run -p vw-acp --example gemini_full_access_probe -- model-source` | 0 | 3 个真实工具结果 |

红测覆盖显式原生模式、elevated 恢复、DenyAll、启动注入及显式 bypass 的原有错误；新增回归还覆盖 default/plan、同 ID load 与 resume→load 回退、缺失/外会话快照和活动 turn 取消。fixture 仅替换外部 Agent，仍通过实际 JSON-RPC 连接和生产生命周期方法运行；不把模拟结果计为供应商能力 E3。

真实 `AcpClient` 另验证普通 cli_local 及 session_model 同形启动材料（私有 gateway 设置与 launch reference；没有重跑安装包装器、模型绑定和网关中继的完整验收）：

| 路径 | 新建 false | 同 ID load true | 同 ID load false |
| --- | --- | --- | --- |
| cli_local | 工具成功，审批 1，Unavailable | 工具成功，审批 1，ClientOnly | 工具成功，审批 1，Unavailable |
| session_model 启动材料 | 工具成功，审批 1，Unavailable | 工具成功，审批 1，ClientOnly | 工具成功，审批 1，Unavailable |

探针明确授予隔离临时项目的独立工作区信任，配置模型始终为 gemini-2.5-flash。没有把完全访问当作信任授权；生产不注入该信任环境变量。六个结果证明实际 JSON-RPC/普通审批路径与保守分类，native resume 不支持及 resume→load 回退分别由真实原始协议和确定性生命周期测试证明。流程确认保护在工具探针构建后补充，其最终证据为独立红测、10 项绿测和全仓 Clippy；不把普通工具探针当作计划/问答实机 E3。

所有 Cargo 阶段通过原子 mkdir 动态领取下列共享槽位，`CARGO_BUILD_BUILD_DIR` 与各自 CARGO_TARGET_DIR 相同以覆盖仓库 build-dir 设置：

| 实际槽位 | 绝对 CARGO_TARGET_DIR | 阶段 |
| --- | --- | --- |
| 2 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-2` | 首轮实现后 fmt 退出 0；随后产物锁等待被中止 |
| 3 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-3` | 第一轮 9 项 Gemini/6 项启动绿测、第一轮全仓 Clippy 退出 0；复用缓存的最终全仓 Clippy 再次退出 0 |
| 5 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-5` | 初始红测、CARGO_INCREMENTAL=0 重试红测、真实双路径客户端工具探针 |
| 7 | `/Users/xiangmin/worker/code/shiju/target/rust-slot-7` | 流程确认红测；最终 fmt、10+6+24 项回归；重复冷缓存 Clippy 被主动中止 |

初始红测编译因磁盘空间不足退出 101，未计为行为失败；空间恢复后以 `CARGO_INCREMENTAL=0 cargo test -p vw-acp --lib acp_full_access_gemini -- --nocapture` 重跑，取得上述 5 个预期行为失败。该次锁目录被外部清理，清理脚本发现 owner 已不存在，未删除其他锁。槽位 2 的旧 Cargo 进程仍持有已删除的 artifact lock，诊断通过 lsof 确认后仅终止本任务的等待进程；后续分配避开实际仍有 Cargo 使用的槽位，未操作其他任务的进程/锁。

所需最终 `cargo fmt --all` 在槽位 7 退出 0，其他完成的全仓格式化阶段同样退出 0。最终 Clippy 的重复冷缓存过程被中止后，重新动态领取已释放的槽位 3，复用全 features 缓存，最终命令退出 0。没有将中断或零测试计为成功。任务所有进程结束后已释放自有锁，保留编译缓存。

最终日志：`red-retry.log`、`workflow-red.log`、`final.log`（最终 10+6+24 项回归及 fmt）、`final-clippy.log`、`rust-probe-retry.log`。首次跨进程探针因初始工作目录继承仓库而启动较慢，在首个结果后中止；改为隔离项目作为进程工作目录后完整重跑双路径，未改生产启动行为。临时 example 完成后移出仓库，源码保存在本地证据目录的 `gemini_full_access_probe.rs`。`git diff --check`、22 行矩阵去重及本地文档链接检查均通过。


本地脱敏证据目录：`/tmp/shiju-full-access-gemini7/`。`probe.py`、`probe.log`、`write.log` 为原始三阶段工具验证；`deny-final.log` 为正确私有配置下 deny；`secure-final.log` 为动态安全设置探针。每个 probe 私有目录的 trace.json/upstream.json/results.json 保存协议摘要及上游计数，不保存请求认证头。上游不会调用供应商模型。所有探针进程和服务器均在 finally 中终止。
