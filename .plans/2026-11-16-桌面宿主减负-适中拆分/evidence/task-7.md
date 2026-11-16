# 任务7：压缩状态消息投影

按用户覆盖要求完成实现并删除任务文件；保持当前目录和分支。未运行任何 Cargo 命令、单元测试、实机测试或最终回归；编译、实际测试注册、通过/失败/ignored 数均未验证，退出码不适用，不标待回归。

## 断言映射

| 原宿主用例 | 领域 host_split_7_tests 同名用例 | 保留宿主断言 |
| --- | --- | --- |
| compaction_status_started_then_completed_updates_one_message | 开始后单条 System、开始文本；完成后仍单条且内容为完成文本、索引相同、ID 为 None | 请求42真实入口、消息数量、消息 ID；新增 pending 索引建立与清除断言 |
| compaction_error_and_cancel_settle_active_and_background_status | 真实 settle_compaction 输入开始状态，分别取消/失败；保留无开始文本、存在目标结束文本断言，补充返回索引及 pending 清除 | 前后台 × 取消/失败真实入口矩阵；请求结束、active_agent_request 清空、后台不污染前台；保留目标消息容器非空与 pending 清除 |

仅迁出领域部分：宿主仍2条，领域新增2条源码候选；不声称实际注册数或宿主测试数下降。宿主移出5条角色/文本断言，消息数量和 ID 在宿主保留以验证接线。后台/前台共用同一领域投影，领域不重复会话路由矩阵。原错误用例的局部 chat clone/collect 夹具删除，App 和请求夹具保留。

## 静态审查

- 已读取任务1静态基线与任务6短证据。RustCodeGraph status 成功，图源码行号存在漂移，以其返回的当前源码确认调用链。
- 生产宿主 upsert_compaction_status 原已委托 desktop_session 的同名函数；handle_agent_compaction_status 负责前后台与存储，settle_interrupted_compaction 原已调用领域 settle_compaction。无需修改生产实现或扩大入口。
- 新模块由 desktop-session/src/lib.rs 的 cfg(test) 独立注册；宿主仍由 stream_events.rs 的 stream_events_tests 注册。仅源码核对，不代表编译确认。
- 相邻领域测试无同名压缩内容投影用例；agent_stream_tests 的压缩事件 JSON 映射属于传输解码，非等价覆盖。
- 把任务6最小 message 构造器原样移入 test_messages.rs，任务6和7两组真实复用；此额外文件及任务6导入修改是直接夹具/注册关系，无新依赖、无跨库框架、无反向宿主依赖或 path/include 宿主源码。
- 修改限领域 lib.rs、任务6夹具导入、新建任务7测试与 test_messages.rs，宿主指定测试、本证据及任务文件删除。plan.md / prompt.md 未修改。
- 全部行为验证按用户要求未执行；静态审查不能证明编译或运行通过。
