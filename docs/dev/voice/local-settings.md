# 本机语音设置与独立音色试听（任务13）

用户入口：系统设置 → 语音 → 编辑语音设置。macOS 缺省选择「macOS 本机（无需语音 API Key）」。该模式无需网关配对、`voice_config`、provider 或云语音凭据即可读取、编辑、保存本机偏好；网关失败不会触发云端回退。

本机识别语言空值表示「系统语言」，不是自动检测。选项由系统声音语言及常用中文/英文候选经 `local::capabilities` 的 `supports_on_device` 确认后列出；系统报告支持仍可能缺本机资源或授权，表单分别展示支持、服务可用性与 Speech/Mic 授权状态。Apple 音色来自 `local::voices()` 的已安装列表，不下载资源；已移除的保存值明确提示重选。刷新按钮重新查询权限与资源，不发起授权。

Speech 与麦克风有独立的授权按钮，仅用户点击才调用现有权限边界；系统弹窗由本人处理。拒绝、系统限制或缺用途说明时给出中文操作指引。独立试听不查询或申请识别/麦克风权限，只用 `local::synthesize` 合成固定短句，再使用任务11的100ms分包、约1秒领先量播放。它不启动录音、不访问聊天、模型或工具。停止试听、切换音色、取消、关闭、切换上下文均取消独立令牌；UI线程不等待合成线程退出，避免阻塞macOS主循环。

本机识别暂不支持自定义词典，表单明确说明，不修改已有云端词典。只有显式选择「云端语音」才加载原租户网关配置；失败保留表单，可重试或切回本机。云端保存继续原增量PATCH与校验，本机字段只写本机原子偏好文件。取消不落盘，异步结果按编辑代次及端点身份拒绝迟到更新。更换引擎、识别语言、声音或麦克风会先结束活动语音，保存后的下一轮听写/对话才读取新偏好。

## 给任务19的稳定接口

- 偏好仍为 `voice::preferences::Preferences::{engine,local_language,local_voice,microphone,retain_recordings,hotkeys}`，`VoiceEngine::{Local,Cloud}`，没有新增网关字段。
- UI使用 `SettingsMessage::Voice(Event::{Open,Engine,LocalLanguage,LocalVoice,Preview,StopPreview,Save,Cancel,RefreshLocal,Permission})`；Permission由表单的SpeechRecognition/Microphone按钮显式触发。原生设置渲染由既有 `system_settings_voice::overlay` 提供。
- 本机试听复用 `voice::local_conversation::speak`，固定短句与独立取消所有者在 `voice::preview::Preview::start_local`；普通聊天请求路由保持任务11的入口。
- 显式 `cargo test -p vw-desktop --test voice-window -- --scenario local-settings` 在真实macOS主循环中合成固定短句并播放、请求取消第二次试听，之后保存六张明暗原生截图；无需Speech/Mic授权。先遵守共享槽位协议并设置同值绝对CARGO_TARGET_DIR/CARGO_BUILD_BUILD_DIR。它证明系统合成/播放与绘制路径，不证明人耳听辨。原 `--scenario deterministic` 保持不播放音频；旧 `--scenario live` 是Cloud专用小窗口，不证明Local普通聊天模型链路。
- 正式 ShiJu.app 中还需本人完成 Speech/Mic 授权、选择声音实际听辨、取消/保存后的下一轮听写与普通模型对话。缺少这些真人结果只记待回归，不阻止软件实现。

## 软件验证（2026-11-12）

最新共享工作区的任务13聚焦14项全通过，全仓 `cargo clippy --all-targets --all-features -- -D warnings` 通过。显式 `local-settings` 原生入口完成实际系统合成/输出回调消费及取消、六帧/两主题绘制，运行21014ms；实际复看明暗设置顶部与长表单底部。Speech仍为尚未请求，试听不以Speech授权为前提。未据此声称本人听辨、实际识别或正式签名.app权限场景通过；这些仍由任务19执行。
