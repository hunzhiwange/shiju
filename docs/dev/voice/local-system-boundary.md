# macOS 本机语音系统边界

用户于 2026-11-12 明确选择无需云 API Key 的本机方案。本次只提供系统边界，原云服务契约与旧计划保持不变；不实现 UI、听写回填或对话路由。

## 给任务 7、11、13 的接口

模块：`vw_desktop::app::voice::local`，实现文件 `local.rs`、`local_macos.rs`。

| 接口 | 消费方式 |
| --- | --- |
| `capabilities(language: Option<&str>) -> Result<Capabilities>` | 只读查询解析后的语言、本机支持、服务可用性、Speech 授权和本机音色；不申请权限、不安装资源 |
| `voices() -> Result<Vec<SystemVoice>>` | 独立于识别语言/权限列出已安装 Apple 系统音色；ID、语言、名称可用于本机设置和试听 |
| `transcribe_pcm(pcm: &[i16], language: Option<&str>, cancel: &Cancellation) -> Result<Transcript>` | 直接消费现有 `capture::Recording.pcm`：24kHz、单声道、PCM16，最多60秒；仅最终文本，错误没有文本载荷 |
| `synthesize(text: &str, voice_id: Option<&str>, cancel: &Cancellation) -> Result<SpeechAudio>` | 最多2000字符，输出最多120秒的24kHz单声道PCM16，不自动播放，不使用第三方语音提供商或未安装音色 |
| `Cancellation::{default, clone, cancel, is_cancelled}` | 每个操作独立令牌；取消后停止原生任务并丢弃迟到回调；不要复用已取消令牌 |

识别和合成是同步阻塞接口，桌面调用方放在 `tokio::task::spawn_blocking` 或其现有工作线程中，不能阻塞 Iced 更新/绘制。合成依赖桌面已有的 macOS 主运行循环；普通 Rust 测试线程没有主循环，不能拿它替代原生系统验收入口。无麦克风采集、扬声器播放或外部网络请求由这些接口隐式启动。

`None` 语言使用系统识别器语言，**不是自动语言检测**；可指定 `zh-CN` 等 BCP-47 标签，兼容 `_` 分隔符。返回的 `Transcript.language` 是解析后的系统语言。调用者继续保存原录音的 generation/会话身份并在异步结果回填时核对；已完成返回值不能替代上层迟到消息检查。

合成的 `SpeechAudio.pcm` 需要转为 little-endian bytes 并分包送入现有 `Playback::push_pcm`。每包不超过200ms，按 `played_ms()` 控制提交量；现有2秒队列遇到溢出会停止，不能把整段合成结果一次塞入队列。结束/打断时既取消合成令牌，也停止已有 `Playback`。

## 权限和严格本机约束

新增 `Permission::SpeechRecognition`、`VoiceCapability::Transcription`。录音仍仅要求 `Microphone`；已有 PCM 的转写只要求 Speech 权限，不额外要求麦克风、辅助功能、输入监控或屏幕权限。

由实际用户点击触发：

```rust,ignore
VoicePermissions::new(SystemPermissions)
    .request_from_user_action(Permission::SpeechRecognition)?;
// 系统弹窗由用户本人处理；返回应用后 refresh(SpeechRecognition)。
```

`bundle_macos.sh` 在签名前写入 `NSSpeechRecognitionUsageDescription`。开发用无此用途说明的非打包进程申请权限会明确报错；查询与转写前置检查不会申请权限，也不会重置 TCC。原任务3的用户授权/拒绝/撤销实测缺口继续保留。

识别创建 `SFSpeechURLRecognitionRequest`，只有当前识别器 `supportsOnDeviceRecognition=true` 才启动，并始终设置/回读 `requiresOnDeviceRecognition=true`；关闭 partial results。无本机支持、未授权、资源不可用、超时均报错，绝不调用联网识别或下载/安装 API。输入只临时写入0600的 WAV 文件，任务取消/结束后清理；不保存历史，不将音频、输入文字、转写或系统 NSError 描述写入日志。Debug 只展示语言和计数。

[Apple 的本机模式契约](https://developer.apple.com/documentation/speech/sfspeechrecognitionrequest/requiresondevicerecognition)说明：只有识别器支持本机识别时该标志才生效。[Speech 用途说明](https://developer.apple.com/documentation/bundleresources/information-property-list/nsspeechrecognitionusagedescription)即使强制本机模式也必须存在。[AVSpeechSynthesizer](https://developer.apple.com/documentation/avfaudio/avspeechsynthesizer)负责本机合成与停止。

## 实际系统证据与限制

当前 macOS27/arm64 SDK 同时提供 Speech、AVSpeechSynthesizer 和新的 SpeechAnalyzer。只读 Swift 探测显示：旧引擎 `zh-CN` 本机支持为 true，`zh-TW` 为 false；SpeechAnalyzer 可用但 installedLocales 为空。选择已具备中文本机支持的旧引擎，不自动下载新模型，也未引入 Swift 运行时桥或新 Rust 依赖。

实际 Rust 主运行循环探测已生成77,335个非静音中文PCM样本（24kHz），取消场景总用时约63ms（包含发出取消前的50ms等待）。当前 Speech 授权为 NotDetermined：**不能据此声称中文转写/听辨已经成功**；只有用户授权后实际离线转写返回，才能补齐这项证据。缺少本机语言资源时请用户在 macOS 键盘/听写或辅助功能朗读设置管理语言/音色，程序不静默联网回退。

## 显式系统验收

先遵守编号任务中的共享 Cargo 槽位规则，令 `CARGO_TARGET_DIR` 与 `CARGO_BUILD_BUILD_DIR` 为同一绝对槽路径，再执行：

```sh
cargo test -p vw-desktop --lib local_voice_ -- --nocapture
cargo test -p vw-desktop --test local-speech-system --
cargo test -p vw-desktop --test local-speech-system -- --transcribe
```

第二条驱动真实 macOS 主循环，后台调用生产边界，验证中文本机合成与取消；输出仅计数/状态。第三条进一步尝试强制本机中文转写，缺 Speech 授权时明确退出2，不能算识别通过。入口不自动申请权限，不接受系统弹窗，不播放音频。它的进程权限身份也不能冒充安装版 ShiJu 的权限实测。
