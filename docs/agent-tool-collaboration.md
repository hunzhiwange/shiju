# AgentTool 团队协作

AgentTool 支持 `launch`（别名 `spawn`）、`run`、`status`、`wait`、`stop`、
`list_agents`、`send_message` 和 `followup_task`。任务身份为真实持久化 session ID。
`launch` 返回后父任务可以继续；`run` 等待结果。团队额度由共享协调器维护，
重新创建工具不会获得独立额度。嵌套等待临时释放执行许可，恢复时重新领取。

## 上下文与权限

`fork_turns=all` 默认继承父级有效消息快照；`none` 不继承对话。
角色工具与技能限制仍然生效，父级权限不能通过子任务扩大。
通信要求可信运行上下文、租户及真实父子关系；同团队父子和兄弟可通信，
跨团队读取和发送会被拒绝。发送方身份由运行时写入。
其他智能体的消息作为 user 数据进入模型安全边界，不能伪造 system 指令。

## 续派、等待与整合

`followup_task` 使用 `target` 和 `message`，只对当前父任务的存活子运行时排队。
上一轮结束后复用同一 session、保留历史、递增轮次并重置轮次取消令牌；
旧轮次完成不能覆盖新轮次。已取消目标不能续派。
`wait` 支持 any/all 子任务终态与 activity 信箱事件，并提供超时结果。
完成通知与通信在模型边界消费；已消费结果不重复注入。
Gateway processor 与 QueryEngine 工具循环在尚有子任务或未整合结果时拦截提前 final，
继续要求模型整合。达到轮次预算或取消会明确失败或取消，不能宣称成功。

## 与本地 Codex V2 对照

| 能力 | 对齐行为 | 有意不同 |
| --- | --- | --- |
| 启动与身份 | 独立任务、父继续、可检查状态 | 复用持久 session ID，不复制 Codex 路径身份 |
| history | 继承有效上下文，允许隔离历史 | 使用 Gateway 消息结构与现有存储 |
| control/mailbox | 同团队通信、边界消费、可信身份 | 进程内有界信箱，不实现跨进程投递 |
| 完成监听 | 事件驱动等待、结果整合门禁 | 共享协调器与既有两条模型循环 |
| 续派 | 同身份串行新轮次、保留历史 | 不从磁盘推断或重建旧运行时权限 |

对照来源为本地 codex-rs/core/src/agent/control/{mailbox,watch}.rs、
session/multi_agents.rs；这是行为对齐，不代表复制其模型能力。

## 进程重启边界

会话和消息可持久化；许可、信箱、取消令牌、完成游标和续派接收端只存在于当前进程。
重启后可读取历史，但不承诺恢复未完成执行、未消费通信或原团队续派权限。
运行时缺失时续派明确报错，不能假装已经恢复。

## 聚焦验收

`cargo test -p vw-agent --lib agcollab_ -- --nocapture` 覆盖协作协议、权限拒绝、
嵌套额度、取消、续派、Gateway processor 和 QueryEngine 的模型边界。
`agcollab_end_to_end_team_workflow` 用真实持久存储和 AgentTool 启动两个子任务，
通过屏障确认重叠执行及父继续工作，再验证父子/兄弟通信、同 session 续派和结果一次消费。
仅模型 stream 边界脚本化；它与两个入口的模型循环测试共同提供分层证据，
不将 stream 注入测试表述为真实网络或完整 Gateway HTTP 请求验收。
