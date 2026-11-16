# 任务5实施证据（尚未完成）

已迁移 Markdown 正文编排与选择感知表格绘制至 desktop-chat-view::message_view::markdown_body。MarkdownView 仅借用编辑器、会话标识、选区状态，并接收链接与代码卡片的局部回调；不引用宿主 App。宿主 markdown_body_view 构造快照并直接调用新入口，保持原消息映射。

调用链：assistant_body → 宿主 markdown_body_view → 领域 markdown_body_view → markdown_block_view / table_view → 原有领域选择组件、markdown 装饰、表格布局。宿主链接解析和代码复制装配暂保留。

当前基线来源总量 3050 行，现存 2970 行，净减 80；生产候选基线 1536 行，现存 1456 行，净减 80。splitlines 口径；未新增宿主文件。

未完成：消息气泡主入口、assistant_body、markdown_links 的领域逻辑与所有相邻测试仍需迁移；尚无 host_module_5_tests 注册及完整逐断言映射。旧 markdown_rich 表格绘制仍被宿主测试引用，需随测试迁移去除。任务卡保留，不声明完整静态交付。

静态证据：独立 rustfmt 解析已修改文件；本任务文件 git diff --check 无告警。全工作区 diff --check 的其它任务文件 system_settings_agents/mod.rs 和 voice_chat_tests.rs 存在空行告警，未改动。

遵守用户禁测要求，未执行任何 Cargo、测试或实机绘制；编译、生命周期及 UI 行为均未验证。后续授权后验证命令仍见任务卡。
