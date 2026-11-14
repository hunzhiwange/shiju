# 同步取消契约

任务 36 实现；按本轮指令未执行编译、测试或实机验证，未领取 Cargo 槽位。

- 授权调用者使用 `device_sync::cancel::cancel`，完整 target 与任务 ID 必须匹配。`Accepted` 和 `TooLate` 均携带当前状态；原 `tasks::cancel` 保持错误码接口。
- 排队未领取任务立即取消；运行中的任务先保持 Running，执行器唤醒并丢弃内联 future，释放请求、锁等待及私有 staging 后记录 Cancelled。禁止派生脱离执行器的后台传输。
- WebDAV/S3 上传的配置与技能 PUT 可取消，可能留下固定键的部分远端产物；manifest 发布前通过同一任务句柄的 `begin_commit` 原子拒绝已接受的取消。下载在本地 apply 前进入同一边界。
- 提交边界开始后拒绝取消，继续等待发布、验证、投影或回滚的真实结果。上传 manifest 成功即记录 committed；下载成功或回滚失败保留 committed 事实。
- 四个传输入口新增必需的 `SyncTaskExecution` 引用，由 `tasks::execute` 闭包提供。现有仓库尚无这四个入口的生产调用者，后续网关任务须将传输和成功基线持久化内联在该闭包中；只有传输成功才允许持久化基线。不得在传输外另行 select 取消。

后续统一验证需覆盖排队取消、GET/PUT 中断及临时资源释放、manifest 边界竞争、提交期间 too_late、回滚失败与已提交状态。
