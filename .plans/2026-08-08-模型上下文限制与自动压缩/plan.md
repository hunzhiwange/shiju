# 模型上下文限制与自动压缩计划

> 目标：补齐模型限制与能力元数据，支持自定义兼容模型配置，并在有效输入窗口达到 90% 时自动压缩上下文。

## 范围

- 补齐 `shiju.json` 与 `api.json` 中模型的上下文、输入/输出限制和能力字段。
- 以厂商官方资料为首选来源，以 `models.dev` 为大规模目录基线；没有可靠资料时使用明确的保守默认值。
- 让自定义 OpenAI 兼容 provider 的添加、编辑模型界面可配置 `limit` 与 `shiju.json` 同名的能力字段。
- 统一桌面端 provider resolver 与 agent 运行时的模型元数据解析，确保界面显示、上下文治理和实际请求使用同一套限制。
- 在活跃会话的下一次 LLM 请求前，按当前输入使用率自动执行上下文治理/压缩。

## 不在范围内

- 不改变 provider 鉴权、请求协议、模型发现接口或模型 ID 匹配规则。
- 不把模型上下文限制与之前的 `/ui` 请求体 413 问题混为同一层；会保留现有 UI 快照修复。
- 不在运行时为未知模型发起额外的远程探测请求，不记录 API Key、原始请求体或敏感模型载荷。

## 已核对的现状

- `shiju.json` 有 34 个模型，仅 6 个已有完整限制；多数条目目前只有 ID/名称。
- `api.json` 约 180 个 provider、数千模型；多数条目已有 `limit`，但音频、图像、视频和部分聚合模型仍有缺失项。
- 原始模型结构已经包含 `limit.context/input/output`、`reasoning`、`tool_call`、`temperature`、`attachment`、`modalities`、`interleaved` 等字段。
- 自定义模型草稿目前只保存 `model_id` 与 `display_name`；解析未知模型时限制为 0，能力使用固定默认值。
- `crates/vw-agent/src/session/compaction.rs` 有 70% 阈值的孤立实现，活跃的 `session/processor/runner.rs` 实际使用的是 `context_governance`，没有自动压缩调用点。

## 方案决策

1. 限制语义保持现有 JSON 结构：`context` 是总上下文窗口，`input` 是厂商明确给出的最大输入，`output` 是最大输出。缺省模型不再以 `context=0` 表示未知。
2. 未配置或无法可靠核实的模型使用集中定义的保守默认值：`context=32768`、`input=28672`、`output=4096`。默认值必须可被自定义模型显式覆盖，并在设置界面提示“使用默认值”。
3. 已有官方值优先于 `models.dev` 值；`models.dev` 只用于补全大目录和交叉核对。纯图像、音频、视频、嵌入或重排模型仍填写可确认的输入限制与能力；输出为非文本或厂商不适用时保留结构语义，不把“不适用”伪装成文本生成能力。
4. 自定义模型配置字段沿用资产命名：`name`、`family`、`reasoning`、`tool_call`、`temperature`、`attachment`、`modalities.input/output`、`limit.context/input/output`、`interleaved`。未填写的能力字段保持现有兼容默认，显式填写的值覆盖基线。
5. 自动压缩使用有效输入窗口：优先 `limit.input`，否则使用 `limit.context - 预留输出空间`；默认模型也走同一规则。当前估计/已观测输入达到或超过 90% 时，在发起下一次 LLM 请求前压缩，避免先请求再收到 length limit exceeded。
6. 压缩优先复用活跃处理器已有的工具结果清理和历史裁剪，保持系统消息、最近对话和工具协议配对；压缩后仍无法低于阈值时返回可诊断错误，不无限重试。孤立的旧 `compaction::is_overflow` 要么改为共用纯判断，要么删除重复阈值，不能继续保留 70% 与 90% 两套运行时语义。

## 外部资料基线

- 阿里云百炼 Coding Plan FAQ：<https://help.aliyun.com/zh/model-studio/coding-plan-faq>
- 阿里云百炼文本生成模型：<https://help.aliyun.com/zh/model-studio/text-generation-model>
- 阿里云百炼视觉模型：<https://help.aliyun.com/zh/model-studio/vision-model/>
- `models.dev` 模型目录：<https://models.dev/>；本地 `api.json` 是该目录的内置基线。

资料核对必须记录模型 ID、上下文、最大输入、最大输出、能力字段、来源 URL 和核对日期；找不到权威值时记录“使用默认值”的原因，不能臆造精确厂商参数。

## 任务顺序

1. [模型目录补齐与限制兜底](1-模型目录补齐与限制兜底.md)（批次 1）
2. [自定义兼容模型能力配置](2-自定义兼容模型能力配置.md)（批次 2，依赖任务 1）
3. [90% 上下文自动压缩](3-90百分比上下文自动压缩.md)（批次 3，依赖任务 1、2）
4. [联调验证与数据审计](4-联调验证与数据审计.md)（批次 4，依赖任务 1、2、3）

## 全局完成标准

- 两个模型资产都能被现有解析器读取；可调用模型不再因缺失限制而得到 `context=0`。
- 自定义模型可保存、重新打开并完整恢复限制与能力字段；未知字段仍安全回退。
- 89% 不触发，达到/超过 90% 在下一次请求前触发；压缩后保留系统指令与协议完整性。
- 相关测试先失败后通过；最终执行 `cargo clippy --all-targets --all-features -- -D warnings`，不扩大为无关全量测试或格式化改动。
