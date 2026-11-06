# PI ACP 权限接入

日期：2026-11-06。依赖批次 24 提交 `fb1dd84d4` 已存在。
用户明确跳过测试驱动、自动化测试、账号实机验收；本次未启动 Pi、读取凭据或连接模型。

## 固定发行源码（E2）

只读下载 [pi-acp 0.0.22 元数据](https://registry.npmjs.org/pi-acp/0.0.22)
与[发行包](https://registry.npmjs.org/pi-acp/-/pi-acp-0.0.22.tgz)，SHA512 与 npm integrity 一致。
包 SHA256：`963dab8dd814b29141ff9a1d496fe273a2470feb85435b933c16130b49682c02`。
入口 `dist/index.js` SHA256：`f9a00a801269ecaabbfb977cdabac6dae9dacd1f8f27e39905217cf84b4b87d5`。
发行入口为 `pi-acp`，启动独立 `pi --mode rpc --no-themes`；适配器版本不代表原生 Pi 版本。
官方适配器仓库为 [svkozak/pi-acp](https://github.com/svkozak/pi-acp)，原生仓库
[badlogic/pi-mono](https://github.com/badlogic/pi-mono) 当前重定向 earendil-works/pi；不更改仓库身份或安装配置。

固定包 README 的 Limitations 明确文件和终端不委托 ACP，由 Pi 本地执行。
`dist/index.js` 没有 requestPermission/readTextFile/writeTextFile/createTerminal 调用；
工具开始和完成仅映射为 tool_call/tool_call_update 通知，通知不能当成执行前权限闸门。
`setSessionMode` 调用 `setThinkingLevel`；off/minimal/low/medium/high/xhigh 都是思考级别，
不可当成权限值，不自动修改。没有权限 config/set 接口可供开启或撤销原生访问。
initialize 公布 loadSession；new/load 创建或重新附着 Pi RPC 进程；resume 使用本仓库既有回退。
这些是源码摘要，未捕获实际协议。

现有托管补丁 `patches/acp-model-source/pi/pi-acp-0.0.22-state-dir.patch` 仅处理私有状态目录
和技能/扩展隔离，没有新增权限回调或沙箱。现有模型来源驱动、版本校验和技能隔离保持原样。
历史模型来源验收见 `docs/shiju/acp-model-source/evidence/pi.md`，不能替代本次权限验收。

## 生产接入（E1）

- `AcpClient::access_policy` 明确将请求完全访问且 ApproveAll 的 PI 分类为 Partial，
  原因为 NoAdvertisedControls。不会把思考选项误认成未验证权限开关，更不会宣称 Native。
  false 保持 NotRequested，DenyAll 保持 Unavailable/DeniedByHost，ApproveReads 保持 Partial。
- 保留普通、托管、cli_local/session_model 访问路径；不因未实机验收禁用后端。
  不注入虚构的 yolo、权限参数或全局设置，不改变模型、工具与提示词。
- `run_actor_prompt_with_controls` 在 DenyAll 下直接拒绝 PI prompt，防止本地工具绕过客户端拒绝。
  ApproveReads 无法限制 Pi 原生写入/执行；关闭完全访问也不能撤销 Pi 进程本来的系统权限。
  用户要求保留正常访问，因此明确披露边界，不假装具备原生沙箱撤销能力。
- `build_event_client` 为 PI 使用现有 transient_permissions。0.0.22 本身不请求审批，
  若自定义/后续入口请求 ACP 审批，只选择 allow_once，不写入持续批准规则；DenyAll 优先。
  客户端文件请求仍按 full_access 检查路径，终端按 permission_mode 检查。
- false→true→false 通过现有 fullAccess/permissionMode 缓存身份和
  `stop_incompatible_cached_clients` 停止同一 runtime_identity 的旧客户端再恢复逻辑会话。
  此处撤销客户端授权，不等于限制 Pi 本地工具；不承诺正在执行工具的瞬时撤权。
- 没有需要新增后端模块的原生控制调用，复用公共机制；按最新指示不创建测试文件。

## 静态检查与实机边界

没有运行测试，有效测试数 0。没有实际 initialize/new/load/resume/mode/config 报文、
审批计数、文件/终端结果或 false→true→false 的执行验收；各平台权限效果均未实测。
矩阵最新版本沿用基线查询，不是本次运行版本。静态检查不能替代工具执行验收。

动态领取槽位 1，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

- `cargo fmt --all`：退出码 0，仅保留本任务必要格式差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，22.04s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 测试命令未执行，有效测试数 0（用户明确跳过）。
- Cargo 及其子进程已结束，自己的 owner 与槽位锁已释放，构建缓存保留。

审查差异仅含 PI 能力分类、host deny、逐次回调批准、矩阵及本证据。
未修改只读 plan.md；其他任务变更不纳入提交。
