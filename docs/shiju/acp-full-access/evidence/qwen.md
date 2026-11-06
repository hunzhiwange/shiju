# Qwen 完全访问证据

日期：2026-11-06（Asia/Shanghai）；平台 macOS arm64；依赖任务 7 提交 `9dd24a939`。本次未找到本机 Qwen 入口，下载固定 npm 制品到隔离临时目录，没有安装、升级或写入用户全局配置。

## 固定制品与源码（E2）

实际执行 `@qwen-code/qwen-code@0.26.7` 的 `cli-entry.js`；隔离 `--version` 返回 0.26.7。npm tarball 的 SHA512 与本次发行元数据 integrity 完全一致。普通入口使用原始 npm 包；托管入口只在临时副本应用仓库现有 `patches/acp-model-source/qwen/apply.py`，没有修改补丁或模型驱动。

官方来源：[审批模式](https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/)、[固定提交 Session](https://github.com/QwenLM/qwen-code/blob/f45c34d8ccbd651f2f0f4d97c9b013c11af52f5b/packages/cli/src/acp-integration/session/Session.ts)、[固定提交 ACP Agent](https://github.com/QwenLM/qwen-code/blob/f45c34d8ccbd651f2f0f4d97c9b013c11af52f5b/packages/cli/src/acp-integration/acpAgent.ts)、[底层 Config](https://github.com/QwenLM/qwen-code/blob/f45c34d8ccbd651f2f0f4d97c9b013c11af52f5b/packages/core/src/config/config.ts)。提交取自本次 npm 元数据 gitHead；旧版 `packages/cli/src/acp/acpSession.ts` 路径返回 404，改查发行提交中的实际入口。

- 精确模式为 `plan / default / auto-edit / auto / yolo`。生产通过 `session/set_config_option` 的 `configId=mode` 设置 `default↔yolo`，响应的 `configOptions` 回读为确认依据。
- `setMode` 本身只发送扩展 mode-update 通知；空 ACK 不作为权限证明，优先使用标准 config 响应。
- default↔yolo 只更新底层权限状态，不替换模型或重建工具与系统指令；涉及 plan 的切换有流程副作用，所以完全访问同步不自动退出 plan。
- 底层会拒绝未信任工作区的特权模式；权限 deny 先于 yolo，不能用宿主批准覆盖。独立原生沙箱、企业部署与其他平台未验收。
- ExitPlanMode 使用 switch_mode，问答及未结构化工具也必须独立交互；客户端明确拒绝自动回答，不猜测标题。

| 制品 | SHA256 |
| --- | --- |
| npm tarball | `4cb0587ce0eb368230b128ffd4ff999e7b517865145a26fdab14b7e744c307f0` |
| package.json | `060c3d68437df8df04505e52a0a1c9116781c1c0ed43e7025014ee644e793fcf` |
| 原始 cli-entry.js | `113efe4104a9fc66f507d34bfd6abbd896ceb77de47f4b468a0825a327fdc6a5` |
| 原始 cli.js | `1beb22bfc41e792eb35f5cc136d79977f5a35efadf7b92be8af8752f473cf3af` |

`access_policy/qwen.rs::verified_entry` 对固定目录内全部 1140 个 JS/JSON 文件按相对路径排序，将路径、NUL、文件 SHA256 原始字节依次散列；拒绝符号链接。原包指纹 `bb52c5bcfc0c461f69dc66799856d0e4b246d47449cecb2fb55a8c0e692272e8`；现有仓库补丁副本指纹 `d34ed30c3b0647300e549f9f7e947e0ab4110307e55508ae77220c3b4e1577ca`。其他文件布局、版本、npx 或 Node/入口注入环境不获得 Native 声明。

## 真实协议与工具（E3）

临时 HOME、QWEN_HOME、项目和运行历史，loopback OpenAI 受控上游，仅使用占位密钥；不读取或打印真实凭据。CLI 未修改权限实现，上游仅生成确定性工具调用；客户端不公布文件/终端能力，原始探针取消全部工具审批。

initialize 协议 1，agentInfo=`qwen-code/0.26.7`，loadSession=true、sessionCapabilities.resume={}，图像/音频/embeddedContext 与 HTTP/SSE MCP。new 公布 modes 和 configOptions，初始 default。config 设置响应均确认请求模式；同 ID load 和 resume 均成功并回读 default。不能把扩展通知等同标准模式回读。

| 原始工具探针 | default | yolo | default（撤销） |
| --- | --- | --- | --- |
| run_shell_command 写临时文件 | 不存在；审批 1 | 存在；审批 0 | 不存在；审批 1 |
| write_file 写临时文件 | 不存在；审批 1 | 存在；审批 0 | 不存在；审批 1 |
| 私有 permissions.deny=[Bash,Write] | 不存在；审批 0 | 不存在；审批 0 | 不存在；审批 0 |

deny 探针中供应商不提供被拒绝工具；结果证明本例配置限制保留，不冒充企业托管策略 E3。客户端 fs/terminal 回调均为 0；工具是 CLI 原生 shell/write，不是客户端 terminal/create 或 fs/write 验收。额外 `craft/drainMidTurnQueue` 是供应商扩展调用，不计工具审批。

上游模型始终 qwen-probe。三阶段主会话请求的系统消息 SHA256 均为 `def1919c53cec50a8681b6e01eefe6f4fd91ebba53803988c0bcf8a7a4765252`，均公布同一组 13 个工具。背景自动记忆请求有不同提示词及 5 个工具，单独识别，不误算为权限切换副作用。

## 生产接入边界

固定制品、macOS arm64、agentInfo、无 Node/重启参数注入、当前 full_access 且 ApproveAll、同会话 config 响应确认全部满足后才报告 Native。已核验入口启动时设置进程范围 `QWEN_CODE_MANAGED_NPM_PIN`（bootstrap 为实际入口、version=null、绝对 updateRoot），阻止 npm wrapper 重定向到独立更新的 cli.js。该设置不修改更新目录、全局配置或用户安装；不接管未知入口。最终生产探针额外配置同版本、会抛错的假更新制品及重定向 pin，实际仍执行已核验包。普通及托管启动均不再注入 yolo；显式启动 bypass 直接拒绝，托管路由材料原样保留。模型选项意外改变时阻止 prompt。

new/load/resume 与 prompt 前同步当前授权；关闭发送 default，失败、缺失或错误回读维持 prompt 屏障；未知制品的 yolo 恢复及显式提权拒绝，受限未知制品只报告 ClientOnly。活动 turn 模式变更先取消并要求结束后重试，不并发撤销正在执行的工具。DenyAll 在原生 prompt 前拒绝。

完全访问与独立 ApproveAll 是不同授权。普通 ApproveAll 可能选择供应商 allow_always 并保存规则，关闭完全访问不删除这些用户批准规则。首次客户端探针在该条件下最后阶段审批 0，不能作为撤销失败或 1→0→1 的证据；最终生产探针在关闭状态使用 ApproveReads，明确拒绝普通 shell，以免历史 allow_always 污染计数。原始协议探针始终取消审批，因此没有此污染。

托管实机验收使用现有补丁及真实模型路由参数和私有环境，通过生产 AcpClient；不宣称重新验收了安装包装器、模型选择 UI 或完整网关中继。其他平台、网络、OS 原生沙箱和企业部署仍需单独验收。

## 验证记录

本地脱敏材料 `/tmp/shiju-full-access-qwen8/`：npm.json、package.tgz、固定提交源码、probe.py、各 probe 私有目录的 trace.json/results.json/upstream.json。初始工具探针复用了 tool-call ID，供应商正确拒绝重复执行；已改用唯一 ID 并重跑，只计 `probe-final.log`、`write-final.log`、`deny-final.log` 和 `prompt-comparison.log` 的最终结果。

Cargo 默认 features；所有阶段原子领取共享槽位，并设置 CARGO_BUILD_BUILD_DIR 与 CARGO_TARGET_DIR 相同以覆盖仓库 build-dir。确切命令、退出码和有效行为测试数如下。

| 命令 / 阶段 | 退出码 | 有效行为测试或结果 |
| --- | --- | --- |
| `cargo test -p vw-acp --lib acp_full_access_qwen -- --nocapture`（red.log） | 101 | 3 项在行为断言失败；没有以编译失败充当红测 |
| 同一聚焦命令（verified.log、final-pinned.log） | 0 | 18 项通过 |
| `cargo test -p vw-acp --lib acp_full_access_launch -- --nocapture` | 0 | 6 项通过 |
| `cargo test -p vw-acp --lib client::event_client_tests -- --nocapture`（verified.log） | 0 | 24 项通过 |
| `cargo fmt --all` | 0 | 每轮实现后格式化，仅提交本任务差异 |
| `cargo run -p vw-acp --example qwen_full_access_probe --`（final-pinned.log） | 0 | 3 阶段真实工具结果，审批 1→0→1，文件 false→true→false |
| `cargo run -p vw-acp --example qwen_full_access_probe -- model-source`（final-pinned.log） | 0 | 现有托管补丁及路由参数；3 阶段真实工具结果，审批 1→0→1，文件 false→true→false |

生产探针的两条入口均保持同一个逻辑 sessionId，在新进程 load 后再次通过 ResumeOrLoad 运行 prompt；普通与托管都通过真实配置设置并声明 Native，关闭后声明 Unavailable。单进程运行中同步、活动 turn 取消、错误回读、模型改变、缺失控制、无效身份和计划确认由确定性 JSON-RPC 回归验证；不将 fixture 当作第三方实机证据。

所有 Cargo 阶段均动态领取到槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`，`CARGO_BUILD_BUILD_DIR` 同值，`CARGO_INCREMENTAL=0`。每阶段 owner 记录包含计划、任务、会话与 PID；阶段结束后移除自有锁，保留缓存。首轮 green.log 的 Host deny 断言失败后，修复原生 prompt 前的 Qwen DenyAll 屏障再重跑。final.log 的早期客户端探针受到既有 allow_always 影响，不计为三阶段验收；verified.log 及 final-pinned.log 使用受限关闭状态后均完整通过。

临时 example 不纳入仓库提交，最终源码保留在本地证据目录的 `qwen_full_access_probe.rs`。探针服务器和 CLI 进程均在阶段结束后退出；没有调用真实供应商上游。差异审查范围包括 Qwen 策略、生产启动/握手/会话同步及必要共享测试接线；未改安装、模型驱动、认证、品牌或总计划。`git diff --check`、22 行矩阵去重和本地链接检查通过。

最终 `cargo clippy --all-targets --all-features -- -D warnings` 在 final-pinned.log 退出 **0**（1m17s）；此前 clippy.log 同命令退出 0（1m52s）。最终生产代码、入口 pin 与临时探针均纳入最终 Clippy。最终 fmt 和 18+6 项回归退出码均为 0；24 项共享回调回归见 verified.log。自有槽位锁已释放，缓存保留。
