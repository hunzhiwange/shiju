# 新增价值需求与取舍

本文件补充上一计划的38项输入能力，重点是“处理成果与管理会话”。不是把粘贴任务进一步切碎。

证据口径：2026-11-16读取当前识矩与本地 `/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src`。识矩下列路径相对 `crates/libs/`。上游无可读Git版本。以下为静态可行性审查，未运行新增行为测试，不声称需求已实现；候选项明确不作为本轮实现承诺。

## 本轮新增的六项独立用户能力

| 需求 | 对用户的价值 | 当前实际依据与缺口 | 上游参考 | 可行性与责任 |
| --- | --- | --- | --- | --- |
| 全文搜索与命中跳转 | 长对话快速找报错、方案和代码 | `tui-pager/src/lib.rs` 只有滚动/跳页按键，没有查询状态与搜索入口 | `transcript_view/search.rs`有字面搜索、分帧预算与取消恢复 | 可在现有pager实现，任务1；先明确只搜索已加载内容 |
| 复制最近回答或代码块 | 无鼠标/SSH场景快速提取成果 | 当前ChatWidget鼠标选区→clipboard已通；`cli-interaction/src/commands.rs`无copy命令，不能将“有复制后端”等同“有语义复制” | `slash_command.rs` Copy命令和回答/部分内容语义 | 当前消息视图与剪贴板可复用，任务2；必须保留原文格式 |
| Markdown导出当前会话 | 保存、分享、归档分析过程 | 命令表无export；当前应用有session快照，但是否全量必须说明，不能从屏幕Buffer拼接冒充导出 | `slash_command.rs` Export命令 | 当前可做已加载会话导出，任务3；不假造全历史API |
| 按内容选择消息后分叉/回退/压缩 | 不用辨认或复制消息ID即可操作 | `app/sessions.rs::open_session_message_picker` label/search均只用message_id，description固定；fork/reset/compact和ID校验已存在 | 上游transcript交互与消息选择；识矩`cli-interaction-model/src/message_action.rs`也有意图模型 | 复用原操作，先改选择体验，任务4；绝不省略确认与归属校验 |
| 非阻塞会话恢复列表 | Gateway慢时仍能输入、取消和看到错误 | `app/sessions.rs::open_session_picker`直接同步调用session_resume_candidates_blocking；现有worker/HostMessage模式可复用 | `resume_picker/catalog.rs`对陈旧分页、错误保留游标有成熟处理 | 现有API可先异步加载，不要求后端分页，任务5 |
| 原生终端选择/复制模式 | 长代码和日志能用终端原生选择，避免鼠标捕获与刷新干扰 | 当前有应用内选区、mouse capture、动态渲染；命令表无raw。不能把“关闭鼠标”当作完整记录呈现 | `slash_command.rs` Raw、`transcript_mode.rs` | 有条件可行，任务6；状态恢复和实机验证不可省略 |

## 更多值得保留的候选，先明确边界

这些不是已确认缺陷，不能仅因上游有同名文件就承诺全部接入。

| 候选 | 价值 | 现有证据与下轮需要查清的事 | 处置 |
| --- | --- | --- | --- |
| 搜索退出后回到原阅读位置 | 查找后不丢阅读上下文 | 上游 `transcript_view/bookmark.rs`用逻辑身份与源偏移恢复；识矩pager当前以行偏移保存位置 | 纳入任务1搜索取消验收，不另造重复任务 |
| 历史位置稳定与“有新内容”提示 | 看旧内容时流式输出不抢屏，还能知道有更新 | 当前pager已有follow_bottom；上游bookmark有unseen_activity。仍需跨resize/追加验证 | 任务1/7记录现有覆盖；新的新内容提示待独立需求确认 |
| 键盘选择单条消息和代码段 | 无鼠标完整完成复制、分叉等动作 | 识矩有MessageActionIntent，但必须核查真实UI调用者；上游有select_mode/copy_mode | 本轮先落实任务2/4的实际需求，不宣称完整选择模式已实现 |
| 超长工具输出折叠/展开 | 不让命令日志淹没回答，仍能查看原始结果 | 上游exec_cell存在compact/live_output/transcript分层；识矩tui-cells也有exec_cell，尚未审完完整交互链 | 保留候选，先审“原文是否完整保存”和展开入口，禁止只加视觉按钮 |
| Diff逐文件导航与跳转源码 | 大改动中快速定位受影响文件 | 当前/diff已存在且runner_tests已有显示测试；上游transcript有diff_source | 不重做diff请求；后续查文件身份、行号和打开位置语义 |
| 上下文压缩前后预览 | 用户理解压缩影响，避免盲选message_id | 当前compact已有真实Gateway操作及确认 | 任务4先改善起点选择；预览依赖后端返回内容，未查证不伪造 |
| 会话列表分页、项目过滤和跨来源搜索 | 大量会话下快速找到目标 | 上游resume_picker有catalog/sources/page_loading；当前候选API是否支持游标未证实 | 任务5只做现有API异步；分页需后端能力审计后再立项 |
| 文件引用带行范围 | 精确给模型提供相关代码，降低上下文噪声 | 当前@token文件搜索已存在，附件严格限制工作区；行范围协议未核实 | 先查请求模型与读取能力，不把路径字符串格式当作后端支持 |
| 屏幕阅读器与减少动画 | 改善无障碍、降低闪烁和干扰 | 上游screen_reader.rs有有界探测与MotionMode；依赖其ConfigLayerStack | 有价值但需识矩自己的设置/平台边界，不照搬上游配置探测与写入 |
| 中断后的重试/继续 | 避免重新输入并保持任务上下文 | 当前interrupt/queue已有接线；自动重试是否幂等、Gateway是否支持续传未核实 | 先审消息幂等/重复工具执行风险，不能只加Retry按钮 |
| IDE选区/打开文件上下文 | 降低手工复制代码成本 | 恢复的ide_context与上游相关命令是参考；当前IDE来源身份/传输协议未证实 | 保留源码，另立跨进程协议需求，不归本轮终端UI直接搬运 |
| 终端图片预览 | 发送前确认图片内容而非仅文件名 | 当前图片附件可发送，上游有image_protocol/sixel等能力；识矩跨终端支持未证实 | 先审终端能力探测与资源限制，文本退化必须可靠，不因有图像库就宣称支持 |

## 优先顺序

优先复制回答、按内容选择消息、非阻塞恢复和全文搜索：直接使用现有数据与业务能力，收益明确。Markdown导出其次；原生选择模式需要更强终端恢复与实机证据。此优先级不改变任务文件声明的真实写入依赖。

不重复立项已经存在的fork/reset/compact、/diff、文件搜索、任务队列、OSC52后端和会话恢复业务；新增价值主要来自更完整的用户入口、可靠的响应与内容处理。

## 验收要求

任务7必须逐项填写：实现提交、真实入口、有效单测数、错误/取消结果、数据来源与平台限制。候选审查不等于功能实现；未加载历史不能默认为已搜索/导出；渲染输出不能代替语义原文；界面选择不能绕过Gateway权限与消息归属。
