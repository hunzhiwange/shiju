# 本机离线语音边界计划

目标：为桌面提供无需云密钥的本机识别、语音合成、取消、音色与能力查询接口。

范围：消费现有24kHz单声道PCM16采集产物；可选语言；Speech识别授权及用途说明；有界合成PCM输出；严格离线、取消和明确失败；不修改既有语音计划。

范围外：UI、会话路由、下载模型、收费服务、代点系统权限、重置TCC。

假设：当前SDK为macOS27；zh-CN的SFSpeechRecognizer报告本机支持，SpeechAnalyzer无已安装资源。实际转写还需用户授权。

## 设计决策

- Rust/objc直接调用SFSpeechRecognizer与AVSpeechSynthesizer；不用运行时Swift进程或重型依赖。
- 对比新SpeechAnalyzer：虽支持中文但需另装资源，当前不选；旧Speech只有同时supportsOnDeviceRecognition且requiresOnDeviceRecognition时才允许识别。
- 合成输出24kHz PCM供已有播放边界使用，不自动播放；仅枚举已安装Apple系统音色，不调用第三方语音提供商。

## 架构说明

- 新增voice::local公共同步边界供调用者置于后台任务；Cancellation可跨线程取消；OS对象只驻留调用线程。
- 能力查询不申请权限；识别申请由单独用户动作触发；音频/文本不记日志，超时/取消停止原生任务并清理临时文件。

## 开发策略

- 单批次单任务，无并行代理；新增独立行为测试先红后绿，先实测SDK能力再实现。
- 只替代OS权限/原生回调边界，真实运行参数、离线门禁、重采样和取消逻辑；本机探测和合成另取实际证据。
- 执行共享Cargo槽位协议、聚焦测试、全仓Clippy；保留用户权限实测事实，不将能力标志当中文听辨结果。
