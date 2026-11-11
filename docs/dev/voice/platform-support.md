# 桌面语音平台与安装包验收

核验日期：2026-11-12。状态针对 ShiJu 当前实现，不等同于底层音频库的支持列表。

| 能力 | macOS | Windows | Linux |
| --- | --- | --- | --- |
| 原生录音 | 未实测完整安装包音频链路；CPAL CoreAudio | 不支持，当前权限后端拒绝启动录音 | 不支持，当前权限后端拒绝启动录音 |
| 播放和设备枚举 | 未实测完整安装包音频链路 | 未实测；底层 CPAL WASAPI | 未实测；底层 CPAL ALSA |
| 系统权限查询与授权引导 | 已有原生实现；安装包授权弹窗待实测 | 不支持，当前权限后端明确拒绝 | 不支持，当前权限后端明确拒绝 |
| 全局语音热键、跨应用听写插入 | 原生实现与前序聚焦验证已具备；签名应用整链路待回归 | 不支持 | 不支持 |
| 主动前台窗口单帧分享 | macOS 14+，前序任务已实测单窗口、取消和窗口关闭；签名应用整链路待回归 | 不支持 | 不支持 |
| 公开语音服务 | 未实测真实服务，需用户配置网关服务凭据及配额 | 未实测 | 未实测 |

## 构建前提

- 当前固定 CPAL 0.18.2（`default-features = false`）。macOS 使用系统 CoreAudio，需 Xcode Command Line Tools 与 macOS SDK；CPAL 该版本声明 CoreAudio 最低 macOS 14.2。旧系统未经实测，不承诺可用，也不在本任务提高全应用最低系统版本。
- Windows 使用 WASAPI，需对应 Rust 工具链、链接器和 Windows SDK；没有启用 ASIO/JACK，不要求安装其 SDK。CPAL 的平台支持不能证明 ShiJu 权限后端或安装包已可用。
- Linux 仍需 ALSA 开发文件和 pkg-config（Debian/Ubuntu：`libasound2-dev pkg-config`；Fedora：`alsa-lib-devel pkgconf-pkg-config`）。禁用默认 features 不会移除平台必需的 ALSA 依赖。还需可用的音频设备/会话权限及桌面其它原生依赖；未验证 PipeWire/PulseAudio 环境兼容性。
- 服务凭据只配置于网关；客户端包内不含凭据。麦克风、扬声器、显示器和系统授权属于客户端，远程网关成功不能代替本机验收。

依据：[CPAL 0.18.2 README](https://docs.rs/crate/cpal/0.18.2/source/README.md)、[Apple Audio Input entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.device.audio-input)、[Hardened Runtime](https://developer.apple.com/documentation/xcode/configuring-the-hardened-runtime)。

## macOS 声明与签名

`scripts/bundle_macos.sh` 实际修改生成的 `.app/Contents/Info.plist`，新增或覆盖：

> ShiJu uses your microphone for dictation and voice chat, and lets websites you allow use it.

`NSMicrophoneUsageDescription` 同时描述原生听写、语音聊天和获准网站的麦克风用途。摄像头说明保持网站用途。用途说明不授予系统访问权限。

- 原生主进程实际签名选用 `scripts/macos_voice_entitlements.plist`，仅包含 `com.apple.security.device.audio-input = true`。
- WebView helper 实际签名仍选用 `scripts/macos_webview_entitlements.plist`，保留原有 camera/audio-input；不改未使用的 `macos-entitlements.plist`。
- Developer ID 模式保留 Hardened Runtime 和原有时间戳/验证；本地 ad-hoc 模式逐个从内到外签名，避免递归签名将主进程与 helper 的权限混用。未加入沙箱、调试或额外运行时例外。
- 输入监控由 IOHID API 查询/申请，辅助功能由 AX 信任查询和系统设置引导，屏幕录制由 CoreGraphics/ScreenCaptureKit 与系统设置管理。不添加臆造的 plist 权限键，不操作 TCC 数据库，不代点系统授权。

## 本次验证

`python3 -m unittest discover -s scripts/tests -p test_voice_packaging.py`：4 项实际执行。使用临时 bundle 运行完整生产脚本、真实 PlistBuddy，替代编译、签名和发布边界；分别覆盖用途键新增/覆盖、Developer ID/ad-hoc 两条签名路径、实际选用的 entitlement、`dev` profile 从 `debug/` 读取产物。测试禁止公证/归档/上传。另执行相邻 `test_macos_signing.py` 17 项回归。

红测：原生用途说明断言与主进程 entitlement 断言失败，已有 helper audio-input 回归通过；随后实际打包发现 `dev` profile 路径错误，新增测试复现查找不存在的 `dev/shi-ju`，修正为 Cargo 实际使用的 `debug/`；最终绿测 4 项全部通过。此结果证明打包行为，不证明真实音频或系统授权完成。

已实测：macOS 27.0 / Apple Silicon，本地 `dev` 构建与 ad-hoc 打包成功。真实 `.app` 的 Info.plist 用途说明、主进程仅 audio-input、helper 保留 camera/audio-input 三条回读断言通过；生产脚本的 `codesign --verify --deep --strict --verbose=2` 退出 0。应用从生成的 `.app/Contents/MacOS/shi-ju` 启动，短时存活并通过原生 UI 观察确认暗色中文项目主窗口正常显示；随后主动结束本次检查进程。没有点击录音、系统授权或发送服务请求。

在已领取的共享槽位内执行的关键命令（全部产物留在该槽位）：

```bash
CARGO_INCREMENTAL=0 cargo build -p vw-desktop --bin shi-ju -p vw-webview --bin vw-webview
SKIP_BUILD=1 PROFILE=dev INCLUDE_CLI_IN_APP=0 MACOS_SIGN_MODE=adhoc MACOS_NOTARIZE=0 PACKAGE_APP=0 CREATE_DMG=0 bash scripts/bundle_macos.sh
```

首轮增量构建在 helper 链接时缺少 `vw-api-types` 内 serde/LLVM 符号；关闭增量后构建成功，未清除共享缓存。`PROFILE=debug` 被 Cargo 拒绝，实际复验使用 `PROFILE=dev`，脚本已正确映射到 `debug/`。产物为 `$CARGO_TARGET_DIR/debug/bundle/osx/ShiJu.app`。

全仓 `cargo clippy --all-targets --all-features -- -D warnings` 已执行，退出 101：当时其它在途 MCP 模块调用私有 `agent_apps_diagnostics::start`（E0603），本次不改动该模块，仍需重跑。Developer ID、公证、用户真实麦克风授权弹窗、录音/播放、权限撤销及收费服务链路未经本次实测；任务 18 保留“已完成，待回归”，由任务 19 继续回归。

本机复验须先动态领取仓库共享槽位，设置绝对 `CARGO_TARGET_DIR`（同时设置 `CARGO_BUILD_BUILD_DIR`，避免仓库 build-dir 配置将中间产物写出槽位）。使用已有脚本环境参数，关闭 `PACKAGE_APP`、`CREATE_DMG`、`MACOS_NOTARIZE`；构建和应用进程全部结束后释放自有锁，保留编译缓存。
