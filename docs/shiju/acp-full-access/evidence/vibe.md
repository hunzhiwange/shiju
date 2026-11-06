# Vibe 完全访问接入

日期：2026-11-06。依赖任务 9 提交 `bb5397c5b` 已存在。

## 范围与结论

分类：客户端自动批准 / 部分支持。完全访问沿现有 `full_access` 与
`PermissionMode::ApproveAll` 传递给客户端文件、终端及标准审批回调。
不自动把 Vibe 的 agent 切到 `auto-approve`，也不添加 CLI `--auto-approve`。
普通、托管、cli_local/session_model 复用同一 ACP 客户端路径及现有模型门控。
未运行供应商进程或工具验收，不宣称原生免审批已实机验证。

用户最新要求覆盖原任务测试驱动和账号验收：不新增或运行测试，不申请凭据，
只完成可用权限接入与静态检查。有效测试数 0（按要求跳过），没有红绿测试或 E3。

## 官方依据与固定源码

- [官方权限说明](https://docs.mistral.ai/vibe/code/safety-approvals-permissions)：
  `auto-approve` 为 agent，管理员可通过 `enable_auto_approve=false` 禁用；
  agent 可覆盖模型、系统指令、工具与权限。
- 官方 GitHub `mistralai/mistral-vibe` 的 `v3.0.4`、`v3.1.0` 源码归档已静态核对。
  [3.0.4 ACP](https://github.com/mistralai/mistral-vibe/blob/v3.0.4/vibe/acp/agent.py)、
  [3.1.0 ACP](https://github.com/mistralai/mistral-vibe/blob/v3.1.0/vibe/acp/agent.py)：
  `set_config_option(mode)` 和 `set_session_mode` 均调用 `agents.switch`；
  `new/load` 返回 mode、model、thinking 配置，mode 值来自 agent 名称。
- [内置 agent](https://github.com/mistralai/mistral-vibe/blob/v3.0.4/vibe/core/agents/models.py)：
  `auto-approve` 设置 `bypass_tool_permissions=true`；`ask` 不添加工具放行覆盖。
- [注册表](https://github.com/mistralai/mistral-vibe/blob/v3.0.4/vibe/core/agents/registry.py)
  允许用户 agent 覆盖内置同名 agent。ACP 不公布完整 profile，不能只凭
  `auto-approve`/`ask` 名字断言模型、提示词和工具不变。
- 源码归档 SHA-256：
  - v3.0.4 `f3b75c360040131c97b2324ffdcd2fca089049fd9670b7ace867c9521d637c2f`
  - v3.1.0 `284f99f9b9d6386e65ca4ef0ca4775776f796b3b053ab3bea2589f5fdc3ac6e9`

以上为源码确认（E2），不等于已安装发行包身份或协议运行证据。未读取用户配置或凭据。

## 权限开启、关闭和恢复

- false→true→false：现有客户端授权随当前会话意图变化，不修改 agent、全局配置或信任目录。
- 用户显式选择 `auto-approve`（config `mode` 或 legacy mode）须有当前完全访问授权；
  上游的未公布模式或管理员拒绝仍通过原协议报错，不伪造允许结果。
- new/load/resume 与每次 prompt 前检查当前 mode/config；无完全访问授权但仍处于
  `auto-approve` 时拒绝继续，提示用户显式选择 `ask` 等审批 agent。
  不隐式替换 profile，不新建逻辑会话。未撤销成功不会继续执行工具。
- 活动 turn 中切换 agent 先取消并要求完成后重试；`DenyAll` 在 prompt 入口拒绝原生执行。
- 启动 bypass 参数显式拒绝，避免无法随会话授权撤销的进程级放行。
- 正常/未知版本或自定义 agent 保留现有 ACP 与客户端审批路径。自定义 profile 的原生
  权限无法仅由名称识别；不宣称覆盖所有自定义 agent/企业规则/原生沙箱。
- 无脱敏协议摘要、审批计数或平台执行覆盖：按用户要求未运行真实协议验收。

## 静态检查

- 动态领取槽位 2；`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`。
- `cargo fmt --all`：退出码 0；只提交本任务格式化差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0（21.59s）。
- `git diff --check`：退出码 0。
- 未运行测试，有效测试数 0；未进行发行二进制、账号或模型上游验收。
- 子进程全部结束后已释放自己的槽位锁，保留编译缓存。
