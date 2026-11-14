# 任务取消 API

`DELETE /v1/agent-apps/device-sync/tasks/{id}?projectId=...&targetHostId=...`
复用任务查询的身份、租户、项目、主机绑定，并要求任务方向对应的既有上传或下载授权。授权失败或不属于完整目标的任务返回 404，取消前不改变任务。

响应为 `SyncTaskCancelResponse`，设置 `Cache-Control: no-store`：

- HTTP 200，`{"kind":"accepted","status":...}`：排队任务立即取消；运行中任务仅请求取消，须继续 GET 轮询，待资源释放后才显示 cancelled。重复取消保持幂等。
- HTTP 409，`{"kind":"too_late","status":...}`：已进入提交临界区或已有其它终态，携带当时真实状态；提交中仍可能 running，继续轮询终态。不会把已提交结果改为 cancelled。
- HTTP 503：无法可靠读取或更新状态，或持久化记录仍活跃但当前进程没有执行所有权。HTTP 请求不初始化恢复、不重新执行任务。

复用领域层显式取消与提交互斥。重启后可读取的 cancelled 终态仍幂等返回 accepted，其它终态返回 too_late。

本轮依用户指令仅实现并审阅源码，未执行编译、单元测试、HTTP 测试或实机测试；未领取 Cargo 槽位，未设置缓存路径。统一验证需覆盖排队、传输、提交竞争、重复取消及越权隔离。
