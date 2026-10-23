# 识矩目录同步记录

同步日期：2026-10-23。模型 ID 来自 https://router.shiju.cloud/api/pricing 的 35 个公开条目；公开目录不保证每个账户均有权限。

能力按维护者要求对照同目录 `api.json`：优先百炼精确 ID，开源 Qwen 使用 DeepInfra 同款条目。保留识矩请求 ID，不复制上游 URL、价格、请求头或状态。推理档位是目录推断，尚未通过识矩在线请求逐款验证；不能视为网关能力声明。

没有精确条目的专用模型使用相近任务的基础元数据；其限制和模态仍待实际接口核对。Kimi K2.8 预览版不继承其他版本的强度档位。空 `reasoning_options` 表示未声明可选档位，预算/开关不转换成强度。后续拿到渠道配置时以实际配置覆盖这些推断。

| 识矩 ID | 元数据来源 | 匹配方式 |
| --- | --- | --- |
| `cosyvoice-v2` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `deepseek-v4-pro-0813` | `alibaba-token-plan-cn/deepseek-v4-pro-0813` | 精确 ID |
| `deepseek-v4.1-flash` | `alibaba-cn/deepseek-v4.1-flash` | 精确 ID |
| `deepseek-v4.1-flash-1017` | `alibaba-token-plan-cn/deepseek-v4.1-flash-1017` | 精确 ID |
| `fun-asr` | `alibaba-cn/qwen3-asr-flash` | 同任务近似能力/限制，待核对 |
| `glm-5.3` | `alibaba-cn/glm-5.3` | 精确 ID |
| `gte-rerank-v2` | `旧 shiju.json` | 保留原有专用能力 |
| `kimi-k3` | `alibaba-cn/kimi-k3` | 精确 ID |
| `kimi/kimi-k2.8-preview` | `alibaba-cn/kimi-k2.6` | 近似基础能力；预览版档位待核对 |
| `paraformer-v2` | `alibaba-cn/qwen3-asr-flash` | 同任务近似能力/限制，待核对 |
| `qwen-audio-3.1-asr-flash-streaming` | `alibaba-cn/qwen3-asr-flash` | 同任务近似能力/限制，待核对 |
| `qwen-audio-3.1-tts-next` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `qwen-image-2.0` | `alibaba-token-plan-cn/qwen-image-2.0` | 精确 ID |
| `qwen-image-2.0-pro` | `alibaba-token-plan-cn/qwen-image-2.0-pro` | 精确 ID |
| `qwen-image-3.0` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `qwen-mt-plus` | `alibaba-cn/qwen-mt-plus` | 精确 ID |
| `qwen-vl-ocr` | `alibaba-cn/qwen-vl-ocr` | 精确 ID |
| `qwen3-rerank` | `旧 shiju.json` | 参考 gte-rerank-v2 专用能力，限制待核对 |
| `qwen3-vl-plus` | `alibaba-cn/qwen3-vl-plus` | 精确 ID |
| `qwen3.7-plus` | `alibaba-cn/qwen3.7-plus` | 精确 ID |
| `qwen3.8-2.4t-a95b` | `deepinfra/Qwen/Qwen3.8-2.4T-A95B` | 同款大小写/厂商前缀 |
| `qwen3.8-27b` | `deepinfra/Qwen/Qwen3.8-27B` | 同款大小写/厂商前缀 |
| `qwen3.8-flash` | `alibaba-cn/qwen3.8-flash` | 精确 ID |
| `qwen3.8-max` | `alibaba-cn/qwen3.8-max` | 精确 ID |
| `qwen3.8-omni-flash` | `alibaba-cn/qwen3.8-omni-flash` | 精确 ID |
| `text-embedding-v3` | `旧 shiju.json` | 保留原有专用能力 |
| `text-embedding-v4` | `旧 shiju.json` | 保留原有专用能力 |
| `voice-enrollment` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `wan2.2-animate-mix` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `wan2.2-s2v` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `wan2.2-s2v-detect` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `wan2.7-image` | `alibaba-token-plan-cn/wan2.7-image` | 精确 ID |
| `wan2.7-image-pro` | `alibaba-token-plan-cn/wan2.7-image-pro` | 精确 ID |
| `wan3.0-video` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
| `wan3.0-video-prime` | `alibaba-token-plan-cn/qwen-image-2.0` | 专用模态；限制参考图像模型，待核对 |
