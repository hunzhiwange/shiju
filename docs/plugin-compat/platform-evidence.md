# 平台行为证据

2026-11-04 已完成用户授权的 **macOS 验收范围**。用户本次明确跳过 Windows、Linux 和 OAuth；这些项目不计为通过，不宣称三平台或 OAuth 完全兼容。确切命令、退出码、槽位与截图散列见 [t64.json](evidence/t64.json)。

| 行为 | macOS arm64 | Linux | Windows |
| --- | --- | --- | --- |
| 原包清单、中文/空格路径、真实安装发现 | 通过，4 项后端测试之一 | 用户跳过 | 用户跳过 |
| Python stdio 真实工具调用、包外输出、shutdown 请求 | 通过 | 用户跳过 | 用户跳过 |
| 宿主 hook 命令与包外数据 | unix 分支通过 | 用户跳过 | 用户跳过，未执行 commandWindows |
| 无效清单拒绝、来源隔离、包字节不变 | 通过 | 用户跳过 | 用户跳过 |
| OAuth 回调链 | 用户跳过 | 用户跳过 | 用户跳过 |
| MCP Apps 原生页面 | 本次通过：初始化/结果、中文、明暗、计数、滚动、取消、关闭回收 | 用户跳过 | 用户跳过 |
| Iced 插件目录与安装更新弹窗 | 本次通过：中文长路径、明暗、错误重试、取消、安装与更新 | 用户跳过 | 用户跳过 |

## 本次 macOS UI

`plugin_compat_t64_tests` 以独立控制目录运行未改写的 t31 HTML/Python 资源夹具，调用实际 `McpAppPanel` 和本仓库重新构建的 WebView helper。CUA 观察“工具结果已加载”，连续点击计数达到 3，切换明暗主题，滚动至第 22 行，取消后显示“已取消”且计数按钮禁用。随后宿主关闭面板、回收服务进程并验证夹具字节不变。该测试须显式传入 `T64_UI_ROOT`、`T64_UI_HELPER`，再以 `--ignored` 执行；本次为 1 passed、0 ignored，不能以默认忽略代替验收。

原生窗口图示：[浅色初始页](evidence/t64/macos/app-light.png)、[深色与计数](evidence/t64/macos/app-dark-count.png)、[滚动](evidence/t64/macos/app-dark-scroll.png)、[取消](evidence/t64/macos/app-cancel.png)。观察记录与 AX 状态保存在同目录。AX 滚动条数值未更新，滚动证明采用实际截图中的内容变化。

复用现有 `plugin-compat-window` 入口，实际启动 Iced 窗口及独立、鉴权的本地 Gateway：目录案例和安装更新案例均退出 0，各包含 1 个有效后端测试。逐帧截图验证中文长路径确实经过字体整形，明暗主题均可读；错误后重试、取消不发布、确认安装/更新及能力差异走实际应用消息与网关。夹具原文不变、未自动执行脚本。图示：[目录浅色](evidence/t64/macos/iced-55-t55-light.png)、[目录深色](evidence/t64/macos/iced-55-t55-dark.png)、[安装审计](evidence/t64/macos/iced-56-t56-review-light.png)、[更新审计](evidence/t64/macos/iced-56-t56-update-dark.png)。

本次检查：后端任务测试 4 passed；原生页面 1 passed；相邻桌面测试 5 passed（旧手工页面案例 1 ignored，由上述任务 64 页面案例单独验收）；Iced 目录/安装更新两个实际窗口案例通过；clippy 与差异检查退出 0。两阶段均使用槽位 3，`CARGO_TARGET_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3`，`CARGO_BUILD_BUILD_DIR=/Users/xiangmin/worker/code/shiju/target/rust-slot-3/build`，锁均已释放、缓存保留。

这些是既有实现的验收，没有生产行为修改或人为制造的红灯。2026-11-03 的相邻 t32 失败保留在历史日志中，未修改该独立行为。夹具来源、许可、散列与预期登记于 `crates/vw-agent/testdata/plugin_compat/t64/sources.json`。
