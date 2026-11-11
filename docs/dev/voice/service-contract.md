# 公开语音服务契约 v1

核对日期：2026-11-11。本文是待实现的协议基线，不代表路由、音频设备或服务已验收。首个 provider key 为 `openai`；听写在录音结束后上传文件，语音聊天走网关代理的公开 Realtime WebSocket。示例均为合成数据，认证头仅写占位符，不含凭据。

## 来源与能力快照

以下均为本次实际读取的 OpenAI 官方文档。后续接入应重新核对；账户授权、配额和可用性须以真实服务验证，不能由此快照推定。

| 来源 | 本次采用的事实 |
| --- | --- |
| [转写概览](https://developers.openai.com/api/docs/guides/transcription) | 文件转写、实时转写、语音对话是不同工作流 |
| [文件转写](https://developers.openai.com/api/docs/guides/speech-to-text) | 文件上限 25 MB；推荐 `gpt-transcribe`；语言提示与词典提示 |
| [转写请求与响应](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create) | multipart、必填 file/model、JSON text/languages；参考页另列 flac/ogg，指南未列，首版不采用 |
| [WebSocket，Realtime 分支](https://developers.openai.com/api/docs/guides/voice-websockets?api=realtime) | `/v1/realtime?model=…`、Bearer、文本 JSON 事件；该页另一分支的 `/v1/live/sessions` 不在本契约内 |
| [实时会话](https://developers.openai.com/api/docs/guides/realtime-conversations) | 会话上限 60 分钟；十种内置音色；打断需清空本机播放并截断服务历史 |
| [客户端事件](https://developers.openai.com/api/reference/resources/realtime/client-events) | 嵌套 audio 配置、24 kHz PCM、commit/clear/cancel/truncate、function_call_output |
| [服务端事件](https://developers.openai.com/api/reference/resources/realtime/server-events) | 字幕/音频增量及完成、工具参数、response.done、error；done 也可能表示取消 |
| [转写模型](https://developers.openai.com/api/docs/models/gpt-transcribe)、[实时模型](https://developers.openai.com/api/docs/models/gpt-realtime-2.1) | 两种能力独立配置；实时模型支持工具调用，使用 `/v1/realtime` |

网关配置分别保存 `transcription_model`、`realtime_model`、`voice`、`language` 与 `dictionary`，通过已有 provider/secret 配置解析凭据，不增加桌面密钥存储或第二套连接配置。v1 可配置模型集合分别为 `{gpt-transcribe}`、`{gpt-realtime-2.1}`，默认即该值；后续扩展必须补齐能力和协议验证，不接受任意模型字符串。官方当前已将旧转写系列标为弃用，因此本次不以旧模型作为默认值。[文件转写](https://developers.openai.com/api/docs/guides/speech-to-text)

音色允许集合为 `alloy, ash, ballad, coral, echo, sage, shimmer, verse, marin, cedar`，默认 `marin`。首次生成音频后更换音色需要结束并新建会话。没有采用公开动态音色目录；UI 使用这份有日期的静态能力清单，不能把 Codex `listVoices` 当公开 API，也不能加入截图的 Cove。试听使用同一实时模型的短会话与固定文本，沿用音频限制；不假设实时模型可调用 `/audio/speech`。[音色约束](https://developers.openai.com/api/docs/guides/realtime-conversations#voice-options)

v1 产品允许的识别语言集合为 `auto, en, es, fr, zh`，这是官方可用代码的保守子集，不声称穷尽语言支持。`auto` 是本地枚举，服务请求省略语言字段；其余转为 `languages: [code]`，不同时发送 `language`。输入字幕的转写也使用 `gpt-transcribe` 与同一映射；对话的回复语言用 instructions 提示，不保证强制语言识别。检测结果允许为空或为集合外代码，不伪造为用户指定语言。[语言](https://developers.openai.com/api/docs/guides/speech-to-text#supported-languages)、[实时转写字段](https://developers.openai.com/api/reference/resources/realtime/client-events#session.update)

## 网关到 OpenAI：文件听写

`POST https://api.openai.com/v1/audio/transcriptions`，`Authorization: Bearer <gateway-resolved-service-key>`，multipart 由 HTTP 库生成 boundary。必填 `file` 是文件字节而非路径，带文件名与正确媒体类型；`model=gpt-transcribe`。首版发送 `response_format=json`、`stream=false`，可选 `languages[]`、`keywords[]`、`prompt`，不请求说话人、时间戳或翻译。[请求字段](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create)

公开指南接受 mp3/mp4/mpeg/mpga/m4a/wav/webm，最大 25 MB。ShiJu v1 只上传 WAV（`audio/wav`，单声道 24 kHz、16 位有符号小端 PCM），文件字节最多 24,000,000，录音最多 180 秒，二者取先到者。这些更严格的媒体、大小与时长是 ShiJu 决策，服务文件接口没有在此要求 24 kHz。服务器检查实际容器、采样元数据和时长，不能只信扩展名。[服务格式与上限](https://developers.openai.com/api/docs/guides/speech-to-text)

词典映射为 `keywords[]`（提示，不是强制替换），拒绝包含 `<`、`>`、CR/LF 的词条；prompt 只放识别上下文，不放执行命令。ShiJu 自定词典上限 100 条、每条 80 个 Unicode 标量、prompt 2,000 个标量；若服务仍拒绝则返回结构化错误，不静默截断或重试旧模型。[提示限制](https://developers.openai.com/api/docs/guides/speech-to-text#add-transcription-context)

完整字段示例（占位符 audio 字节由真实文件替代；不应手写固定 boundary）：

```http
POST /v1/audio/transcriptions HTTP/1.1
Host: api.openai.com
Authorization: Bearer <gateway-resolved-service-key>
Content-Type: multipart/form-data; boundary=synthetic-voice

--synthetic-voice
Content-Disposition: form-data; name="model"

gpt-transcribe
--synthetic-voice
Content-Disposition: form-data; name="response_format"

json
--synthetic-voice
Content-Disposition: form-data; name="stream"

false
--synthetic-voice
Content-Disposition: form-data; name="languages[]"

zh
--synthetic-voice
Content-Disposition: form-data; name="keywords[]"

ShiJu
--synthetic-voice
Content-Disposition: form-data; name="file"; filename="dictation.wav"
Content-Type: audio/wav

<synthetic WAV bytes>
--synthetic-voice--
```

```http
HTTP/1.1 200 OK
Content-Type: application/json

{"text":"打开 ShiJu 设置。","languages":[{"code":"zh"}]}
```

这是最终文本，`languages: []` 表示未可靠检测；可忽略上游新增的 usage 等字段。非 2xx、缺失 text、无效 JSON 都返回失败；不把字幕片段拼成伪成功。[响应](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create)

## 网关到 OpenAI：Realtime

连接 `wss://api.openai.com/v1/realtime?model=gpt-realtime-2.1`，握手使用网关解析的 Bearer 服务凭据；可发送由本方生成、不可反推用户身份的 `OpenAI-Safety-Identifier`。不附带 ChatGPT cookie、私有 intent/architecture 参数或 Beta 协议头。等待 `session.created`，发送配置并收到 `session.updated` 后才向桌面确认 ready。[握手](https://developers.openai.com/api/docs/guides/voice-websockets?api=realtime)

输入、输出均选择 `audio/pcm`、24000 Hz，PCM16 单声道小端原始字节，不带 WAV 头；JSON 中用标准 Base64。设备原生格式必须在本机转换，不能仅改采样率标签。公开协议另支持 G.711，首版不协商；不能将 Live 分支允许的 16 kHz 搬到 Realtime。[格式](https://developers.openai.com/api/reference/resources/realtime/client-events#session.update)

以下是一轮手动提交的完整应用消息顺序；`AAAAAA==` 是两个静音采样的编码示意，实际一轮必须累积真实有效音频，不使用该短片段做服务成功验证。服务端示例只展示本方消费的字段。

```jsonl
{"type":"session.created","event_id":"e1","session":{"id":"sess_demo","type":"realtime"}}
{"type":"session.update","event_id":"cfg1","session":{"type":"realtime","output_modalities":["audio"],"instructions":"用简短中文回答。需要执行工作时调用 shiju_handoff；遵守审批结果。","audio":{"input":{"format":{"type":"audio/pcm","rate":24000},"transcription":{"model":"gpt-transcribe","languages":["zh"],"keywords":["ShiJu"]},"turn_detection":null},"output":{"format":{"type":"audio/pcm","rate":24000},"voice":"marin"}},"tools":[{"type":"function","name":"shiju_handoff","description":"交给当前 ShiJu 会话执行用户工作，沿用权限审批。","parameters":{"type":"object","properties":{"instruction":{"type":"string"}},"required":["instruction"],"additionalProperties":false}}],"tool_choice":"auto"}}
{"type":"session.updated","event_id":"e2","session":{"id":"sess_demo","type":"realtime","output_modalities":["audio"]}}
{"type":"input_audio_buffer.append","event_id":"a1","audio":"AAAAAA=="}
{"type":"input_audio_buffer.commit","event_id":"commit1"}
{"type":"input_audio_buffer.committed","event_id":"e3","item_id":"user1","previous_item_id":null}
{"type":"response.create","event_id":"create1"}
{"type":"conversation.item.input_audio_transcription.completed","event_id":"e4","item_id":"user1","content_index":0,"transcript":"你好。","languages":[{"code":"zh"}]}
{"type":"response.created","event_id":"e5","response":{"id":"r1","status":"in_progress"}}
{"type":"response.output_audio_transcript.delta","event_id":"e6","response_id":"r1","item_id":"assistant1","output_index":0,"content_index":0,"delta":"你好"}
{"type":"response.output_audio.delta","event_id":"e7","response_id":"r1","item_id":"assistant1","output_index":0,"content_index":0,"delta":"AAAAAA=="}
{"type":"response.output_audio_transcript.done","event_id":"e8","response_id":"r1","item_id":"assistant1","output_index":0,"content_index":0,"transcript":"你好。"}
{"type":"response.output_audio.done","event_id":"e9","response_id":"r1","item_id":"assistant1","output_index":0,"content_index":0}
{"type":"response.done","event_id":"e10","response":{"id":"r1","status":"completed","output":[]}}
```

生产中的 `session.updated` 有完整有效配置，网关必须校验实际格式/音色；上面是消费字段投影，不能伪造确认。输入转写与回答异步，顺序可交错。commit 只提交输入，不触发回答；手动模式随后发送一次 response.create。免提模式改用 `server_vad`，显式设 `create_response=true, interrupt_response=true`，处理 speech_started/stopped 与自动 committed，不重复手动提交。[事件流程](https://developers.openai.com/api/docs/guides/realtime-conversations)、[事件定义](https://developers.openai.com/api/reference/resources/realtime/client-events)

必须消费的服务事件及对应结果：

| 事件 | ShiJu 处理 |
| --- | --- |
| session.created / session.updated | 握手、确认有效配置；配置失败不启用设备采集 |
| input_audio_buffer.speech_started / speech_stopped / committed | 免提回合边界；speech_started 立即停止旧播放 |
| conversation.item.input_audio_transcription.delta / completed / failed | user 字幕增量/最终/识别失败；不触发工作执行 |
| response.created / output_item.added / output_item.done | 建立 response→item→generation 映射，检查工具完成状态 |
| response.output_audio.delta / done | 音频入有界队列/流结束；done 不等于播放完或回答成功 |
| response.output_audio_transcript.delta / done | assistant 字幕，最终全文替换增量，不重复追加 |
| response.function_call_arguments.delta / done | 只缓冲参数，完成并确认工具 item 完整后才验证执行 |
| response.done | 根据 completed/cancelled/failed/incomplete 结束回答 |
| conversation.item.truncated / input_audio_buffer.cleared | 确认截断/清空；失败不得继续沿用不一致历史 |
| error / 连接关闭 | 脱敏结构化错误、停止采集与播放、关闭会话 |

事件名与完成状态以[服务端参考](https://developers.openai.com/api/reference/resources/realtime/server-events)为准。未知服务端事件可忽略；未知媒体格式、缺失关联 ID、超限或状态顺序错误必须失败关闭。error 的 type/code/message/param/event_id 只在内存解析，不能将服务原文或原始载荷直接透传或写日志。

打断先停止本机播放、清空待播放帧，记录该 item 实际已播放的 `audio_end_ms`；网关对仍在生成的 response 发送 response.cancel，并对已输出音频 item 发送 conversation.item.truncate（含 content_index）。VAD 已取消时不再次取消；取消竞态返回“无活动回答”不视为新回答。禁止使用仅 WebRTC/SIP 可用的 output_audio_buffer.clear。服务协议中的静音不会自动停止回答；ShiJu v1 为保证切代隔离，在静音时也主动取消旧回答、停止播放并清空未提交输入（产品策略）；结束则停止全部媒体、取消生成、清空未提交输入、关闭上游 WebSocket，不提交残留语音。[打断](https://developers.openai.com/api/docs/guides/realtime-conversations#handling-interruptions)、[取消](https://developers.openai.com/api/reference/resources/realtime/client-events#response.cancel)

```jsonl
{"type":"response.cancel","event_id":"cancel1","response_id":"r1"}
{"type":"conversation.item.truncate","event_id":"trunc1","item_id":"assistant1","content_index":0,"audio_end_ms":400}
{"type":"conversation.item.truncated","event_id":"e11","item_id":"assistant1","content_index":0,"audio_end_ms":400}
{"type":"response.done","event_id":"e12","response":{"id":"r1","status":"cancelled","output":[]}}
{"type":"input_audio_buffer.clear","event_id":"clear1"}
{"type":"input_audio_buffer.cleared","event_id":"e13"}
```

## 桌面到网关：身份、端点与 DTO

以下路径和限额均为 **ShiJu v1 设计决定**，不是 OpenAI API。路由纳入现有 `/api` 鉴权与 TenantScope 提取，使用 GatewayEndpoint 的 skey、platform/company 上下文头，WebSocket 在升级前执行同样校验；语音路由必须有可验证的用户身份，网关未配置鉴权时拒绝语音请求，不因监听回环地址而豁免；凭据不得放查询参数。远程连接使用 HTTPS/WSS。启动时冻结 `(endpoint, tenant, authenticated_principal, chat_session_id, voice_session_id)`，每次请求和帧都核对所有权，不能只凭客户端传来的 tenant/session ID 授权。换网关、租户或聊天会话时取消原会话；重连必须重新启动，不续用旧流。

| 方法与路径 | 请求 → 响应 |
| --- | --- |
| POST `/api/voice/transcriptions` | multipart：`metadata` 为 TranscriptionRequest JSON，`file` 为 WAV → 200 TranscriptionResponse |
| DELETE `/api/voice/transcriptions/{request_id}` | 同所有者请求取消 → 204；不存在/已取消也为 204，不泄漏其他租户请求 |
| POST `/api/voice/sessions` | VoiceSessionStart → 201 VoiceSessionStarted；上游配置确认后才返回 |
| GET `/api/voice/sessions/{voice_session_id}/stream` | 一次性附着的鉴权 WebSocket 升级 → 101；无附着票据或服务密钥返回给桌面 |
| DELETE `/api/voice/sessions/{voice_session_id}` | 幂等停止 → 204；关闭上游并释放资源后确认；超时返回结构化错误 |

模型、音色、语言、词典来自启动时网关配置快照；请求只携带 provider 配置引用 `provider_id`，不接受服务 URL、密钥或任意上游 session.update。设备列表、设备 ID、权限、热键、录音路径不在网关 DTO 内。配置仍通过已有配置系统管理，不新增音色目录路由。

通用类型：所有 JSON 字段 snake_case；ID 为 1–128 字节 ASCII 不透明字符串；generation/seq 为非负整数（不超过 2^53−1），时间单位 ms。请求拒绝未知字段和枚举值。generation 在每个本机操作/语音会话从 1 开始；只能由当前所有者或该会话的网关 VAD 处理推进，禁止回退、溢出。API 解析时校验实际数据大小再分配。

| DTO | 必填字段与含义 |
| --- | --- |
| TranscriptionRequest | `request_id, chat_session_id, generation, provider_id`；request_id 为本次唯一操作 ID，绑定录音起始草稿 |
| TranscriptionResponse | `request_id, chat_session_id, generation, text, detected_languages`；最终全文，detected_languages 为代码字符串数组，允许空 |
| VoiceSessionStart | `request_id, chat_session_id, generation, provider_id, turn_mode`；turn_mode 为 manual/server_vad |
| VoiceSessionStarted | 回显 request_id/chat_session_id/generation，加 `voice_session_id, stream_path, format, voice, expires_in_ms`；format 固定 `{encoding:"pcm_s16le",sample_rate:24000,channels:1}`；不返回上游 session ID |
| VoiceFrame | `type, voice_session_id, generation, seq, payload`；连接已绑定 tenant/chat，不允许帧重新绑定；seq 双向各从 1 严格递增 |
| VoiceError | `code, message, retryable, request_id`；可选 voice_session_id/generation；message 为本方固定安全描述 |

HTTP 示例（认证和租户上下文由现有客户端注入；metadata 与 file 采用 multipart，文件边界同上）：

```json
{"request_id":"dict_demo","chat_session_id":"chat_demo","generation":1,"provider_id":"voice_primary"}
```

```json
{"request_id":"dict_demo","chat_session_id":"chat_demo","generation":1,"text":"打开设置。","detected_languages":["zh"]}
```

```json
{"request_id":"start_demo","chat_session_id":"chat_demo","generation":1,"provider_id":"voice_primary","turn_mode":"manual"}
```

```json
{"request_id":"start_demo","chat_session_id":"chat_demo","generation":1,"voice_session_id":"voice_demo","stream_path":"/api/voice/sessions/voice_demo/stream","format":{"encoding":"pcm_s16le","sample_rate":24000,"channels":1},"voice":"marin","expires_in_ms":1800000}
```

WebSocket 只传文本 JSON，不接受二进制帧；每条文本是一条完整 VoiceFrame，不使用换行切包。payload 按 type 强类型解析：

| 方向/type | payload |
| --- | --- |
| C→G `audio.append` | `{audio, samples_per_channel}`；Base64 原始 PCM，长度必须等于 samples_per_channel×2 |
| C→G `input.commit` | `{}`；仅 manual，提交一轮且触发一次回答 |
| C→G `input.mute` | `{muted}`；关闭/开启采集，推进 generation 并清空未提交输入 |
| C→G `response.interrupt` | `{response_id, item_id, content_index, audio_end_ms}`；尚无输出时 item_id/content_index/audio_end_ms 为 null |
| C→G `playback.progress` | `{response_id, item_id, content_index, audio_end_ms}`；每 100 ms 及播放结束报告实际已播放位置，单调且不超过已下发时长 |
| C→G `session.stop` | `{}`；所有者结束，等价 DELETE |
| G→C `session.ready` | `{format, voice}`；绑定流与配置已就绪 |
| G→C `generation.changed` | `{reason, response_id, item_id, content_index}`；reason 为 interrupt/mute/vad；三个关联字段可为 null，确认新代已生效 |
| G→C `audio.delta` | `{response_id, item_id, content_index, audio, samples_per_channel}`；网关按受限帧重分块 |
| G→C `audio.done` | `{response_id, item_id, content_index}`；不代表扬声器已播放完 |
| G→C `transcript.delta` | `{role, item_id, content_index, delta}`；role 为 user/assistant |
| G→C `transcript.final` | `{role, item_id, content_index, text}`；全文替换，幂等 |
| G→C `response.started` / `response.done` | `{response_id}` / `{response_id, status}`；status 为 completed/cancelled/failed/incomplete |
| G→C `handoff.status` | `{call_id, status, text}`；status 为 accepted/awaiting_approval/running/completed/failed/cancelled，text 为给用户的状态或最终答复 |
| G→C `error` | VoiceError |
| G→C `session.closed` | `{outcome, reason}`；outcome 为 ended/failed，reason 使用固定代码 |

示例帧可直接按以上类型解析（seq 在各方向独立计数）：

```jsonl
{"type":"session.ready","voice_session_id":"voice_demo","generation":1,"seq":1,"payload":{"format":{"encoding":"pcm_s16le","sample_rate":24000,"channels":1},"voice":"marin"}}
{"type":"audio.append","voice_session_id":"voice_demo","generation":1,"seq":1,"payload":{"audio":"AAAAAA==","samples_per_channel":2}}
{"type":"input.commit","voice_session_id":"voice_demo","generation":1,"seq":2,"payload":{}}
{"type":"transcript.final","voice_session_id":"voice_demo","generation":1,"seq":2,"payload":{"role":"user","item_id":"user1","content_index":0,"text":"你好。"}}
{"type":"session.stop","voice_session_id":"voice_demo","generation":1,"seq":3,"payload":{}}
{"type":"session.closed","voice_session_id":"voice_demo","generation":1,"seq":3,"payload":{"outcome":"ended","reason":"user_stop"}}
```

## generation、取消、错误与资源边界

以下全部为 ShiJu 自定策略。取消必须抢占媒体，不能排在音频队列尾部。

- 本地打断/静音先原子推进 generation，立即清空设备及应用采集/播放队列。控制帧带当前代加 1，网关仅接受下一代，停止旧 generation 后回 generation.changed；收到确认前本机不发送新代音频。VAD 打断由网关先推进并发送 generation.changed（payload 额外含被打断的 response_id/item_id/content_index，可为 null），客户端收到即同步，并用新代 response.interrupt 回报停止时的实际已播放位置；网关先取消旧回答，待回报后截断历史，1 秒内未回报则关闭，不拿最近 progress 伪装精确播放位置。VAD 后的 response.interrupt 是当前代截断位置回报，不再推进代；双方同时提议同一新代视作同一次失效，重复控制帧不重复执行。旧代媒体/字幕/工具结果直接丢弃；未来代普通媒体是协议错误。上游本身没有 generation 字段，必须按 response/item ID 的创建代映射，绝不能把迟到事件标成当前代。
- generation 同时隔离输入与输出；本地控制切代清空尚未提交输入、未完成字幕缓存；VAD 切代保留触发 speech_started 的当前输入，将该事件的 item_id 预绑定新代，已在服务输入缓冲区内的新语音不发送 clear（否则会丢失用户话头），其后 committed 与字幕继承此代；已确认的最终字幕可保留为历史。字幕以 `(voice_session_id,generation,role,item_id,content_index)` 合并，event_id 去重；final 替换且只应用一次。输入字幕只是显示，不调用代理。
- 听写取消通过 request_id 终止上游请求；上传断开也取消，服务可能已产生费用。取消/切换草稿后即使成功响应迟到也不可回填。取消后用户重试必须新 request_id/generation，不自动重放。
- 启动/转写 request_id 在所有者范围内去重；同 ID 同内容复用进行中结果，不再请求上游；同 ID 不同内容返回 conflict。活动请求与终态去重记录仅内存保留，会话结束后最多 5 分钟，每所有者最多 128 条；满时拒绝新请求，不提前淘汰活动记录。不保证进程重启后的幂等，不自动恢复或重放。

| 边界 | v1 限额与超出行为 |
| --- | --- |
| HTTP | WAV ≤24,000,000 字节、180 秒；multipart 总体 ≤24,100,000 字节，metadata ≤8 KiB；超出 413；非法媒体 415 |
| WebSocket | 完整解压后消息 ≤64 KiB；禁用消息压缩；媒体每帧 ≤4,800 字节/2,400 采样/100 ms，常规 20 ms；极短尾帧允许；≤100 个输入音频帧/秒，按实际采样总时长限速至实时速率（允许 100 ms 突发），超出关闭 |
| 媒体队列 | 每连接、每方向同时 ≤50 帧且 ≤1 秒 PCM；播放/转发独立受限，超出清空并报 backpressure，关闭会话；不能无限增长或悄悄漏音 |
| 控制队列 | 独立容量 16，停止/失效信号另用可抢占取消通道；耗尽则失败关闭 |
| 文本与工具 | 每 item 最终字幕/累积增量 ≤64 KiB；函数参数 ≤8 KiB，instruction ≤4,000 个标量；结果文本 ≤16 KiB；去重 ID/关联 item 每会话 ≤4,096，超出结束会话 |
| 时长与并发 | 单轮输入 ≤180 秒（到限停止并提示，不隐式提交）；语音会话 ≤30 分钟；同一所有者最多 1 个活动语音会话与 1 个转写请求；超出 409 |
| 超时 | 上游连接/配置确认各 10 秒；启动成功后 10 秒未附着则回收；文件转写含上传共 60 秒；提交后 30 秒无回答进展即失败；关闭清理 5 秒后强制释放 |
| 心跳 | WebSocket 每 15 秒 ping、10 秒未 pong 关闭；工具等待审批独立于音频回答超时，但仍受总会话 30 分钟限制 |

上游 audio.delta 可能大于本方单帧，适配器先在 1 MiB 消息上限内解析、验证 Base64 与偶数字节长度，再按 100 ms 分块；不能一次为全部分块分配无界队列。OpenAI input_audio_buffer.append 的公开上限为 15 MiB，远大于本方帧预算，不可拿它作为客户端队列大小。[上游上限](https://developers.openai.com/api/reference/resources/realtime/client-events#input_audio_buffer.append)

错误 HTTP 体统一 `{"error":VoiceError}`，WebSocket 用 error 帧后必要时 session.closed。code 集合：`invalid_request, unauthorized, forbidden, not_found, conflict, unsupported_media, payload_too_large, unsupported_capability, provider_auth, rate_limited, provider_unavailable, timeout, cancelled, protocol_error, backpressure`。分别映射 400/401/403/404/409/415/413/422/502/429/502/504/409/502/429。retryable 仅 rate_limited/provider_unavailable/timeout 为 true，表示允许用户重新发起，不代表自动重放。已关闭连接无法发送终态时以连接关闭判失败；正常 stop 使用关闭码 1000，非法协议 1008，超大 1009，内部失败 1011。上游错误使用安全固定描述，例如：

```json
{"error":{"code":"provider_auth","message":"语音服务认证失败，请检查网关服务配置。","retryable":false,"request_id":"dict_demo"}}
```

## 工具交接与隐私边界

只公开一个函数 `shiju_handoff`，参数 schema 见 session.update。它把用户工作交给启动时已绑定的 ShiJu 聊天会话，不授予模型 shell、文件或审批能力。必须等待完整 function_call item（status=completed）、完整 arguments 且对应 response 正常完成，再严格解析，不能仅因 arguments.done 就执行，因为取消也会发 done。[函数事件](https://developers.openai.com/api/reference/resources/realtime/server-events#response.function_call_arguments.done)

以 `(tenant, principal, voice_session_id, generation, call_id)` 原子去重并绑定 provider response/item；同 ID 不同参数报 protocol_error。最多一个在途交接，使用现有 session/shared.rs、session/stream.rs 的执行与审批链，拒绝跨 tenant/session 的参数。重复事件只回既有状态，不再次运行。取消 generation 或结束会话时取消待执行/待审批工作并请求取消运行中的工作；已发生副作用不承诺回滚，迟到结果不得回到新代。去重容量与会话生命周期同上。

将审批状态和最终结果投影到 handoff.status；最终结果通过公开 `conversation.item.create` 的 function_call_output 返回，再发送一次 response.create，让实时服务说出结果。不把中间字幕增量变成工作请求，不输出隐藏推理。失败/拒绝也返回明确结果。下面先展示完整函数调用项的服务事件投影，再展示返回结果；实际须同时等待 response.done(completed)。示例中的 output 是 JSON 字符串：[工具结果协议](https://developers.openai.com/api/reference/resources/realtime/client-events#conversation.item.create)

```jsonl
{"type":"response.function_call_arguments.done","event_id":"tool_args1","response_id":"r_tool","item_id":"tool_item1","output_index":0,"call_id":"call_demo","name":"shiju_handoff","arguments":"{\"instruction\":\"整理当前项目待办\"}"}
{"type":"response.output_item.done","event_id":"tool_item_done1","response_id":"r_tool","output_index":0,"item":{"type":"function_call","id":"tool_item1","status":"completed","call_id":"call_demo","name":"shiju_handoff","arguments":"{\"instruction\":\"整理当前项目待办\"}"}}
{"type":"response.done","event_id":"tool_response_done1","response":{"id":"r_tool","status":"completed","output":[]}}
{"type":"conversation.item.create","event_id":"tool_result1","item":{"type":"function_call_output","call_id":"call_demo","output":"{\"status\":\"completed\",\"text\":\"已完成用户请求。\"}"}}
{"type":"response.create","event_id":"tool_reply1"}
```

本机设备只由客户端枚举和操作；远端网关不枚举设备、不取本机屏幕。本机麦克风须用户主动启用并完成系统授权；屏幕上下文默认关闭，只有显式开启、可见分享状态与系统授权齐备后才能采集。v1 音频流协议不接受屏幕帧，后续屏幕任务须定义独立受限消息。录音留存默认关闭；开启后最多最近 20 段、仅本机保存。tracing 不记录原始音频、字幕、词典、工具参数/结果、截图、鉴权头或服务错误原文，只记录安全错误代码、耗时与计数。内存载荷随取消/关闭释放，不承诺服务端保留政策等同于本机关闭留存。

## 本地核对入口与实现边界

- ShiJu：`crates/vw-agent/src/gateway/api/handlers/desktop.rs` 已有偏好路由与 TenantScope；`crates/vw-gateway-client/src/http.rs::apply_request_context` 注入既有认证/租户头；`crates/vw-gateway-client/src/client/desktop_settings_api.rs` 是客户端接入模式；`crates/vw-desktop/src/app/config_desktop.rs` 按 endpoint/tenant 管理偏好。本文不修改这些在途代码。
- Codex 参考以外部 checkout 相对路径记录（本次可读，无 Git 提交身份）：`codex-rs/voice-host/README.md` 明示 private foundation，mute/generation/有界队列可参考，但 receive/decode、TUI 集成与真实媒体证明未完成；不能当作可交付音频后端。
- `codex-rs/tui/src/app/realtime_settings.rs` 分开本机设备和服务音色；其 thread/listVoices 是内部 app-server 协议，公开服务没有在本次来源中建立同形目录接口。
- `codex-rs/codex-api/src/endpoint/realtime_call.rs` 同时含公开 multipart SDP 与 backend-api JSON、quicksilver/avas、Live 分支。ShiJu 只按官方 Realtime WebSocket 建连，不复制这些私有请求或借用 Codex 凭据。
- `codex-rs/app-server-protocol/src/protocol/v2/realtime.rs` 将音频、字幕、工作和关闭分开，可借鉴事件职责；实验参数、Cove 枚举和私有交接不构成公开能力依据。

本任务仅文档核对，不需要凭据/实机，也未声称真实 API、麦克风、扬声器或 macOS 插入成功。后续任务须验证 DTO 解析、租户隔离、迟到帧、取消、审批去重和真实服务/设备；服务端若拒绝本快照中的模型或字段，应返回明确错误并更新能力证据，不做私有接口回退。
