# Hermes 完全访问接入

日期：2026-11-06。依赖批次 12 提交 `d9301b8b0` 已存在。
按用户最新要求不新增或运行测试，不申请账号、密码、BYOK 或受控上游。
有效测试数 0；以下区分固定源码确认与实机未测，不以缺少实机证据禁用普通入口。

## 固定发行源码（E2）

从 [PyPI 0.19.0 元数据](https://pypi.org/pypi/hermes-agent/0.19.0/json)
获取 [官方 wheel](https://files.pythonhosted.org/packages/e5/30/c85be8290e9565dc3c7a9720e93f3e59e09b1b163487be4946c3aa848f80/hermes_agent-0.19.0-py3-none-any.whl)，
内存读取并校验 SHA256 与元数据一致：
`bd0bac012aee38a60894781f4597dc29ee7bedb3448540249921f10d3bef327f`。
本机 uv 缓存的以下源码与该官方 wheel 逐文件 SHA256 相同：

- `acp_adapter/server.py`：`e780061d47fa6c874f89676afc75ab386b50f79bb269fdef38095245e8f32145`。
- `acp_adapter/edit_approval.py`：`589c4d8d538137249a132358bad0826316ea6d4e1bdbd6eec6c7d1af989b8661`。
- `acp_adapter/permissions.py`：`cff95b8e8a6be9369058dfa0b6937f56c60ec5361297eff987ba19f201a195cb`。

发行入口保持 `hermes acp`，wheel 为 py3-none-any，不代表各平台已运行。
[官方源码仓库](https://github.com/NousResearch/hermes-agent) main 已拆分 modes 模块；
本接入的精确语义以固定 0.19.0 wheel 为依据，不用浮动 main 推断版本效果。

`HermesACPAgent._session_modes` 公布 default、accept_edits、dont_ask，
分别映射编辑策略 ask、workspace_session、session。`set_session_mode` 只修改会话
mode 并保存，不替换模型、工具或提示词；返回空响应，没有 CurrentModeUpdate。
隐藏 `edit_approval_policy` 配置虽接受 ask/workspace_session/session，
但未在 configOptions 公布，响应为空列表，因此不主动使用该配置路径。
`should_auto_approve_edit` 即便 session 策略仍排除 .git、.ssh 和敏感文件名。
终端通过独立 `make_approval_callback` 进入 request_permission，模式不解除其审批。
故分类为部分支持，不能宣称全工具原生免审批或沙箱解除。

## 生产接入（E1）

`access_policy::hermes` 定义精确 mode 值；`AcpClient::sync_hermes_access`
在 new/load/resume 和每次 prompt 前读取当前会话公布的 modes。
只有同时公布 default 与 dont_ask 且当前 `full_access && ApproveAll` 时发送
`session/set_mode(dont_ask)`；关闭或只读批准时发送 default。
不把 accept_edits 当成关闭态，因为它仍放行工作区与临时目录编辑。
旧版或自定义入口没有公布控制时保留原有客户端访问路径；已知提升态却无法公布
关闭控制时明确报错。每次 prompt 重新发送当前意图，不依赖可能过时的 current mode。

使用现有 `confirmed_mode_control` 的超时、协议错误及 pending_controls 屏障；
空响应只记为请求获应答，绝不伪造当前 mode 回读或 Native 分类。
运行中手动 mode/config 修改先请求取消，等待结束再重试；无授权时拒绝非 default
mode 和隐藏编辑配置的提升值。`DenyAll` 在 prompt 入口直接拒绝原生执行。
终端和敏感编辑继续走现有客户端审批，供应商硬拒绝不被覆盖，不注入全局权限参数。
普通/托管、cli_local/session_model 复用该客户端路径；现有安装及模型来源门控不变。
应用切换授权继续复用包含 fullAccess/permissionMode 的缓存身份与旧进程停止机制，
按当前授权加载原 sessionId；本任务不承诺对已启动工具做瞬时撤权。

## 实机与验证边界

没有运行第三方 CLI，没有实际 initialize/new/load/resume/config/mode 报文、
审批次数、文件/终端结果、false→true→false 的执行验收或跨平台证据。
以上协议方法与值是发行源码摘要，不是捕获报文。工具执行和供应商实际撤销均未测。
用户明确跳过测试，未执行红绿测试，不以编译成功代替行为验收。

静态检查：动态领取槽位 1，
`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

- `cargo fmt --all`：退出码 0，仅保留本任务必要差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，20.01s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 测试命令未执行，有效测试数 0（用户明确跳过）。
- Cargo 子进程均已结束；owner 与槽位锁已删除，缓存保留。

审查仅涉及 Hermes 模式同步、授权保护、能力分类、证据与矩阵；
其他工作区变更未纳入提交。
