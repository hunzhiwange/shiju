# 会话聊天性能重构计划

## 目标

让长会话的打开、向上翻页、滚动、输入、流式输出和持久化都保持有界工作量，彻底移除随历史消息总量或当前回复累计长度线性放大的 UI 热路径。

本计划不把“Rust 写的”视为性能保证。Rust 能降低单次执行成本，但无法修复全量加载、全量克隆、全文哈希、全文解析、全索引重建和全表重写造成的复杂度问题。

## 范围

覆盖会话消息增量存储、Gateway 游标页与 delta 协议、Desktop 分页时间线、活动消息 overlay、帧级 delta 调度、revision 局部渲染、虚拟高度快路径、增量保存、重缓存边界和长会话回归。

## 假设

- 现有 `session_messages.seq_no` 在单个租户/项目/会话内稳定递增，可作为内部页边界；外部游标仍保持不透明。
- 现有完整 UI 会话端点可暂留作兼容、导入和显式结构操作，但 Desktop 正常热路径可以切换到新增有界接口。
- Iced 继续作为桌面 UI 框架；性能目标通过修正数据与渲染复杂度实现。

## 真实问题边界

当前 VibeWindow 已经有虚拟列表、Fenwick 高度索引、32 条分块预热和 50 ms 流式合并，但以下路径仍会把局部变化放大成全量工作：

1. 打开会话时，存储层、Gateway 和 Desktop 都会获取完整消息历史。
2. 流式期间约每秒保存一次完整 `ChatSession`，存储层删除该会话全部消息后再逐条插入。
3. 每批流式增量会克隆不断增长的完整 `ChatMessage`，并可能全文哈希、全文解析和重建完整渲染缓存。
4. 虚拟列表在消息数量不变时仍可能重建整棵高度索引。
5. `SessionRuntimeState`、完整聊天快照和重型 Markdown/编辑器/工具缓存会在视图或预热路径中被不必要地复制或长期保留。
6. 异步加载、测量和动作回调缺少统一的稳定条目身份、generation 与 revision 约束，分页后容易把结果落到错误消息。

因此核心问题不是 Iced 或 Rust 本身，而是正常路径仍包含 `O(历史消息数 + 当前回复累计字节数)` 的工作。

## Codex Desktop 对照结论

本计划参考的是 Codex Desktop，而不是 Ratatui TUI。桌面端前台是 Chromium/React，Rust `codex app-server` 负责线程、turn、item、持久化和协议，两者通过增量状态通信协作。

从安装包前端与 `/Users/Shared/work/dir/data/codes/codex-main/codex-rs` 后端可迁移的关键模式是：

- 历史按游标分页，首屏只取最近一页；旧页在接近顶部时加载。
- 已提交历史与活动 turn 分离，活动内容作为 overlay 合并，避免反复复制整个历史。
- 文本 delta 进入帧级队列；前台按约 60 Hz 释放小批内容，积压时自适应放大，终态先排空再完成。
- 状态按稳定 id 与 revision 局部更新；未变化对象保持身份，渲染层通过 memo/cache 跳过工作。
- 虚拟化按 turn 行工作，单行高度变化走快路径，只有结构变化才重建索引。
- Rust 后端使用 `Thread/Turn/Item`、游标页、`itemsView` 和活动 turn 快照，不把完整线程作为每次增量的传输单位。
- JSON-RPC 通道有界，历史落 JSONL/SQLite 元数据，前台与存储都对工作量设置边界。

VibeWindow 不需要照搬 React 或强行引入 turn 模型；应把这些约束映射到现有 `session/message` 领域。

关键后端证据位于：

- `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/app-server-protocol/src/protocol/common.rs`：`thread/read`、`thread/turns/list`、`thread/items/list` 与 turn 增量通知。
- `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/app-server/src/thread_state.rs`：分页初始 turn、resume cursor store 与 `active_turn_snapshot`。
- `/Users/Shared/work/dir/data/codes/codex-main/codex-rs/app-server-protocol/src/protocol/thread_history_projection_tests.rs`：turn/item 事件到历史投影的局部变更语义。

桌面前端源码不在 `codex-rs`；本机 `/Applications/ChatGPT.app/Contents/Resources/app.asar` 的编译产物用于确认帧级 delta、局部 patch 和 turn 行虚拟化，Rust 仓库用于确认协议、分页与活动 turn 后端边界。

## 方案权衡

- 只增大 50 ms/1 s 合并间隔：改动小，但每次仍做全量工作，还会增加首字和终态延迟；不采用。
- 继续扩大完整消息缓存：能缓解完成态重复解析，但流式正文每次变化仍会使全文 key 失效，也不能解决全量历史 I/O；不采用。
- 建立分页历史、活动 overlay、帧级 delta、revision 局部失效和增量持久化：改动面较大，但直接把热路径从全量复杂度降为页/单条/单帧有界工作；采用。

## 目标架构

```text
SQLite / PostgreSQL session_messages(seq_no)
  ├─ summary：元数据与消息计数，不携带完整正文
  ├─ page(cursor, limit)：按游标读取固定大小消息页
  └─ delta：按 seq_no 原子 upsert / truncate，不删除并重写全表
                │
                ▼
Gateway 有界协议
  ├─ 首屏只返回最近页
  ├─ 旧页按需加载
  └─ 流式保存只提交活动消息变化
                │
                ▼
Desktop PagedChatTimeline
  ├─ 稳定 ChatEntryKey + revision + generation
  ├─ 已加载历史页
  ├─ 活动回复 overlay
  └─ 顶部翻页与锚点恢复
                │
                ▼
Frame Delta Scheduler
  ├─ 前台约 16 ms，后台 100 ms
  ├─ 首 delta 立即可见，目标 24 字符/帧
  ├─ 积压感知，最多按 8 帧额度追赶
  └─ terminal 先排空再完成
                │
                ▼
Iced 局部渲染
  ├─ stable-prefix / mutable-tail 增量投影
  ├─ key + revision 驱动缓存
  ├─ 单行高度 point update
  └─ 仅保留可视区、保护页和活动消息的重型缓存
```

## 设计决策

### 1. 正常路径禁止完整历史

完整 session 读取与保存只保留给兼容、导入、显式重置等非热路径。正常打开、滚动和流式保存必须分别使用 summary、page 与 delta。

### 2. 使用现有 `seq_no` 作为存储游标基础

`session_messages` 已以 `(platform_id, company_id, project_id, session_id, seq_no)` 为主键，适合稳定分页和局部 upsert。游标对外保持不透明，并绑定会话作用域和排序方向。

### 3. 活动消息与历史页分离

历史页一旦加载即保持稳定；活动 assistant 消息使用 overlay，并在完成后原子并入规范消息序列，避免每批 delta 重建全部历史。

### 4. 稳定身份贯穿异步边界

复制、fork、reset、上下文动作、测量和后台预热都以 `ChatEntryKey` 定位，并携带 session generation 与 entry revision。索引只作当次渲染位置，不能作为异步结果身份。

### 5. 帧级调度和 Iced 增量投影分层

Codex Desktop 的帧队列负责交互节奏；VibeWindow 的 stable-prefix / mutable-tail 是适配 Iced Markdown 渲染器的实现选择，不声称是 Codex Desktop 的原样实现。

### 6. 性能验收优先断言操作次数

CI 不使用易受机器影响的绝对毫秒阈值。测试记录加载条数、传输字节、完整消息克隆、全文哈希/解析、高度全量重建、SQL 删除/写入、挂载行数和重型缓存条数；人工基准再报告同机前后耗时和帧表现。

## 任务批次

### 批次 1：建立证据

- [任务 1：建立端到端性能基线](./1-建立端到端性能基线.md)

### 批次 2：并行建设底座

- [任务 2：建立消息增量存储原语](./2-建立消息增量存储原语.md)（依赖任务 1）
- [任务 3：增量维护虚拟高度索引](./3-增量维护虚拟高度索引.md)（依赖任务 1）
- [任务 4：建立帧级 Delta 调度器](./4-建立帧级Delta调度器.md)（依赖任务 1）
- [任务 5：建立分页时间线状态](./5-建立分页时间线状态.md)（依赖任务 1）

### 批次 3：建立有界协议

- [任务 6：暴露游标分页与增量提交协议](./6-暴露游标分页与增量提交协议.md)（依赖任务 2）

### 批次 4：切换首屏加载模型

- [任务 7：接入有界首屏与活动尾](./7-接入有界首屏与活动尾.md)（依赖任务 5、6）

### 批次 5：并行完成历史翻页与增量保存

- [任务 8：接入向上翻页与稳定目标](./8-接入向上翻页与稳定目标.md)（依赖任务 3、5、7）
- [任务 9：接入增量流式持久化](./9-接入增量流式持久化.md)（依赖任务 2、6、7）

### 批次 6：移除渲染全量失效

- [任务 10：使用 Revision 驱动局部渲染](./10-使用Revision驱动局部渲染.md)（依赖任务 8）

### 批次 7：接入帧级增量渲染

- [任务 11：接入增量流式渲染](./11-接入增量流式渲染.md)（依赖任务 4、10）

### 批次 8：收紧内存边界

- [任务 12：限定重缓存与预热范围](./12-限定重缓存与预热范围.md)（依赖任务 8、9、11）

### 批次 9：真实性能验收

- [任务 13：完成长会话真实性能验收](./13-完成长会话真实性能验收.md)（依赖任务 1 至 12）

## 验收标准

- 10,000 条历史消息的正常打开只读取和传输首屏页，默认不超过 64 条；达到顶部才加载旧页。
- 向前合并页面后，视觉锚点和复制、fork、reset、上下文操作仍指向原消息。
- 512 KiB assistant 回复经 1,000 次 delta 到达时，流式热路径不再完整克隆 `ChatMessage`、全文哈希/解析或重建完整高度索引。
- 正常流式保存只 upsert 改变的消息与元数据，不执行该会话消息全表 `DELETE + INSERT`。
- 前台帧调度保证首 delta 及时显示、积压可追赶、terminal 事件在内容排空后提交；后台会话更新降频。
- 未变化消息的对象身份、revision、渲染缓存和高度保持稳定；单消息变化只更新对应缓存与高度节点。
- 重型 Markdown、文本编辑器和工具块缓存规模受可视区与保护页上限约束，活动消息不会被淘汰。
- Markdown、代码围栏、`<think>`、工具生命周期、中文、暗黑/明亮主题和完成态结果保持正确。
- SQLite 与 PostgreSQL 的分页、增量写入和截断语义一致。
- 聚焦测试以及 `cargo clippy --all-targets --all-features -- -D warnings` 通过；若失败仅允许记录可证明与本计划无关的既有问题。

## 范围外

- 不更换 Iced，不迁移到 React/Chromium，不引入重型 UI 依赖。
- 不重写 Agent 推理、工具执行或权限模型。
- 不在没有真实调用者前提下增加通用事件总线、插件点或推测性配置。
- 不以简单增大 debounce、关闭 Markdown、减少历史功能或牺牲正确性掩盖性能问题。

## 回滚策略

每个任务独立提交。协议新增先保留旧完整端点，Desktop 切换后再以调用测试保证正常路径不回退；若某一集成任务回滚，底层分页、delta、调度器和测试仍可单独保留，不要求数据库破坏性迁移。
