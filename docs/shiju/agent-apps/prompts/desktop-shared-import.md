# 桌面分享提示词确认契约

来源：cc-switch `PromptConfirmation.tsx`、`deeplink/prompt.rs`，对应既有来源快照 ca58c0313360eae7c851fbd7553c1614c7af03e7。

母任务20分发入口：先通过现有 `prompts::Event::Open(target_app)` 加载用户明确选择的项目、主机及外部应用作用域，待 catalog 与 preview 就绪后发送 `prompts::Event::SharedImport(shared_import::Event::Open(PromptSharePayload))`。载荷是已解码领域数据，不在此实现协议注册或解析。入口拒绝未加载、忙碌及不支持目标；系统提示词不属于此应用作用域。调用者负责导航到对应应用提示词视图。

弹窗展示名称、描述、完整正文、来源应用、目标应用、主机与项目。分享元数据的 activationSuggested 不赋予激活权限；导入后激活默认关闭，由用户显式更改。确认调用任务37 SDK `agent_app_prompts_import_share`，携带目录 revision 和目标文件 revision。取消仅清理内存，不调用 SDK。

成功或激活部分失败均关闭弹窗并使旧目录与文件预览失效，要求刷新。传输失败也关闭弹窗并要求先核对目录，避免服务端已持久化时重复提交。忙碌时禁止取消和重复确认，旧上下文/代际回复被丢弃。弹窗使用统一 modal、checkbox 和按钮样式，内容区复用 4px 滚动条与两侧 7px 留白，Unicode 使用默认字体。

按本轮用户要求未运行 Cargo、单元测试、实机测试或最终回归，未领取构建槽位。后续统一验证需覆盖取消零写入、激活选择、作用域变化、重复提交、部分成功及 macOS Unicode 实际渲染。
