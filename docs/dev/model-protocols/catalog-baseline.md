# 全量目录核查基线

核查日期：2026-11-11。证据类型：本地静态目录与源码审查；未进行官方查询或真实调用。

供应商身份 231；供应商来源记录 231；模型来源记录 8949。
来源身份为 `(source, provider_id, model_key)`，JSON Pointer 定位原始条目；API ID、canonical ID 独立保留。相同 canonical ID 不合并。

## 来源与分组

| 来源 | SHA-256 | 供应商 | 模型 |
| --- | --- | ---: | ---: |
| crates/vw-provider-resolver/assets/model/api.json | `347b23758b2122f2fbfa723319a8670e59a7f164862d48177effefc1cc8dd272` | 230 | 8909 |
| crates/vw-provider-resolver/assets/model/shiju.json | `19d231eb5580c1a8af5176694a74ab7f68ccbd42d9c40bbf496ff017bc5ca7dd` | 1 | 40 |

新增供应商：无；删除供应商：无。
源变化：无，哈希和计数匹配初始目录基线。
新增供应商按供应商数量最少的组分配，同量取较小组号；原有分配不变，空模型供应商也参与完整性核对。

| 组 | 后续任务 | 供应商 | 模型来源记录 |
| ---: | ---: | ---: | ---: |
| 1 | 2 | 39 | 1399 |
| 2 | 3 | 39 | 1537 |
| 3 | 4 | 39 | 1775 |
| 4 | 5 | 39 | 2155 |
| 5 | 6 | 39 | 1035 |
| 6 | 7 | 36 | 1048 |

## 声明与缺口

模型 provider 覆盖 840 条；字段计数：`{'api': 618, 'npm': 508, 'shape': 103}`。
适用性初筛：`{'no_text_output': 232, 'specialized_transport_review': 5, 'text_candidate_requires_protocol_review': 8712}`。text 模态只表示候选；嵌入、实时、专用生成、停用状态和工具能力仍须逐条查询。
effective_declarations 仅按字段应用模型覆盖，不推断 npm 与 adapter 等价、不推断原生端点、不宣称运行时已支持。

供应商未声明 API：25 家（必须查 SDK 默认值或部署参数，禁止猜测 URL）：

| 供应商 | SDK/adapter 声明 | 文档种子（尚未查询） |
| --- | --- | --- |
| aihubmix | @aihubmix/ai-sdk-provider | https://docs.aihubmix.com |
| amazon-bedrock | @ai-sdk/amazon-bedrock | https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html |
| anthropic | @ai-sdk/anthropic | https://docs.anthropic.com/en/docs/about-claude/models |
| azure | @ai-sdk/azure | https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models |
| azure-cognitive-services | @ai-sdk/azure | https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models |
| cerebras | @ai-sdk/cerebras | https://inference-docs.cerebras.ai/models/overview |
| cloudflare-ai-gateway | ai-gateway-provider | https://developers.cloudflare.com/ai-gateway/ |
| cohere | @ai-sdk/cohere | https://docs.cohere.com/docs/models |
| deepinfra | @ai-sdk/deepinfra | https://deepinfra.com/models |
| gitlab | gitlab-ai-provider | https://docs.gitlab.com/user/duo_agent_platform/ |
| google | @ai-sdk/google | https://ai.google.dev/gemini-api/docs/models |
| google-vertex | @ai-sdk/google-vertex | https://cloud.google.com/vertex-ai/generative-ai/docs/models |
| google-vertex-anthropic | @ai-sdk/google-vertex/anthropic | https://cloud.google.com/vertex-ai/generative-ai/docs/partner-models/claude |
| groq | @ai-sdk/groq | https://console.groq.com/docs/models |
| mistral | @ai-sdk/mistral | https://docs.mistral.ai/getting-started/models/ |
| openai | @ai-sdk/openai | https://platform.openai.com/docs/models |
| perplexity | @ai-sdk/perplexity | https://docs.perplexity.ai |
| qvac | @qvac/ai-sdk-provider | https://www.npmjs.com/package/@qvac/ai-sdk-provider |
| salad-cloud | @saladtechnologies-oss/ai-sdk-provider | https://docs.salad.com/ai-gateway/explanation/overview |
| sap-ai-core | @jerome-benoit/sap-ai-provider-v2 | https://help.sap.com/docs/sap-ai-core |
| togetherai | @ai-sdk/togetherai | https://docs.together.ai/docs/serverless-models |
| venice | venice-ai-sdk-provider | https://docs.venice.ai |
| vercel | @ai-sdk/gateway | https://github.com/vercel/ai/tree/5eb85cc45a259553501f535b8ac79a77d0e79223/packages/gateway |
| watsonx | watsonx-ai-provider | https://www.ibm.com/docs/en/watsonx/saas?topic=solutions-supported-foundation-models |
| xai | @ai-sdk/xai | https://docs.x.ai/docs/models |

## 已审查源码与交付依赖

- `crates/vw-shared/src/provider/models.rs`：Provider 保存 adapter、不保存 npm；ModelProviderInfo 仅保存 adapter，模型级 api/npm/shape 丢失。
- `crates/vw-shared/src/provider/state.rs::from_models_dev_model`：模型 adapter 优先，API 使用供应商地址；缺省 adapter 会进入默认兼容路径。声明盘点不能据此证明上游兼容。
- `crates/vw-provider-resolver/src/models.rs::parse_models_text/merge_bundled_shiju`：直接反序列化；只用 shiju.json 中的 shiju 整体替换，非 shiju 补充不会装载。本清单保留被遮蔽或未装载的来源行。
- 开始时已有未提交依赖：`crates/vw-agent/src/model_relay/{mod.rs,server.rs,transport.rs,generation.rs}`、`crates/vw-agent-apps/src/aggregation/{mod.rs,model_ids_tests.rs}`、`crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/{mod.rs,reachability.rs,model_check.rs,model_check/}`。这些文件属于共享传输、聚合和诊断工作；本任务没有修改、提交或验证它们，后续实现任务需核对实际交付。

## 可重现命令与验证边界

```bash
python3 -m unittest discover -s scripts -p "test_audit_model_protocols.py"
python3 scripts/audit_model_protocols.py inventory --api crates/vw-provider-resolver/assets/model/api.json --shiju crates/vw-provider-resolver/assets/model/shiju.json --output docs/dev/model-protocols
python3 scripts/audit_model_protocols.py check-research --inventory docs/dev/model-protocols/catalog-inventory.json --report docs/dev/model-protocols/research/group-01.json --group 1
```

首次生成以计划目录基线为种子；后续生成默认复用输出 research-groups.json 的 baseline_groups/baseline_sources，可用 --baseline 显式指定种子。输入目录只读，输出可重复生成；源变化会记录增删和哈希差异。
inventory 子命令核对来源行唯一性及各组互斥完整性，任何遗漏或冲突均非零退出。check-research 的成功仅证明合同和覆盖，不证明官方资料内容或服务实测正确。

## 研究 JSON 合同 v1

报告根字段：schema_version=1、group（整数）、source_hashes（全部来源路径→SHA-256）、providers（本组完整供应商 ID 列表）、evidence、records。
evidence 每项：id（唯一）、type（official_docs/official_sdk/official_query/real_call/offline）、url（查询或来源 HTTP URL）、checked_at（ISO 日期）、excerpt（证据摘录或查询未果说明）。不保存密钥或敏感原始响应。
records 每项必须逐条对应来源身份 source/provider_id/model_key，并保留 model_api_id、canonical_model_id（无值填 null）；不能只填 canonical ID 或一个供应商总论。
每行还需 conclusion、reason、evidence_ids、request_protocols（chat/messages/responses/gemini）、endpoints（已确认端点字符串数组）、auth（认证方式）、deployment（地域/套餐/必要参数）、limitations（字符串数组）。
capabilities 含 text/tools/thinking/media/streaming/structured_output 六个非空字符串评估；thinking 应涵盖关闭/强制/回放，streaming 应区分 JSON/SSE/终态；未知项明确写 unknown 并解释。
conclusion 仅允许：官方确认、实测通过、明确不支持、资料不足、不适用于文本ACP、缺少凭据、实现缺口。资料不足也必须提供查询 URL、日期、摘录/未果说明及 reason；端点和协议未知可填空数组。
官方确认需 official_docs 或 official_sdk 证据以及非空协议、端点；实测通过需 real_call 证据和 live_verification 的 http_status/binding/events/final_state。401/403、限流、超时和未知模型应在 reason 分别记录，不能据单次失败断言永久不支持。
报告严格核对分组、全部源哈希、来源身份、API ID、canonical ID、重复行、漏行及证据引用；不访问网络，不代替官方或实时验收。
