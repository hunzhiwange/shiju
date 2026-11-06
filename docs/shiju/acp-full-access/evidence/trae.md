# TRAE ACP 权限接入

日期：2026-11-06。依赖批次 25 提交 `76e2a6505` 已存在。
用户最新要求跳过 TDD、自动化测试和账号实机验收；本次未启动 CLI、读取凭据或连接模型。

## 来源与版本边界

基线为 `0.121.3`，历史发布查询 `v0.121.3`，版本文本来源、响应哈希见
[初始证据](README.md#trae)。本次没有下载或执行发行包，运行版本、平台和制品哈希未知；
未取得固定版本权限源码，不宣称权限 E2/E3。TRAE CLI 与开源 trae-agent 是不同入口。

本次读取[官方 ACP 文档](https://docs.trae.cn/cli_agent-client-protocol)：
正常入口为 `traecli acp serve`，也支持 `--acp`；全局 permission-mode 位于 acp serve 前，
文档列出 default、auto、bypass_permissions。服务参数 `-y/--yolo` 跳过确认并关闭沙箱。
文档目前属于 CLI 2.0，不能外推为基线 0.121.3 的能力或实际 ACP 握手。
本次没有确认标准会话 config/mode 权限 ID，故不杜撰会话配置，不自动注入启动 bypass。

注册表的普通及本地入口保持原样；TRAE 无 managed_runtime 契约。
session_model 保持原有平台认证与独立运行根门控，不把权限接入扩展为认证、模型来源或安装改造。
cli_local 正常访问不因缺少实机证据而禁用，不修改用户全局配置、模型、工具或提示词。

## 生产接入（E1）

- `build_event_client` 为 TRAE 启用 transient_permissions，复用已有生产审批解析器。
  去掉 AllowAlways 后按当前 permissionMode 处理，ApproveAll 只选择 AllowOnce，
  无一次性允许项时拒绝或取消，不把可撤销的当前授权写成供应商持续规则。
- `AcpClient::access_policy` 对 TRAE 明确使用客户端能力分类，不因公布 modes/configOptions
  升格为原生支持。开启且 ApproveAll 为 ClientOnly；关闭为 NotRequested；
  ApproveReads 为 Partial；DenyAll 为 Unavailable/DeniedByHost。
- `run_actor_prompt_with_controls` 在 DenyAll 时拒绝 TRAE prompt，避免原生工具不回调时绕过
  host deny。文件、终端、取消及会话绑定检查沿用现有处理器。
- `launch_args` 补齐 TRAE 的 `-y` 授权检测。`-y`、`--yolo` 和显式
  `--permission-mode bypass_permissions`（含等号形式）要求当前 full_access 与 ApproveAll。
  当前授权下允许用户显式传入，不自动追加，也不接受其绕过 host deny。
- 开启、关闭及恢复沿用 fullAccess/permissionMode 缓存身份变化、停止不兼容旧客户端及
  当前授权恢复逻辑。new/load/resume 保持既有协议路径，不写入永久批准。
  这撤销客户端自动批准，不保证原生配置已有 allow、已执行工具或旧 CLI 沙箱瞬时撤销。
  官方启动 bypass 不是已核验的会话级撤权控制，不宣称已接入原生免审批。
- 没有新增原生控制调用，无需重复后端模块；按用户要求不新增测试文件。

## 静态验证与限制

有效测试数 0，按用户要求未运行测试与实机验收。
未采集 initialize/new/load/resume/config/mode 报文、审批次数或文件/终端结果。
false→true→false、恢复、deny、显式 bypass 的实际效果及跨平台覆盖均未实机验证。
完成仅指生产代码接入，静态检查不替代供应商执行证据。

动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`；
`CARGO_BUILD_BUILD_DIR` 使用同一路径，避免仓库 build-dir 配置造成共享写入。

- `cargo fmt --all`：退出码 0，仅保留本任务必要差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，23.06s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 未运行测试，有效测试数 0（用户明确跳过）；无新增 features。
- Cargo 与其子进程结束后，已清理自己的 owner 与槽位锁，保留缓存。

审查差异仅包含 TRAE 客户端能力分类、逐次批准、host deny、短参数授权检查及本证据/矩阵。
没有修改只读 plan.md，工作区其他任务变更不纳入提交。
