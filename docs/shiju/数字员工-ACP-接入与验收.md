# 数字员工 ACP 接入与验收

## 从员工列表开聊

1. 登录 ShiJu，连接本机网关。
2. 打开「数字员工」，在私有或共享员工列表中点击员工的「聊天」。桌面会在原聊天窗口建立员工会话。
3. 发送文本。在原会话中继续提问，关闭聊天后可从会话历史重开。要换员工，从员工列表另开会话。
4. 回复过程中点击「停止」。看到取消结果后，可以在同一会话发送下一条。

本机网关须启用内置「数字员工」智能体（`acp-employee-acp`）。员工须已发布并允许当前账号访问。共享员工还需要已绑定、就绪的 QwenPaw Worker。网关必须直接监听回环地址且未启用隧道。网关通过 IAM 验证 ShiJu 登录凭据，并将员工会话绑定到 IAM 返回的账号。本地网关的用户 ID 和公司 ID 用于本地会话分区。关闭本机网关 SKey 鉴权时可使用默认本地连接。产品网关仍用 ShiJu 登录凭据执行公司与员工访问检查。

## 发布产物与部署版本

`employee-acp` 随现有 `vw-cli` 的 `shiju` 二进制发布。CLI 压缩包和带 CLI 的桌面安装包沿用各平台打包入口。macOS 应用包中的路径为 `Contents/MacOS/shiju`。可运行 `shiju employee-acp --help` 核对随包二进制是否包含该命令。实际聊天由本机网关启动这个子命令并通过标准输入输出交换 ACP 消息。

桌面和配套网关须来自包含员工列表聊天入口与内置 `employee-acp` 的同一构建。版本号相同时，仍需核对打包提交或子命令。Agent Cloud 至少须包含会话 API 提交 `e7fdf74`、Matrix 别名修复 `095ec8c`，并已应用 `20261121_add_shared_employee_conversations.sql`。可信 HTTPS 产品网关须允许当前 ShiJu 登录态访问 `/sj/agent/api/v1/` 下的会话、消息和取消接口。

网关保管请求期登录凭据，并仅注入对应员工子进程。回环 HTTP 替身入口需要显式启用 `employee-acp-test-support`，且只在启用调试断言的构建中生效。GitHub 发布所用的 `dist` profile 继承 `release`，因此全特性发布构建也不编入替身入口。

## 正文和停止语义

新版 QwenPaw Worker 直接传递可见正文。Agent Cloud 以 SSE 推送指定轮次的正文快照，员工 ACP 将新增部分转为 `agent_message_chunk`，断线重连时复用已显示正文以避免重复。轮次完成时补齐最终正文，重新打开会话时读取已确认的历史。未声明 SSE 能力的部署继续使用历史接口。

流式需要升级 Agent Cloud 和 QwenPaw Worker，并应用 `20261125_add_conversation_reply_progress.sql`。私有直聊经 Controller 查询时也要升级 Controller。验收时请用较长回复确认生成中出现多个正文片段，且最终正文没有重复。

桌面停止请求经网关到达 ACP 子进程。子进程通过产品取消接口确认轮次状态，聊天随确认结果结束。如果产品已先完成，仍可收到最终正文。产品确认取消表示该轮以取消状态结束，晚到回复不应成为该轮正文。要确认 Worker 是否实际中止推理，须单独核对 Worker 侧执行记录。

## 三类验收证据

| 环境 | 复验范围 |
| --- | --- |
| 隔离回环 | 在仓库根目录执行下方命令。通过普通聊天 HTTP 路由与真实 ACP 宿主启动 Cargo 生成的 `shiju employee-acp`。回环替身校验账号、员工、轮次、取消和提交键。 |
| 本地产品与模拟 Worker | 在 Agent Cloud 的 `control-plane` 目录，以初始化了会话结构的临时 PostgreSQL 执行 `go test -count=1 -run '^TestEmployeeACPProductRoundTrip$' -v ./tests`。通过受控环境变量 `EMPLOYEE_ACP_TEST_DATABASE_URL` 提供临时连接。该测试覆盖实际产品接口、数据库、模拟 Matrix 与 Worker 回包，并查询新增 Task 和 Attempt 均为零。 |
| 目标部署与实际桌面 | 使用已授权的可写测试租户、包含上述接线的桌面及网关、已发布的私有和共享员工 ID、共享员工实际 QwenPaw 镜像版本、两位有权限的用户及一位无权限或异公司的用户。 |

```bash
python3 scripts/cargo_slot.py cargo test -j 1 -p vw-cli --features employee-acp-test-support --test employee_acp_roundtrip
```

本轮采用本地模拟 Agent Cloud，真实租户验收延期到后端发布后。跨进程用例通过了同一员工会话的网关重启恢复、已提交消息的原键重试、旧绑定记录迁移、双账号隔离、撤权复核和取消后续聊。错账号注入返回 HTTP 401。macOS 桌面构建和受控 UI 验收可在仓库根目录执行：

```bash
python3 scripts/cargo_slot.py cargo build -j 1 -p vw-desktop --bin shi-ju
python3 scripts/cargo_slot.py cargo test -j 1 -p vw-desktop --lib employee_acp_desktop_
```

桌面用例通过实际员工选聊消息入口验证私有和共享聊天请求、重开、停止及账号切换后的迟到结果，并通过 Iced 检查明暗主题下「聊天」按钮的绘制和点击。

### 2026-11-22 本地验收记录

- 槽位 3 编译了公共网关、`shiju` 和跨进程用例。错账号注入退出 101，具名用例收到 HTTP 401。清除注入变量后退出 0，具名用例通过 1 项、失败 0 项、辅助 seed 忽略 1 项。重开修复见 `cc1974a6c`。
- Agent Cloud 本地产品测试使用 PostgreSQL 17 与模拟 Worker，三个子项均通过。数据库实测新增 Task 与 Attempt 均为零。
- 复用槽位 6 的 macOS arm64 桌面构建 `target/rust-slot-6/debug/shi-ju`。共享工作区随后重编在电脑操作设置页 `computer_control.rs:17` 遇到 E0283，退出 101，保留先前产物。隔离源码副本避开该处类型歧义，员工实现保持同版本。桌面定向测试退出 0，私有、共享和明暗主题绘制及点击共 3 项通过，见 `e1f712811e`。

真实部署按以下顺序复验。

1. 分别从私有和共享员工列表点击「聊天」，各完成两轮，再关闭并从历史重开。核对员工和会话未变，检查实际聊天画面。
2. 两个授权账号同时向同一共享员工发送各自唯一的测试标记。下一轮分别要求复述，确认两人只看到自己的标记。用负向账号验证访问被拒绝，再撤销一个既有账号的员工权限并重开其会话。
3. 停止一轮可控的长请求。核对取消状态、晚到回包、原会话下一轮，以及该租户内本轮验收产生的 Task 与 Attempt 增量均为零。
4. 交付记录仅保留脱敏版本、结果和时间。原始账号、消息、云端标识和凭据仅保存在受控位置。
