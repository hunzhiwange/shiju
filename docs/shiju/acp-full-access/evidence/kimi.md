# Kimi ACP 权限接入

日期：2026-11-06；任务 16。依赖批次 15 的提交 `10629e876` 已存在，共用会话控制、客户端审批及恢复路径可用。根据用户更新的授权，跳过测试驱动、自动化测试、账号及受控上游验收；本次不运行外部 CLI，不读取凭据、不修改全局配置。**工具执行效果、审批次数、运行中撤销和跨平台效果均未实机验收。**

## 官方来源与源码确认（E2）

旧 [MoonshotAI/kimi-cli](https://github.com/MoonshotAI/kimi-cli) 已归档，官方指向新版 [MoonshotAI/kimi-code](https://github.com/MoonshotAI/kimi-code)。本次固定阅读提交 `480019652d029623fa31133cfa613adf9afa1417` 的下列源码，未把旧版 Python CLI 的 yolo 参数套到新版 ACP。

- [ACP CLI 入口](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/apps/kimi-code/src/cli/sub/acp.ts)：`kimi acp` 启动 `@moonshot-ai/acp-server`，与交互 CLI 入口分离。仓库现有普通注册与托管入口均沿用 `kimi acp`。
- [modes.ts](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/packages/acp-server/src/modes.ts)：模式为 `default/plan/auto/yolo`，分别映射 manual、规划+manual、auto、yolo；切换包含计划状态，不能隐式替换用户的 plan。
- [config-options.ts](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/packages/acp-server/src/config-options.ts)、[server.ts](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/packages/acp-server/src/server.ts)：new/load/resume 返回 modes 和 configOptions，`configId=mode` 接受上述精确值，响应带最新 configOptions。
- [session.ts](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/packages/acp-server/src/session.ts)：setMode 设置引擎 permission 并发布 current_mode_update、config_option_update；本身不切换 model/thinking。
- [approval.ts](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/packages/acp-server/src/approval.ts)、[question.ts](https://github.com/MoonshotAI/kimi-code/blob/480019652d029623fa31133cfa613adf9afa1417/packages/acp-server/src/question.ts)：普通审批使用 approve_once/approve_always/reject，always 写入 session 范围规则；计划与问答共用 request_permission，但使用独立 option ID。

查询 [npm 发布元数据](https://registry.npmjs.org/@moonshot-ai/kimi-code/latest) 得到 2.5.2，integrity 为 `sha512-a7UuugqQuPBHB2ZMiu8ezAFIwJ0dn6xanuRnTqa2pXXu12ng7kvigAlXy47TxNKpnyj8vgaNStedTv4kbqSCuA==`。这仅是发布身份，不代表安装、运行、制品哈希回读或该制品与上述源码的一致性已验证。没有实际发行包/平台执行证据（E3），不宣称原生免审批已验收。

## 生产接入（E1）

`access_policy/kimi.rs` 定义精确模式解析与审批通道识别；`client/kimi_access.rs::sync_kimi_access` 接到共享同步入口，在 new/load/resume 后及每次 prompt 前按当前 full_access + ApproveAll 设置 `mode=yolo`，关闭或读取审批设置 `mode=default`。仅使用会话公布的配置，旧版或自定义入口未公布配置时保留现有客户端审批；已处于高权限却无法恢复 default 时明确报错。

同步复用超时、设置响应当前值确认与失败屏障；服务端拒绝不回退到猜测参数。保留 plan，开启完全访问不自动退出计划；响应改变 model/thinking 等其他配置时阻止继续。guard_kimi_mode 要求高权限模式具有当前授权，活动 turn 先取消并要求完成后重试。DenyAll 在 prompt 前拒绝原生执行，不能被 full_access 覆盖。

不自动追加启动 yolo。显式启动 bypass 要求移除后使用会话控制，避免启动权限脱离撤销路径。普通、本地和托管入口均使用共用同步；已有 cli_local/session_model 配置、认证与能力门控保持现状，不因缺少实机证据禁用正常入口。

客户端批准复用现有 transient resolver，只选 allow_once，避免持久授权。Kimi 问答、计划及未知允许选项返回明确交互错误，不把答案当工具审批。新增字段仅同步更新既有测试构造器以供 all-targets 编译，未新增或执行测试。

能力显示为 Partial / UnverifiedControls（公布配置处于 yolo 时）；缺少原生配置保留客户端分类。不开企业或沙箱绕过，不声称覆盖供应商硬拒绝。协议脱敏摘要来自固定源码：new/load/resume → mode 配置，set_config_option(mode, default|yolo) → configOptions 当前值；本次没有协议抓包或审批计数。

## 静态检查

按用户要求未运行测试，有效行为测试数不适用。Cargo 阶段动态领取槽位 2，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-2`，检查后清理自己的锁并保留缓存。先执行 `cargo fmt --all`，再执行 `cargo clippy --all-targets --all-features -- -D warnings`；默认 workspace 全 features/all targets，仅编译与静态检查，不运行测试。两轮 `cargo fmt --all` 与 Clippy 均退出 0；最终 Clippy 在 18.13 秒内完成。`git diff --check` 退出 0。临时日志为 `/tmp/kimi16-fmt.log`、`/tmp/kimi16-clippy.log`，不保证长期保留。全部阶段使用槽位 2，已释放锁。建议下一步在正常登录环境试用完全访问开关。
