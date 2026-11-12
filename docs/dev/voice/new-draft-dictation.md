# 新会话首条消息前的本机听写

## 问题与修复边界

新会话键盘输入使用 `current_session_runtime_mut()` 的本地 `__empty__` 编辑器。旧听写 `voice::scope` 先要求 `active_session_id`，使尚未发送消息的本机听写直接报“请先选择聊天会话”。本次只修正听写及共用草稿归属边界；不提前创建网关会话、不自动发送、不更改语言或其它偏好。

复用已有 `session_view_cache.navigation_generation` 区分不同本地草稿，用强类型 composer 身份区分本地草稿与正式会话。新草稿到正式会话或导航变化使在途听写失效；已完成回填的文字由原 `SendPressed` 首条消息路径正常消费，沿用原模型、历史和审批。

## 实施与验证顺序

1. 新增真实 `App::update` 测试，经键盘编辑消息准备新草稿，再点击听写；先观察旧代码在 Recording 断言失败。
2. 本机采集及识别元数据允许没有远端会话；Cloud 明确拒绝未建立会话，不使用本地草稿标记作为远端ID。
3. 独立测试覆盖中文原生光标、已有草稿、只回填一次、不发送、取消/编辑、另开草稿/切换项目或会话、首次发送身份转换和迟到回包。
4. 冻结源码后运行格式化、聚焦测试及全仓 `cargo clippy --all-targets --all-features -- -D warnings`，记录实际槽位与结果。

## 稳定内部接口

- `voice_draft::Scope.composer: Composer::{Session(String), Draft(u64)}`；Draft值为现有导航代次，仅用于本地归属比较，绝不序列化为远端会话ID。
- `Scope::session_id() -> Option<&str>`；`dictation::{select_route,prepare,retry}` 的会话参数为 `Option<String>`，Local接受None，Cloud在任何网关查询前拒绝None。
- 不新增持久化字段；`engine/local_language/local_voice`及用户已保存的zh-CN等偏好不改写。

## 相邻入口

本机逐轮语音聊天当前仍保存正式会话字符串，使用 `SendToSession` 加载已有历史并以该会话拥有模型请求及停止权。它也有同类新会话限制，但不能仅将本地草稿标记传给该协议。支持它还需处理首次项目解析/正式会话建立与逐轮状态的身份迁移；本次优先修复用户实际使用的听写，不将其宣称为新会话逐轮聊天支持。

## 验证记录

- 有效红测：`cargo test -p vw-desktop --lib voice_task_07_new_draft_ -- --nocapture` 退出101，1项在真实App::update的Recording断言失败（实际Error），不是缺模块/编译失败；日志`/tmp/voice07-new-draft-red.log`。
- 同步一处任务19取消夹具的Scope字段；第一次绿测编译因此退出101、未运行测试，未作为行为证据。
- 最终 `cargo fmt --all` 退出0；没有保留无关纯格式变化。冻结源码后 `cargo test -p vw-desktop --lib voice_task_ -- --nocapture --test-threads=1` 退出0，144通过、0失败/忽略、9398过滤，运行6.12s，含本次10项；日志`/tmp/voice07-new-draft-final-green.log`。新增App测试的正常发送及项目/会话导航均经过真实App消息路由，只有物理录音/OS识别完成事件被替代；新增测试的模型与网关任务不轮询，不发送实际请求。
- 实际动态领取共享槽5，两个Cargo目录同为仓库根目录下的`target/rust-slot-5`，CARGO_BUILD_JOBS=2；结束时释放自己的锁，缓存保留。全仓Clippy结果如下。

- `cargo clippy --all-targets --all-features -- -D warnings` 自然退出0，6m48s，日志`/tmp/voice07-new-draft-clippy.log`。冻结后的10份Rust源码摘要在绿测前置门禁和提交前均一致；未加入lint豁免。

## 新App交接

任务19在本次修复提交之上构建新的独立开发.app并带到前台，保留用户原有本机偏好，直接在新会话点击听写重试。旧包`dist/local-voice-20261112-ea994e/ShiJu.app`不包含此修复，本轮未操作、覆盖或重新启动它。无需先发送文字或选择旧会话；本次代码验证不冒充新包已完成本人重试。
