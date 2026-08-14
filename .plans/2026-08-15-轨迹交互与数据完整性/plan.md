# 轨迹交互与数据完整性计划

目标：补齐参考实现中的可点击轨迹控制、紧凑时间块与折叠交互，并消除最新步骤原始轨迹偶发缺失。

范围：修复 LLM 原始步骤载荷的落盘顺序和错误可观测性；增加单步骤原始载荷 API 与有限重试；记录真实工具生命周期时间；扩展轨迹投影、Duration/Turns/Calls 控制、可点击色块、提示框及轮次/工具折叠；完成暗黑主题和回归验证。

范围外：不复制 DeepSeek Harness 的缩放拖拽、区间选择或嵌套子工具拓扑；不迁移旧历史补造工具时间；不重构无关聊天、存储或网关协议。

假设：当前 `session_step_llm_raw` 已支持 SQLite/PostgreSQL 按步骤 upsert；Assistant 有可信 step 起止时间，Input 默认没有持续时间；只有新增工具生命周期事件后，Tools 才能按真实耗时缩放，旧数据继续使用最小可见宽度。

## 设计决策

- 参考 `/Users/Shared/work/dir/data/codes/deepseek-harness/packages/client/ui-trajectory` 的三泳道、工具栏、折叠和点击行为，但沿用 Iced 与现有轨迹领域模型。
- 选择“原始载荷先落盘，再发布 `StepFinish`”；落盘失败记录 session/step 警告，但不因可观测性工件失败而中断模型响应。
- 选择新增带时间戳的类型化工具生命周期事件并持久化到 session step；拒绝用相邻模型步骤或消息顺序推算工具耗时。
- Duration 在“真实耗时”和“等宽”间切换；未知时长始终保留最小可点击块，提示框明确显示不可用。
- 搜索结果临时越过折叠过滤但不改写折叠状态；选择隐藏记录时展开其所属轮次和工具组。

## 架构说明

- 原始载荷写入归 `vw-agent` processor/ui_store 所有；读取经专用 gateway handler 和 `vw-gateway-client`；桌面只做有限重试。
- 工具时间由 processor 事件产生，经 gateway SSE 到 desktop，并保存在 `vw-shared::session::ChatSessionStep`；轨迹投影只消费持久化事实。
- 瞬时折叠和显示模式保存在桌面 App 状态，不写入会话；所有新增 Iced 样式必须覆盖暗黑主题，滚动条保持 4px。

## 开发策略

- 对会改变行为的代码，使用失败验证-通过验证-重构。
- 在实施步骤前规划失败验证命令和预期失败。
- 将实施步骤限制在失败测试所证明的行为范围内。
- 测试真实行为，不验证模拟对象或实现细节。
