# 桌面原生模板编辑（任务 33）

来源：cc-switch 固定快照 ca58c0313360eae7c851fbd7553c1614c7af03e7 的 PiNativePromptResources.tsx / PiPromptTemplates；消费任务 28 resources-sdk.md 与任务 29 desktop-list.md 的实际入口及 SDK。

应用设置 Pi 页增加原生模板弹窗。列表显示 Rust 返回的 description、argument_hint，按 slug 排序；正文编辑保留 frontmatter 与变量，不在客户端解析或展开。新增使用 missing；修改带原始 slug 与 revision；改名冲突保留草稿并显示错误。删除需确认，成功后保留原文供撤销，撤销始终使用 missing 创建，禁止覆盖后来同名文件。删除失败不提供伪成功撤销；刷新失败保留列表，初始读取失败禁止新增。所有响应核对网关上下文、应用目录代数、项目、主机及请求代数，过期响应不更新界面。

命令复制仅调用 Iced 客户端剪贴板，内容为 /slug，不发主机命令。所有 Unicode 使用默认字体，弹窗和表单复用 system_settings_common；滚动区沿用统一布局的 4px 滚动条与内容留白。保存后提示用户在 Pi 执行 /reload。

本轮遵循最新指令，仅实现代码与本合同；未调用 Cargo，未领取槽位，未编写或执行测试，未做 macOS 实测或最终回归，不声明验证通过。后续统一验证需覆盖变量与元数据往返、改名冲突、陈旧 revision、删除撤销遇到同名新文件、错误与乱序响应、剪贴板和 macOS 中文实际渲染。
