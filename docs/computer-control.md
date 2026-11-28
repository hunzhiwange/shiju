# 电脑操控（macOS）

电脑操控通过 ShiJu Computer Use 执行应用观察、截图与输入。实际操作前请确认目标应用、网关状态和系统权限。

## 安装与授权

原生执行应用随 macOS 安装包内置，位于 `ShiJu.app/Contents/Helpers/ShiJu Computer Use.app`。本机网关提示执行应用未安装时，打开「设置 → 电脑操控 → 任意应用 → 安装与权限」，复制安装命令到终端运行，再回到设置刷新状态。若 ShiJu 已放在 `/Applications`，也可以运行下方命令。

```sh
"/Applications/ShiJu.app/Contents/MacOS/shi-ju" --install-computer-use
```

安装后，执行应用保存在 `~/.shiju-harness/Helpers/ShiJu Computer Use.app`，以独立应用身份启动。在「任意应用」中分别打开辅助功能和屏幕录制，将授权窗口里的 ShiJu Computer Use 图标拖入对应列表并开启。返回 ShiJu 刷新状态。

远程网关的报告和系统权限对应运行网关的 Mac。请在该 Mac 上安装执行应用并授予权限。

## 应用授权与停止

「任意应用」的开关控制原生应用的使用策略。应用授权按 Bundle ID 展示，可在「始终允许的应用」或「任意应用 → 管理权限」中撤销。等待服务端确认后，以刷新后的授权列表为准。

需要中止时，点击「停止所有电脑操控」并等待服务端确认。停止会中止会话，已经提交给应用的修改需要在应用内处理。

锁屏使用由独立策略和后端状态共同决定。当前设置页拒绝开启后端未报告支持的锁屏能力。

浏览器操控可在「设置 → MCP → 一键安装」中安装 OpenCode Browse MCP。再使用卡片上的入口安装浏览器扩展，按「安装详情」配对。
