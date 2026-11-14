# 同步任务执行记录

`device_sync::tasks::accept` 接收已授权、规范化的完整 target 和请求，返回带任务 ID 的状态。同一 target 的 request_id 在当前网关进程内去重；参数或 origin 改变返回 IdempotencyConflict。不同 target 不共享记录。读状态、取消、执行均需重新授权完整 target，任务 ID 不是授权凭据。

`execute` 只允许领取一次。调用方在闭包内调用已有 `run_with_sync_lock`，重读连接并执行 WebDAV/S3 上传或下载服务，不分离后台子任务。报告阶段按 WaitingForLock、InspectingRemote、Exporting、Transferring、Verifying、Previewing、Committing、Projecting、RollingBack、PersistingStatus 排序，可跳过不适用阶段。字节数累计不回退，已知总量不能消失或减少。

在首次远端写入或本地发布前调用 `begin_commit`；它与取消串行，取消先到则禁止提交，提交先到则取消返回 TooLate。成功发布后调用 `mark_committed`，后续失败保留 committed 事实。提交期间必须内联等待应用、回滚及投影，不能通过 drop/select 取消。取消运行中任务只是请求，执行器在 checkpoint 观察后退出；已排队任务直接取消。闭包仅返回安全错误码，投影及状态写入告警通过 warn 记录，不能用原始网络错误填充状态。

执行 future 被丢弃记录 Interrupted，提交期间被异常丢弃但尚未报告提交成功时 committed 仍为 false，不能据此推断远端完全未写入；恢复必须重新检查远端与安全快照。成功与失败结果仅由独占执行器写一次，结束后拒绝进度更新。

记录保存在进程内，不构成持久任务库；重启后不提供跨进程幂等或任务恢复保证。调度、API 路由与持久恢复由后续任务接入，本任务不增加平行数据库。

本轮按用户要求只实现并审阅代码，未编译、未运行测试或实机验证；未领取 Cargo 槽位，未设置 Cargo 缓存路径。后续统一验证需覆盖幂等冲突、并发领取、跨作用域拒绝、字节回退、取消/提交竞争和中断语义。
