# 任务4阶段证据（尚未完成）

当前仅完成输入面板外壳与文件引用编辑入口下沉，不满足整项验收，不删除任务卡。

## 已接通生产调用链

- 宿主 input_panel::view → desktop_chat_input::input_panel::layout::surface：覆盖层、拖放组件、主题容器、背景图片、上方子视图顺序、聊天阅读列归领域；宿主投影主题/拖放状态并明确映射文件搜索关闭、斜杠关闭、拖放和原生路径消息。
- 宿主 handle_remove_file_reference → input_panel::file_references::remove_file_reference：完整提及匹配、反向删除、保留路径内部空白、光标移到文尾归领域；宿主保留当前会话选择、全局草稿同步和悬停重置。
- 依赖均为已有 desktop-editor、desktop-ui-controls、desktop-theme、desktop-chat-view，无新增依赖或反向引用宿主。

## 原断言映射

原 message/chat/input/file_search_tests.rs 的 remove_file_reference_deletes_mention_and_compacts_whitespace、remove_file_reference_removes_all_matching_mentions、removing_chinese_file_keeps_other_quoted_path_whitespace 三个文本结果断言，分别同名迁入 host_module_4_tests。第一个用例的悬停断言保留为宿主 remove_file_reference_clears_host_hover。没有删除断言或复制大型 App 夹具。

## 未完成范围

- 输入编辑、搜索、斜杠与菜单完整局部 Update/状态归属仍需迁移；shared 中聊天定位保留代码尚需逐函数审计。
- 整体 view 的移动控件栏及局部策略仍在宿主；其余子模块当前多为状态投影/消息映射，仍需逐项确认。
- agent_sessions、ACP、model_popover、quick_messages、reasoning、send_controls、todo、voice 及整体输入面板 App 测试夹具尚未整组迁入领域。
- 尚未完成所有来源逐函数、字段组和断言清单；不能把本次阶段减量视为任务完成。

## 静态统计

baseline.json task=4：8726 行，其中生产候选 3105 行；当前原来源 8662 行，其中生产候选 3060 行；相对基线减少 64 行，生产候选减少 45 行。此值包含本任务开始前已有迁移，不全部归因本次。本次没有新建宿主文件。

## 验证

仅对本次文件进行差异/注册/依赖方向源码核对与 git diff --check。按用户禁测要求未运行 Cargo、编译、测试或绘制，不能声称行为通过。
以后授权命令：`python3 scripts/cargo_slot.py cargo test -p desktop-chat-input --lib host_module_4_tests -- --test-threads=1`。
