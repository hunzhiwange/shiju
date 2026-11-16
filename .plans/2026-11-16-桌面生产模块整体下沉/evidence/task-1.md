# 任务1阶段证据（未完成）

使用 do-task-plans、rustcodegraph 技能。索引存在；本轮并行编辑造成漂移，使用实际源码补齐。严格未运行 Cargo、测试、绘制或回归。

## 已实现

ActiveSessionViewState 从宿主 state/agent.rs 迁入 desktop-session::session::active，宿主使用类型别名。投影类型参数沿已有 desktop-chat-view 实现传入 project_trajectory，避免 desktop-session 反向依赖 desktop-chat-view。步骤替换、upsert、清空、原始载荷增量与替换、消息元信息及用户问题索引十一项生产入口由领域状态执行；App 保留薄委派，真实调用者继续使用原方法。

恢复 full_access、恢复历史模型、标记 ACP 重建三个转换归 runtime.rs；宿主仍负责定位会话及网关任务。

## 断言归属

active_tests 六个完整用例按原名迁入 host_module_1_tests：refresh_tail_message_meta_appends_then_replaces_changed_label、refresh_tail_message_meta_rebuilds_gaps_and_truncates_out_of_range_entries、question_index_updates_incrementally_for_appended_user_message、active_session_step_mutations_keep_steps_calls_and_index_map_consistent、active_session_step_mutations_normalize_blank_turn_ids、active_session_clear_removes_step_raw_payloads。所有旧断言保留，App 初始化换为真实领域状态；聊天消息作为局部输入。领域投影参数用 ()，真实轨迹投影仍在既有宿主契约验证，尚未迁完。没有删除任何行为断言。

## 尚未完成，不能删卡

runtime 的默认初始化、工具设置选取及整组 runtime_tests 尚需领域上下文；active 的缓存切换、目录查找、history boundary 及其测试尚未完成归属。ui_projection 的消息分类与缓存失效尚需收口。chat_ui/loading/view_cache 等来源中现有薄装配与领域测试还需逐项审计，不能将已有委派视作全部验收完成。没有编译或运行证据。

## 行数

baseline 任务来源 3020 行，当前 2854 行，来源净减少 166 行；新增宿主替代文件 0。宿主 state/agent.rs 为必要状态归属接线，另计。生产候选与测试分类仍需最终审计，当前不作完整生产净减量结论。

## 静态验证

本任务路径 git diff --check 已单独执行；全工作区 diff --check 发现其它任务的 config_tests.rs、message_ops.rs、stream_tests.rs 空白问题，不纳入本任务修改。

后续获授权才可运行：python3 scripts/cargo_slot.py cargo test -p desktop-session --lib host_module_1_tests -- --test-threads=1，以及宿主 cargo check -p vw-desktop --lib。本轮均未运行。
