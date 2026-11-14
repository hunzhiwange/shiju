# 任务 45：文件管理器打开

2026-11-14，仅完成代码实现，未验证。

路径归属沿用 workspace-contracts.md 的执行主机边界，现有普通路径消息没有独立目标主机字段，因此仅允许当前直连 loopback 网关的路径；远端连接明确返回错误，不向本机系统或远端 reveal 接口分派。复用 GatewayEndpoint::is_loopback，未新增远程路径协议。

OpenPathInFinder 与项目菜单 Finder 分支统一进入 config_file_manager.rs：本机绝对路径或本机 file URL，经存在性检查后在 macOS 使用 /usr/bin/open -R，等待退出状态并通过既有 PathRevealFinished/toast 反馈错误。原始路径保留百分号，拒绝相对路径、控制字符、网络文件 URL。其它外部编辑器分派保持原入口。

按本轮用户要求未运行 Cargo、单元测试、实机测试和最终回归，未领取槽位，CARGO_TARGET_DIR 不适用。后续统一验证本机文件/目录、中文与百分号路径、远端连接拒绝、无效路径及系统命令失败反馈。任务卡按用户要求删除，不代表验收通过。
