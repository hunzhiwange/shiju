# 语音集成验收

## macOS 本机路线：直接使用完整 ShiJu.app

macOS 默认引擎是「macOS 本机（无需语音 API Key）」。听写、已有录音重识别和音色试听不要求云语音 Key、网关语音配置、bootstrap 环境变量或预先创建的验收会话。语音聊天是「本机识别 → 当前会话普通聊天模型 → 本机朗读」；模型可能在远端运行，因此不称为全链路离线聊天。录音不上传语音服务，不自动下载大型模型，不静默切换到 Cloud。

1. 打开本轮准备的完整 `ShiJu.app`。开发验收包沿用应用已有的开发版连接与普通聊天配置，不覆盖已有 `.app` 或配置。普通模型目录可见不等于实际推理已验证；若普通聊天还不可用，先在正常设置中选好连接和模型，听写与试听仍可独立试用。
2. 系统设置 → 语音 → 编辑语音设置：选择「macOS 本机」，识别语言选择「系统语言」或支持的「简体中文」，朗读声音选择已安装中文音色。系统语言不是自动语言检测。本机暂不支持自定义词典。
3. 分别点击 Speech 与麦克风授权按钮，由本人处理 macOS 弹窗，返回后点击刷新。试听仅使用本机合成和扬声器，不要求 Speech/Mic 授权；先试听并确认人耳可听，再保存。取消不保存本次偏好。
4. 进入普通会话，点击输入框麦克风，中文说话后在60秒内点完成：结果只回填草稿，不自动发送。取消或切换会话后，迟到结果不得写入。
5. 清空草稿、等待普通聊天空闲，关闭任务/工作流模式，点击语音聊天 → 开始录音 → 停止并发送。识别文字送到同一会话当前普通模型，保留历史和审批；本人处理工具审批。最终正文才由本机朗读，思考、工具、代码和表格不读出。本机模式在输入框上方展示月亮与紧凑控件，正文、输入框、模型和审批入口保持可用；仍为逐轮对话。
6. 分别测试静音、打断和结束；结束应释放录音、本机识别/合成和播放，只停止该通话拥有的模型请求。Local 主动屏幕分享默认关闭，每次选择并确认单窗口后仅发送一帧给当前普通聊天图像模型；取消、结束、会话切换或撤权后拒绝迟到帧。采集端不另存截图，网关与模型按聊天策略处理、保存图像。

Speech/Mic 弹窗、真实中文听辨、权限拒绝/撤销、物理设备拔插、跨应用目标和后台热键必须由操作者实测，不自动点击、重置 TCC 或确认已听到。未获得这些证据时保留编号任务的待回归状态；缺云语音 Key不是本机路线的障碍。

## 软件与原生系统入口

显式入口不属于默认测试。每次调用 Cargo 前，按任务19共享协议动态领取 `target/rust-slot-1` 至 `target/rust-slot-20`，设置同值绝对 `CARGO_TARGET_DIR` / `CARGO_BUILD_BUILD_DIR`；Cargo、窗口及所有槽位产物消费者退出后释放自身锁，保留缓存。

```sh
# 完整 App 消息路由、互斥、取消/迟到和真实 HTTP/WS 生命周期
cargo test -p vw-desktop --lib voice_task_19_ -- --nocapture --test-threads=1
# 本机与 Cloud 现有聚焦回归，包含普通模型/历史/审批策略、偏好取消/保存
cargo test -p vw-desktop --lib voice_task_ -- --nocapture --test-threads=1
# 本机边界与权限门禁；不自动授权
cargo test -p vw-desktop --lib local_voice_ -- --nocapture
# 隔离 HTTP/WS + 六张原生明暗截图，不播放或录音
cargo test -p vw-desktop --test voice-window -- --scenario deterministic
# 实际 macOS 主循环合成 → 有界播放 → 取消，并生成六张明暗截图；不录音
cargo test -p vw-desktop --test voice-window -- --scenario local-settings
# 真实本机合成和取消；--transcribe 需要本人已授予 Speech 权限
cargo test -p vw-desktop --test local-speech-system --
cargo test -p vw-desktop --test local-speech-system -- --transcribe
```

`VOICE_ACCEPTANCE_OUTPUT` 可选，仅指定截图目录。`local-settings` 和系统探测证明实际合成/输出回调及取消，不证明人耳听辨。未变动的原生路径可复用已有证据，不为凑记录反复编译。普通模型完成事件在完整 App 中转发；小型 `voice-window` 不承载本机普通聊天验收，也不能替代完整应用。

集成测试只替代物理音频及模型边界：验证完整 `App::update` 的本机普通聊天请求、仅最终正文触发朗读、结束拒绝迟到结果，以及真实本机试听取消所有者、草稿保护和互斥。现有任务7/11/13还覆盖本机路由不访问语音网关、模型/历史/审批沿用、原子偏好保存与取消。模拟完成事件不等于已执行真实普通模型推理或工具。

## 开发验收包

遵守共享槽位后使用既有 `scripts/bundle_macos.sh`，`PROFILE=dev`、`MACOS_SIGN_MODE=adhoc`、`PACKAGE_APP=0`、`CREATE_DMG=0`、`MACOS_NOTARIZE=0`。完整包包含正常 `shi-ju`、`vw-webview` 和 `shiju` CLI。若槽位已有包，先保留旧包，再将新包复制到唯一验收目录；不覆盖用户应用，不上传、安装或公证。检查签名、主程序与辅助程序、图标及 `NSMicrophoneUsageDescription` / `NSSpeechRecognitionUsageDescription`，从复制后的完整包启动。确切路径和本轮构建证据记录于任务19。

## 用途说明诊断与本地启动

授权提示缺少 `NSSpeechRecognitionUsageDescription` 或 `NSMicrophoneUsageDescription` 时，先核对运行进程的可执行路径与对应包的 `Contents/Info.plist`。裸 `cargo run` 或自建临时 `.app` 不保证携带用途说明；仅有目录后缀不能证明它是支持的完整包。门禁会在请求系统授权前明确报错，不能删除门禁绕过。

2026-11-13 只读现场检查：运行中的 `/Applications/ShiJu.app`（`com.rsung.shiju`，0.6.0，20261113.031307，adhoc）具备两项说明；另一个临时 DimCode 验收包（`com.rsung.shiju.dimcode-validation`）两项均缺失，签名显示 `Info.plist=not bound`。这些证据说明临时包不具备语音授权前提，尚不能确认用户截图来自哪个进程。没有关闭、替换或重新授权任一实例。

本地生成并检查完整开发包（入口打印实际槽位；不安装、不发布）：

```sh
python3 scripts/cargo_slot.py sh -c '
  MODE=app PROFILE=dev PACKAGE_APP=0 CREATE_DMG=0 INCLUDE_CLI_IN_APP=0 MACOS_SIGN_MODE=adhoc MACOS_NOTARIZE=0 bash scripts/bundle_macos.sh || exit $?
  app="$CARGO_TARGET_DIR/debug/bundle/osx/ShiJu.app"
  for key in NSSpeechRecognitionUsageDescription NSMicrophoneUsageDescription CFBundleIdentifier CFBundleShortVersionString; do
    plutil -extract "$key" raw -o - "$app/Contents/Info.plist" || exit $?
  done
  codesign -d --entitlements :- "$app"
  codesign -d --entitlements :- "$app/Contents/MacOS/vw-webview"
'
```

主包仅有 audio-input entitlement；网页 helper 有 camera 与 audio-input。重复打包夹具覆盖旧用途说明并检查两个签名边界；编译和签名替身不能证明真人授权成功。验收时从完整包启动，应用使用槽位产物期间须持续持有槽位入口；不要将异步 `open` 结束误当作应用退出。缺少有效产物或真人授权结果时不得宣称可录音。

## Cloud 显式路线

选择「云端语音」后才加载租户语音配置、provider和云语音服务凭据。原实时通道、词典和主动屏幕上下文继续按原约束验收，不把这些 Cloud 场景当成本机支持。

```sh
# 兼容保留的 Cloud 专用隔离入口；不是本机路线的前提
cargo test -p vw-desktop --test voice-window -- --scenario live
```

仅此旧 Cloud 入口要求显式 `VOICE_ACCEPTANCE_GATEWAY_BOOTSTRAP`（已配置 ShiJu 网关文件）和 `VOICE_ACCEPTANCE_SESSION`（专用现有会话），验证回环网关、听写/实时开关和provider。它不会寻找 Codex 凭据或上传历史录音；缺条件非零退出。无 Cloud Key 时不运行该收费路线，也不阻止本机包交付。小窗口人工确认不替代正式 `.app` 实机清单。

## 待操作者确认的结果

| 场景 | 必须观察的行为 |
| --- | --- |
| 中文/英文听写、取消、忙碌态 | 原光标回填一次、不自动发送；取消/切换后不写入；听写/聊天/回放/试听互斥 |
| Speech/Mic 独立授权 | 本人处理弹窗；刷新准确；拒绝或撤销不采集、不绕权 |
| 拔出、重选设备 | 旧流停止、错误可见、重新选择后恢复 |
| 音色试听与偏好 | 本人可听、取消不保存、保存后的下一轮生效 |
| 普通模型与审批 | 同一会话模型/历史/工具审批，拒绝不执行，只朗读最终正文 |
| 静音、打断、结束 | 未发送录音可取消；停止本轮媒体和自有请求，拒绝迟到结果 |
| 后台热键与跨应用 | 显式启用并授权；固定可编辑目标，焦点变化/安全字段/关闭/取消拒绝写入 |
| 最近二十段 | 默认不保存；启用后仅本机保留，播放/重识别/删除有效 |
| 正式完整应用 | 本轮包的用途说明、权限归属、真实录音/播放和明暗中文显示正确 |
| Cloud 专属能力（选择时） | 云端词典、实时流和每次确认的屏幕上下文按原约束；Local 单帧分享使用当前普通聊天图像模型，与 Cloud 实时图像通道分别验证 |

只记录场景、状态、计数、队列峰值与释放耗时，不保存音频、转写、屏幕或凭据到日志。当前交付就绪与软件证据、真人尚未确认的事实分别记录在任务19；没有操作者结果不删除该任务或伪造全部完成。

## 月亮界面本轮回归

2026-11-13 的命令、产物检查与真人验收缺口见 [月亮界面验收记录](moon-interface-acceptance.md)。软件验证与真人授权、中文识别和听辨分别记录。
