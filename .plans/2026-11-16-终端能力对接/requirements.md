# 终端能力逐项核查与需求矩阵

## 证据口径

核查日期：2026-11-16。当前仓库 `/Users/xiangmin/worker/code/shiju`；下表 `tui-*`、`cli-*` 相对 `crates/libs/`。上游只读根为 `/Users/xiangmin/worker/code/codex-main/codex-rs/tui/src`，该目录无可读取 Git 元数据，不推测版本。

已完成的实际验证：恢复的258个文件逐字节等于 `40de872d6^` 对应文件；提交 `44f78e22d`。保留原有有效测试修复，快照和许可未动。恢复源码没有被加入编译图，因此本轮不通过运行Cargo宣称恢复能力可用。下列为源码调用链审计，具名测试仅证明已有测试资产，除明确列出的先前终端18项证据外，不声称本轮执行通过。

三种结论必须分开：有参考代码、有实际入口、有当前行为/实机通过证据。不得只凭函数名、同名crate、模块不可达、或一次编译成功判断能力价值与支持状态。

## 优先级与范围

P0：任务1/2防误提交，任务3资源/快捷键边界，任务7终端恢复。P1：任务4/5/6/8富输入与历史。P2：任务9显式Vim、任务10已实现复制核验、任务11终端提醒/标题设计。任务12负责跨任务验收；优先级不替代真实依赖批次。

本轮选择逐能力适配现有库，保留旧源码作为参考。整套激活旧TUI会重新引入Codex依赖；照搬最新上游会更换运行协议。上游独有业务能力不以界面代码存在为对接承诺。

| ID | 用户能力 | 当前判断 | 静态依据 | 接入或验收归属 |
| --- | --- | --- | --- | --- |
| 01 | 括号多行粘贴 | 已有静态接线 | cli-interaction/src/app/terminal.rs → Event::Paste → ChatWidget::handle_paste → ChatComposer::handle_paste | 任务2/12：真实事件保留换行、不发送，手动Enter才提交 |
| 02 | 非括号快速键流粘贴 | 确认缺少组合器接线 | tui-input/src/paste_burst.rs 已有状态机；tui-composer/src/chat_composer.rs 文档明确未接入 | 任务1：复用底层，不重写检测器 |
| 03 | 粘贴结束无人继续输入 | 缺少刷新链 | cli-interaction/src/app/terminal.rs 当前50ms轮询，没有组合器到期刷新 | 任务2：按截止时间刷新，验证单字符不悬挂 |
| 04 | 粘贴内Enter防误提交 | 确认当前风险路径 | ChatComposer::handle_key_event 普通Enter直接 submit_or_queue | 任务1/2：键流多行不可触发提交 |
| 05 | 中文IME/emoji快速输入 | 有底层算法，组合器待适配 | PasteBurst::on_plain_char_no_hold 与 TextArea UTF-8编辑 | 任务1：非ASCII无首字悬挂，修饰键不吞字符 |
| 06 | AltGr与快捷键边界 | 需行为验证 | tui-clipboard/src/clipboard_image.rs is_image_paste_key 用 intersects Ctrl/Alt | 任务3：Ctrl+Alt文字与真正粘贴键分流，不误读剪贴板 |
| 07 | 长文本折叠展示 | 当前组合器直接插入全文 | ChatComposer::handle_paste 直接 textarea.insert_str；上游 LARGE_PASTE_CHAR_THRESHOLD=1000 | 任务4：保序还原原文，不能发送占位符 |
| 08 | 代码粘贴首尾空白 | 当前提交会trim，需区分普通输入与原文 | ChatComposer::submit_or_queue text.trim；PastedText通道可承载原文 | 任务4：证明空白和插入位置，不随意更改普通提交契约 |
| 09 | 占位符删除与撤销 | 已有可复用原子元素 | tui-input/src/textarea.rs insert_element/add_element_range/remove_element_range | 任务4：删除、撤销、编辑后全文准确 |
| 10 | 系统图片粘贴 | 已有真实静态接线 | ChatWidget::handle_key_event → read_clipboard_image_attachment → add_attachment | 任务3/12：验证实际附件结构，不重复开发图片读取 |
| 11 | 图片发送到Gateway | 已有具名测试，未在本轮执行 | cli-interaction/src/attachment_tests.rs clipboard_image_reaches_gateway_without_a_workspace_file | 任务12：复核PNG字节、MIME、无工作区文件路径 |
| 12 | 图片尺寸与内存上限 | 部分校验存在 | clipboard_image::encode_rgba_as_png 有checked_mul和RGBA长度；attachment.rs有5MiB检查，编码前缺明确资源上界 | 任务3：零尺寸、溢出、超大原始尺寸有界拒绝 |
| 13 | 剪贴板无图片或不可用 | 已有错误展示路径 | ChatWidget读取失败推 Error 事件，不清空组合器 | 任务3：草稿/附件不变，避免repeat反复读取 |
| 14 | WSL图片读取回退 | 上游有，本地未找到回退 | 上游 clipboard_paste.rs try_wsl_clipboard_fallback；本地只用arboard | 任务3：列为需实机验证，不能因文本复制支持WSL就声称图片可用；新增外部命令另列依据 |
| 15 | 拖放/粘贴图片文件路径 | 上游有，本地组合器未分类 | 上游 normalize_pasted_path/handle_paste_image_path；本地handle_paste只插文本 | 任务6：明确单文件候选，普通文本保留 |
| 16 | 路径引号、空格与file URI | 上游已有解析参考 | 上游 clipboard_paste.rs normalize_pasted_path tests；本地工作区附件校验已有 | 任务6：路径规范化与权限校验分离，不用shell执行 |
| 17 | 工作区越界、符号链接与大小 | 已有实际请求边界 | cli-interaction-model/src/attachment.rs canonicalize/starts_with/root/metadata/MAX_ATTACHMENT_BYTES | 任务6/12：越界/错误不发送，恢复原草稿 |
| 18 | 历史上下导航 | 已有组合器接线 | ChatComposer::restore_history_previous/next 在光标边界触发 | 任务5：保留未提交草稿与Unicode |
| 19 | 历史中的图片和长粘贴 | 确认富状态未保存/恢复 | submit_or_queue 使用HistoryEntry::new(text)，restore_history只设置entry.text | 任务5：真实附件、原子元素完整往返，别只证明HistoryEntry有字段 |
| 20 | Ctrl-R反向搜索 | 底层有，组合器未见入口 | ChatComposerHistory::search存在；当前handle_key_event无Ctrl-R分支 | 任务8：接受/取消/无匹配，接受搜索不发送 |
| 21 | 持久会话历史与本地输入历史 | 职责分离，不能混同 | chat_composer_history模块文档明确本地；cli-interaction/src/app/sessions.rs归Gateway | 任务5/8/12：本轮不新建历史存储，不挪用旧Codex session接口 |
| 22 | 任务运行中排队输入 | 已有实际接线与测试 | ChatComposer Tab→Queue；tui-chat/chatwidget/input_queue_tests.rs | 任务5/12：FIFO、图片/长粘贴随队列只发送一次 |
| 23 | 重编最后排队消息 | 已有具名测试 | editing_last_queued_message_replaces_it_instead_of_duplicating_it；shift_left_edits_the_last_queued_message_and_keeps_fifo_order | 任务5/12：保留现有键位，验证附件完整，不重复开发 |
| 24 | 失败提交恢复 | 已有具名测试 | cli-interaction/src/attachment_tests.rs attachment_failure_restores_the_composer_draft | 任务5/12：增加富内容组合边界，区分校验失败和网络失败 |
| 25 | 外部编辑器启动与回填 | 已有真实实现 | cli-interaction/src/app/runner.rs Editor::new().edit；runner_panels::launch_editor；runner_host EditorFinished | 任务7：复用edit库，验证取消/失败/附件与草稿 |
| 26 | 编辑器切换终端状态 | 存在直接raw切换，完整状态待证 | cli-interaction/src/app/terminal.rs disable_raw_mode/enable_raw_mode；生命周期同时管理paste/mouse/screen | 任务7：两种模式恢复，包含初始化/返回错误，不照抄Codex进程策略 |
| 27 | Vim编辑模式 | 底层存在，入口默认关闭 | tui-input/TextArea set_vim_enabled为pub(crate)且默认false；app/settings.rs vim_mode Disabled | 任务9：复用既有SettingKey::Vim，真实设置改变编辑行为 |
| 28 | 自定义键位设置 | 底层RuntimeKeymap存在，产品入口未证实 | tui-input/src/keymap.rs；恢复的keymap_setup依赖上游配置语义 | 任务9/12核对现有能力矩阵；本轮仅接Vim与冲突所需接口，不迁Codex配置体系 |
| 29 | 文件搜索@token | 已有运行器与陈旧结果测试 | runner_tests file_search_discards_stale_results_and_applies_latest_query；ChatComposer replace_file_search_token | 任务6/12：粘贴内容不能误触异步搜索或覆盖后缀 |
| 30 | 全文pager与终端历史 | 已有当前入口 | cli-interaction/src/app/terminal.rs pager分流；tui-terminal插入历史；vw-tui现有vt100测试 | 任务2/7/12：粘贴不落到背景草稿，resize恢复不损历史 |
| 31 | SSH/OSC52复制 | 已有后端与测试 | tui-clipboard/src/clipboard_copy_tests.rs ssh_uses_osc52_and_skips_native_on_success/osc52_encoding_roundtrips | 任务10：复用并验证转义、大小、中文、失败 |
| 32 | tmux/WSL/本机复制 | 已有环境分派与lease | tui-clipboard/src/clipboard_copy.rs及测试 local_wsl_environment/remote_tmux_environment | 任务10：区分测试替身与真实平台证据，不泛化成图片支持 |
| 33 | HTML富复制 | 上游有，当前价值与调用者待确认 | codex-main/tui/src/clipboard_html.rs；当前copy_to_clipboard接受文本 | 任务10：只记录需求边界，无真实调用者不增加重依赖 |
| 34 | 审批/问题覆盖层输入 | 已有独立分流 | ChatWidget::handle_key_event/handle_paste在overlay存在时先转发 | 任务2/3/12：粘贴不提交后台任务、不改权限决策 |
| 35 | 完成/审批等待的桌面提醒 | 恢复代码有，当前纯模型不等于系统通知 | vw-tui/src/notifications；cli-interaction-model/src/notification.rs NotificationStore | 任务11：形成启用、去重、焦点与隐私设计，本轮不默认开启 |
| 36 | 终端窗口标题 | 有可复用净化参考，运行接线未找到 | vw-tui/src/terminal_title.rs sanitize_terminal_title；当前session_title_tests验证会话标题，不等于OSC标题 | 任务11：分清会话标题与窗口标题，ESC/C1净化及退出恢复 |
| 37 | 会话恢复/模型/工具选择 | 已有Gateway领域，不复活旧业务 | cli-interaction/src/app/sessions.rs、settings.rs、integrations.rs 与现有session/integration测试 | 任务12按现有真实入口记录覆盖，缺口单列，不搬Codex auth/session业务 |
| 38 | 宠物/IDE上下文/内联可视化 | 恢复为候选参考，尚无本轮完整可行性证据 | vw-tui/src/pets、ide_context、inline_visualization | 本轮范围外保留源码；后续需独立产品需求、运行协议与安全审查，不再按不可达删掉 |

## 实施者必须补齐的动态证据

- 每一项记录真实输入、当前错误或既有正确结果、预期用户行为、真实所有者与调用链；新缺口先红后绿。库内测试之外，输入路径至少验证到真实动作/请求组装。
- 粘贴测试覆盖不同速度、定时flush、CRLF、中文/emoji、普通按键、AltGr、Enter、Ctrl-C/Esc、overlay、pager、任务运行/空闲；不靠真实sleep制造不稳定测试。
- 富内容测试覆盖多次粘贴插入位置、历史/队列/撤销/外部编辑器往返、文件校验/网络失败恢复，以及绝不发送展示占位符。
- 平台证据分别记录 macOS/Linux/Windows/WSL、SSH/tmux、inline/alternate；未测只列待验证，不贴“全平台支持”。OS剪贴板和外部编辑器实机项允许后续安排，不能影响已通过纯行为项的准确结论。
- 对通知/标题需先明确真实用户启用入口、去重/焦点策略、敏感数据与终端控制序列处理；本计划仅完成设计审查，不默认修改用户环境。
- 新需求超出已列范围时先记录边界和证据，不重新批量删除参考代码。未来删除必须先完成能力映射、明确不采用原因、调用与资源审计，再单独安排审查。
