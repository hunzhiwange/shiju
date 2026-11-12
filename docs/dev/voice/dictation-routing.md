# 输入框听写引擎交接

输入框开始、完成、取消与近期录音重识别按 `app::voice::preferences::Preferences` 路由。Local 不读取网关语音配置、不要求 provider/回环地址、不自动回退或上传云端；Cloud 必须显式选择，并沿用网关配置和回环约束。

## 已落地偏好

- `engine: VoiceEngine`，序列化为 `"local"` / `"cloud"`。旧偏好缺字段时，macOS 默认 Local，其它平台默认 Cloud。未知引擎拒绝解析。
- `local_language: Option<String>`，默认 `None`，表示系统语言，**不是自动检测**；显式值使用冻结本机边界接受的 BCP-47 标签。
- 保存应保留现有 microphone、热键、retain_recordings 等字段。`local_voice` 尚无听写消费者，留给任务11/13在接入系统合成时添加。

## 听写生命周期

`app::message::voice::dictation` 消费冻结的 `app::voice::local` API；Local 直接传递 Recording.pcm（24kHz、单声道、i16），采集上限60秒。近期录音重识别使用严格 WAV 解析与相同60秒限制，不另开麦克风。Cloud 保留原180秒采集上限。

显式开始动作在后台查询能力、请求 SpeechRecognition 和 Microphone；两者未授权不打开采集。显式重识别只请求 SpeechRecognition。拒绝、缺离线资源、超时及格式错误均提供中文反馈，无云端回退。

准备和识别使用 `local::Cancellation`；取消、准备超时、识别超时、上下文变化会取消令牌。原生迟到结果仍被令牌和已有 Scope/cursor/generation 校验拒绝。正常完成解除取消守卫；结果只回填草稿，近期录音结果仍需显式应用，不自动发送。

## 任务11/13接入点

任务11复用 `engine` / `local_language` 选择对话引擎，按冻结边界调用本机识别与合成；本轮未改造对话链路。添加真实合成消费者时再新增 `local_voice`，取消时同时停止原生识别、合成和播放。

任务13的本机设置路径不应依赖 gateway voice_config；语言空值显示“系统语言”。引擎或语言变更应先取消活动语音生命周期再保存；权限请求保持显式用户动作。本轮未实现完整设置表单。

## 回归边界

自动化验证覆盖本机路由、权限门禁、取消/迟到回调、超时、PCM及近期录音重识别。冻结系统边界的真实 Speech 状态尚未授权，正式签名 .app 中真人批准、录音、回填、取消及上下文变化仍待操作者回归；缺少这项证据不代表本机代码实现被云端 Key 阻塞。
