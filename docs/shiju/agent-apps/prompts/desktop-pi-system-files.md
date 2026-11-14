# 桌面 Pi 系统文件（任务 32）

消费任务 28 的 PiPromptResourcesScope、PiPromptFileKind、pi_prompt_file_read/replace/delete 与任务 29 的应用提示词入口。来源为固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 PiNativePromptResources.tsx / PiSystemPromptFiles；本地来源位于 src/components/prompts/PiNativePromptResources.tsx。

Pi 应用详情提供系统文件弹窗，固定 APPEND_SYSTEM.md（追加）及 SYSTEM.md（覆盖）两个页签，分别保存存在状态、基线 revision 和草稿。复用统一设置编辑器、按钮、弹窗布局及内容区 4px 滚动条、两侧 7px 留白；使用默认字体承载 Unicode。此处仅管理外部 Pi 文件，不修改或拼接识矩运行时系统提示词。

首次读取初始化草稿。重新读取只呈现主机当前内容，不覆盖已有草稿；用户比较后显式接受新 revision 才能继续写入。保存和删除始终提交基线 revision。409 与其他写入失败保留草稿并阻止直接重试，需重新读取、比较并确认；不自动重放。创建 SYSTEM.md 与删除分别需要二次确认。删除成功显示不存在并保留本地草稿，可显式再次创建。空白及超过 1 MiB 的保存直接拒绝，后端继续执行授权和文件安全检查。

响应核对 gateway/tenant/project 上下文、应用目录 generation、编辑器 generation、项目和执行主机、资源索引及 pending 状态，避免旧请求污染当前编辑器。请求期间禁止关闭、切换页签和编辑。错误通过既有 toast 输出，HTTP 正文不用于提示。

按本轮用户要求，未执行任何 Cargo 操作、单元测试、实机测试或其它测试，未领取槽位、未启动最终回归；不声明验证通过。后续统一验证应覆盖两个独立草稿、读改删、409 保留草稿、重新读取及显式接受 revision、网络失败、乱序响应、项目/主机切换、覆盖创建确认、删除确认、Unicode 与明暗主题的 macOS 实际渲染。
