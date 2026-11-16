# 任务6：工具流文本投影

已按用户覆盖要求完成代码迁移；保持当前目录和分支，未运行 Cargo、单元测试、实机测试或最终回归。编译、实际注册、通过/失败/ignored 数均未验证，退出码不适用；不标待回归。

## 断言映射

| 原宿主用例 | 领域 host_split_6_tests 同名用例 | 保留宿主契约 |
| --- | --- | --- |
| buffered_tool_event_starts_on_its_own_line | 用真实 tail、queue_delta、文本投影重放 Checking / the repo. / 工具 / Done，保留完整内容等值断言 | 请求42路由、实际队列/刷新入口；核对缓冲已清空、刷新标志复位、消息数量和助手角色 |
| active_workflow_update_appends_tool_block_and_preserves_message_id | 真实 upsert_workflow_tool_to_tail；保留 answer 后工具块前缀，核对单条消息、返回索引和 ID | 请求42的工作流入口、消息数量与宿主 ID 接线 |

仅迁出领域部分：宿主用例仍为2条，新增领域用例2条（静态候选，非运行清单）。删除宿主文本断言2行；未删除共享 message/agent_request 夹具或完整 App 初始化，夹具减少0行。宿主新增队列契约断言，因此物理行数不下降，不宣称测试数或夹具减量收益。

## 实现与静态审查

- 任务1证据为 evidence/baseline.md 与 before-host-tests.txt，非 task-1.md；已读取静态基线及注册链。
- 领域 stream_events 新增 prepare_buffered_tool_line，原宿主3行换行控制流原样下沉，queue_agent_tool_delta 真实调用；队列定时、请求匹配保留宿主。
- 工作流生产调用本已接到 upsert_workflow_tool_to_tail，不重复改接。
- lib.rs 以 cfg(test) 注册 host_split_6_tests；独立小型 ChatMessage 夹具，无反向宿主依赖，无 path/include 宿主源码，无新增依赖。
- 相邻领域用例覆盖普通 delta 和已有工具块替换；本组覆盖缓冲文本后的换行与已有普通回答后的首次工具块追加，未复制等价场景。
- 修改范围：领域 lib.rs、新测试及 stream_events.rs；宿主 stream_events.rs 与指定测试；本证据及自身任务文件删除。
- 本任务暂存区 git diff --cached --check 退出码0。整体 git diff --check 后续因其它任务的 system_settings_channels_ui_refresh_17_tests.rs 文件末尾空行退出2，未修改该无关文件。仅源码核对，没有执行任何行为验证。工作区其它任务变更未纳入本提交。
