# OpenClaw ACP 权限接入

日期：2026-11-06。依赖批次 23 提交 `7e3291f32` 已存在。
用户明确跳过测试驱动、自动化测试和账号实机验收；本次未启动 OpenClaw、申请凭据或连接模型。

## 来源与边界

- 普通入口 `openclaw acp`；托管契约 `openclaw@2026.10.10`、入口 `openclaw.mjs`。
  契约入口 SHA256 为 `469e60c9f60b615ff71c985a54238ee532a8f949866d76aebe7d6e4bc7219902`。
  从 npm 下载固定发行包只读校验，SHA512 与契约 integrity 一致，入口 SHA256 也一致；未安装或执行脚本。
  不更改安装、模型来源、认证、现有技能隔离补丁或版本门控。
- [官方 ACP 文档](https://docs.openclaw.ai/cli/acp) 说明这是 stdio 到 Gateway WebSocket 的桥接，
  执行审批转为 ACP `session/request_permission`；不调用 ACP 客户端文件和终端方法。
- 静态读取官方 `v2026.10.10` 源码（E2），没有执行供应商进程：
  [permission-relay.ts](https://github.com/openclaw/openclaw/blob/v2026.10.10/src/acp/permission-relay.ts)
  提供 `allow-once/allow_once`、`allow-always/allow_always`、`deny/reject_once`。
  允许项由 Gateway 的 `allowedDecisions` 过滤，缺省仅 allow-once 与 deny。
  SHA256 `d9f3ed86322ce9c8de3329fddd0bccbd9f0ffe0fc405b42f9949f7e65ec24c4f`。
- [translator.agent-events.ts](https://github.com/openclaw/openclaw/blob/v2026.10.10/src/acp/translator.agent-events.ts)
  对活动 prompt 的审批取 `exec.approval.get`，向客户端请求，然后调用 `exec.approval.resolve`。
  取消/失败路径回落 deny，不能用客户端应答替代网关最终执行判定。
- [translator.session-lifecycle.ts](https://github.com/openclaw/openclaw/blob/v2026.10.10/src/acp/translator.session-lifecycle.ts)
  实现 new/load/resume；legacy mode 修改 thinkingLevel，不能当成权限模式。
  config 设置调用 `sessions.patch`，回读可带 overrides；不是独立执行验收。
- [translator.presentation.ts](https://github.com/openclaw/openclaw/blob/v2026.10.10/src/acp/translator.presentation.ts)
  公布 `elevated_level=off/on/ask/full`。
  不自动设置该字段：它改变 Gateway 主机执行边界，不能将其等同于保留 deny 的工具批准开关。
  不发送 ACPX 插件 approve-all 配置；那是 OpenClaw 启动外部 harness 的另一层。

## 生产接入

分类：**客户端自动批准**，未宣称原生免审批已验证。

- `build_event_client` 将 OpenClaw 接入已有 transient_permissions：仅逐次批准，过滤 AllowAlways，
  避免本次可撤销授权产生 Gateway 持久 allowlist。没有 AllowOnce 时不伪造允许项。
- `AcpClient::access_policy` 对 OpenClaw 使用 ClientOnly；ApproveReads 为 Partial，DenyAll 优先。
  公布的思考/输出/elevated 控件不作为已验证原生权限依据。
- `run_actor_prompt_with_controls` 在 host DenyAll 下拒绝 prompt，覆盖无需审批即可执行的网关工具。
  正常授权路径保持可用；不因缺少实机证据禁用后端。
- false→true→false 按当前 permissionMode 应答。现有缓存 key 包含 fullAccess/permissionMode，
  会话所有者通过 `stop_incompatible_cached_clients` 停止不兼容旧客户端再恢复逻辑会话。
  不保证同进程热撤销，不修改用户已有网关配置、elevated 设置、沙箱或批准规则。
- 普通/托管、cli_local/session_model 复用公共路径与既有门控。
  关闭完全访问不等于撤销用户原来授予 Gateway 的权限；网关执行和客户端文件边界是两层。
- 仅复用已有生产调用，无需新建后端模块或无调用者抽象；按用户要求不新增测试文件。

## 静态检查与限制

未运行测试，有效测试数 0；没有红绿测试或真实协议摘要。
新建、加载、恢复、开启/关闭、审批次数、文件/终端效果与平台覆盖均未实机验收。
矩阵最新版本沿用之前查询，不代表本次运行版本。

动态领取槽位 **1**，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-1`。

- `cargo fmt --all`：退出码 0；仅提交本任务必要差异。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0，22.29s。
- `git diff --check -- crates/vw-acp docs/shiju/acp-full-access`：退出码 0。
- 有效测试数 0（用户要求不运行），未实机验收。
- 子进程全部结束，自己的 owner/槽位锁已释放，缓存保留。

审查差异仅含客户端逐次批准、host deny、能力分类、独立证据和矩阵；只读 plan.md 未修改。
