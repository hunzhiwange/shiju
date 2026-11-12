# 全量模型协议核查验收

核查日期：2026-11-12（Asia/Shanghai）。状态：已完成；最终回归已闭环，任务23删除。本轮补齐生产relay的9个缺失转换方向，保留原生优先、精确模型/部署/账号授权与能力拒绝；未修改两份源目录。任务22基线为提交 `17daf9535`，本轮重新构建网关并重跑受影响服务与真实客户端，历史成功不冒充新实测。

## 最终统一回归（2026-11-12）

动态领取并使用槽位5；`CARGO_TARGET_DIR`与`CARGO_BUILD_BUILD_DIR`均为`/Users/xiangmin/worker/code/shiju/target/rust-slot-5`，`CARGO_INCREMENTAL=0`、`CARGO_BUILD_JOBS=1`。完整Agent测试以包级`opt-level=0/debug=0/codegen-units=16`执行，保留默认features；Clippy为全workspace全部targets/features。确切argv、退出码、内存采样摘要、二进制SHA及构建前后稳定的源码哈希见coverage.validation.final_regression。

|验证|退出码|有效结果|
|---|---:|---|
|共享`model_protocol_`|0|24项通过|
|解析器`model_protocol_`|0|23项通过；8949条/35796格精确匹配|
|当前Agent同一二进制的协议与relay两组|0|75+20=95项通过、0失败；1项要求安装CLI的用例按原标记忽略|
|`cargo fmt --all`|0|已格式化并审查修改范围|
|`cargo clippy --all-targets --all-features -- -D warnings`|0|最新共享工作区及修复后均通过|
|主coverage审计|0|8949条/35796格，无遗漏/重复|

Messages失败已查明为测试配置遗留：原生思考场景设置的`reasoning=true`随配置合并保留，返回Chat后进入必须等待终态的思考收据缓冲路径。取消场景现在明确恢复非思考fixture，并限制首帧等待；保留原60秒租期。另将500ms到期断言拆成独立测试，在流建立后推进本地Tokio时钟，验证令牌拒绝、上游取消与流结束；生产认证、租期和安全策略未修改。

SIGKILL未再复现：调查了48GiB主机上的并发大编译、磁盘与ulimit，以及明确未采取终止动作的系统磁盘写入诊断。根因不能从现有系统日志定论；单Cargo作业与低内存参数已成功完成完整Agent代码生成和行为验收。初次新构建的Cursor旧拒绝断言与工作区更新不一致，随后统一重建验证了当前支持清单。未将缓存二进制、编译成功或零测试作为通过证据。

本轮仅修改测试fixture；先前真实服务、29项安装客户端与12项显式none HTTP探针保留原时间、二进制与范围，账户权限/配额、CLI控制限制等未通过项继续按逐条条件复测。以下历史失败和上一轮状态保留作追溯，不代表最终回归仍未完成。

## 全量覆盖

[coverage.json](coverage.json)包含全部8949条来源记录、231家供应商、35796个协议格，主覆盖检查遗漏0、重复0。身份为 `(source, provider_id, model_key)`；每条保留官方报告路径/记录索引/证据ID、离线选择、service及ACP观测索引与未通过项完成条件，所有索引从0开始。离线、官方查询、受控HTTP回环和真实账号结果分别保留。

|源文件|供应商|模型|SHA-256|
|---|---:|---:|---|
|`crates/vw-provider-resolver/assets/model/api.json`|230|8909|`347b23758b2122f2fbfa723319a8670e59a7f164862d48177effefc1cc8dd272`|
|`crates/vw-provider-resolver/assets/model/shiju.json`|1|40|`19d231eb5580c1a8af5176694a74ab7f68ccbd42d9c40bbf496ff017bc5ca7dd`|

两源哈希与inventory一致；6组官方研究check-research通过，记录数依次1399、1537、1775、2155、1035、1048。研究结论：官方确认3720、资料不足4793、不适用于文本ACP416、明确不支持17、专用agent API实现缺口3。官方确认表示精确接口合同，不等于当前账号可用。账号未配置、访问被拒和资料不足不能互相替代。

`protocols.json`从研究投影为122个供应商条目、292个端点、6698个精确模型绑定和25项SDK默认入口审查。新增/修正Vercel与Cohere兼容入口、阿里Messages认证、OpenCode Gemini操作、OCI API-key模式与必需项目头、Privatemode本地代理及部分Claude兼容认证。OCI IAM签名/会话不纳入API-key路径；远端明文HTTP与未经确认的SDK默认入口不发布。SDK原始身份保持不变；OpenCode仅在原SDK未知且存在已授权精确HTTP合同的情况下选择其客户端wire，再由真实启动路由复验权限和完整能力。

## 原生与转换边界

离线基本文本格：原生6668、转换8100、拒绝21028；3630条同时有原生与转换，62条只有原生，5257条全部拒绝。拒绝原因为UnknownProtocols 19168、NotApplicableOrNotLoaded 1748、MissingDeploymentParameters 120；ConversionUnavailable为0。此前126条/378格普通文本转换缺口已修复。官方确认却全部拒绝的28条均为Cloudflare Workers AI端点缺真实account_id，逐条完成条件已列coverage，不猜测账号参数。

生产relay现有12个有向转换（四协议任意不同输入/目标）均登记真实实现。新增转入Messages、Responses、Gemini的9方向以局部Chat wire复用已有入站转换和响应格式器，新增原生请求编码、响应解码、原生SSE终态校验。支持有界文本、函数工具ID/参数/结果关联、流式和可表达的JSON Schema；工具参数完整验证后才发出，截断/取消/错误不发成功终态，流与非流响应总量限制4MiB。Gemini工具schema保留为parametersJsonSchema。

新增方向不声明Thinking/ThinkingReplay或媒体支持：只映射用户明确请求的none，缺省不擅自关闭；无法保存的思考签名、opaque状态、托管工具、未知字段、不可表达的严格/并行工具约束明确拒绝。原有转入Chat的思考收据与模型控制策略保持。12方向指本轮relay/ACP执行路径，不宣称所有AISDK工厂或所有供应商都支持全部能力；每个请求仍由共享selector按当前授权与能力判断，同协议原生优先，歧义拒绝。

真实生产HTTP回环新增12项：9个方向各覆盖文本JSON/SSE、工具调用/结果续轮、工具SSE及schema请求，另3项截断（57次HTTP交换）；实际使用ProviderRelayBackend及其上游HTTP，不以模拟转换函数代替生产连线。新增组件从可编译行为红灯到19项通过，工厂2项行为红灯到当前聚焦套件通过；详细前后记录在coverage.validation。

真实DeepSeek原生Messages单目标HTTP探针：Chat、Responses、Gemini三种输入显式请求none，各自完成文本JSON、SSE、函数调用及工具结果续轮，共12次均通过；上游12次HTTP200、请求模型均为deepseek-v4-pro、完整终态通过。证据见coverage.validation.native_inverse_live。默认思考探针按边界拒绝，未静默关闭。安装客户端的none控制拒绝仍保留为失败，不能把上述协议HTTP成功改写为ACP客户端成功。

## 专用接口适用性

原研究3项实现缺口完整保留，最终分类为 `specialized_agent_api_outside_standard_text_acp`，不宣称已实现，也不把专用任务伪装成普通Chat/Gemini模型：

|模型|官方合同与后续条件|
|---|---|
|`echo/echo`|[Echo run API](https://echo.tracerml.ai/docs/api)采用workspace Bearer、saved-run异步快照及独立动作权限；需专用生命周期、取消和授权适配。|
|`google/deep-research-max-preview-04-2026`|[Google Deep Research](https://aistudio.google.com/docs/deep-research)仅Interactions agent且要求后台执行，不能使用generate_content；需独立agent任务适配。|
|`google/deep-research-preview-04-2026`|同上；普通四协议文本工具回合合同不能表达其专用任务行为。|

## 当前账号与真实客户端

本轮正式产物：[service-matrix.json](service-matrix.json)、[acp-acceptance.json](acp-acceptance.json)。网关/relay二进制、源码差异与生产模块哈希绑定实测；共享工作区存在其它任务修改，因此不声称来自干净提交。真实工具链必须同时满足模型/账号/工作区绑定、完整SSE、工具读写、文件哈希、上游终态与chat.done，HTTP200和握手不算通过。

服务核查时间：`2026-11-12T14:04:11.725309+00:00`。统计：`{"attempt_status": {"invalid_response": 30, "passed": 33, "unauthorized": 9, "unexpected_http_status": 16}, "configured_models": 33, "model_status": {"access_denied": 9, "bounded_modes_passed": 9, "not_applicable_text": 7, "observed_with_failures": 8}, "production_attempt_status": {"capability_rejected": 6, "invalid_response": 1, "passed": 230, "unexpected_http_status": 20}, "research_protocol_cells": 35796, "source_binding_status": {"configured_binding_only": 7, "not_configured_enabled": 8919, "observed": 23}, "source_records": 8949, "thinking_tool_calls_with_observed_thinking": 12, "thinking_tool_results_with_successful_terminal": 12}`。

ACP核查时间：`2026-11-12T14:13:33.212348+00:00`。统计：`{"capability_rejected": 3, "incomplete_evidence": 16, "passed": 7, "transport_error": 1, "upstream_server_error": 2}`；验收：`{"all_probes_passed": false, "core_acceptance_passed": true, "provider_pairs": {"alibaba-token-plan-cn": true, "deepseek": true}, "shiju_public_interface_observed": true, "successful_client_protocols": {"chat": true, "gemini": true, "messages": true, "responses": true}}`。

ACP未通过观测（逐项保留，不用离线或旧成功覆盖）：

|索引|供应商/客户端|源→目标|模式|结果/完成条件|
|---:|---|---|---|---|
|1|alibaba-token-plan-cn/opencode|chat→messages|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|2|alibaba-token-plan-cn/opencode|chat→messages|explicit_none|incomplete_evidence；固定CLI拒绝显式none的session/set_config_option；需客户端提供可表达的关闭控制后重测，不能静默改用默认值。|
|4|alibaba-token-plan-cn/claude|messages→chat|request_default|capability_rejected；当前请求能力不能无损表达；改用有完整能力的原生路线或由用户显式调整请求。|
|5|alibaba-token-plan-cn/claude|messages→chat|explicit_none|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|7|alibaba-token-plan-cn/codex|responses→messages|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|8|alibaba-token-plan-cn/codex|responses→messages|explicit_none|incomplete_evidence；固定CLI拒绝显式none的session/set_config_option；需客户端提供可表达的关闭控制后重测，不能静默改用默认值。|
|10|alibaba-token-plan-cn/gemini|gemini→messages|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|11|alibaba-token-plan-cn/gemini|gemini→messages|explicit_none|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|12|deepseek/opencode|chat→chat|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|13|deepseek/opencode|chat→messages|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|14|deepseek/opencode|chat→messages|explicit_none|incomplete_evidence；固定CLI拒绝显式none的session/set_config_option；需客户端提供可表达的关闭控制后重测，不能静默改用默认值。|
|16|deepseek/claude|messages→chat|request_default|capability_rejected；当前请求能力不能无损表达；改用有完整能力的原生路线或由用户显式调整请求。|
|17|deepseek/claude|messages→chat|explicit_none|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|19|deepseek/codex|responses→messages|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|20|deepseek/codex|responses→messages|explicit_none|incomplete_evidence；固定CLI拒绝显式none的session/set_config_option；需客户端提供可表达的关闭控制后重测，不能静默改用默认值。|
|22|deepseek/gemini|gemini→messages|request_default|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|23|deepseek/gemini|gemini→messages|explicit_none|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|24|shiju/opencode|chat→chat|request_default|upstream_server_error；恢复同一上游服务/配额后重跑。|
|25|shiju/claude|messages→chat|request_default|capability_rejected；当前请求能力不能无损表达；改用有完整能力的原生路线或由用户显式调整请求。|
|26|shiju/claude|messages→chat|explicit_none|incomplete_evidence；重新执行并取得完整绑定、工具读写/哈希及终态证据。|
|27|shiju/codex|responses→chat|request_default|upstream_server_error；恢复同一上游服务/配额后重跑。|
|28|shiju/gemini|gemini→chat|request_default|transport_error；恢复客户端/上游传输后重跑同一绑定。|

服务观测的失败/拒绝、原HTTP与阶段逐条保留于service；coverage.summary.service_observation_followups为每个未通过观测列出失败尝试索引及完成条件。未配置模型继续以官方查询结论交付，不计作真实调用成功；配额、套餐/模型权限及服务恢复属于外部条件，不修改账号、不购费、不更换端点制造成功。

## 本轮验证

确切argv、退出码、有效数量和环境保存在coverage.validation；regression记录主要修复回归；last-test与last-test-retry记录最终nullable修正后的完整构建尝试。current-components为当前源码独立编译的19项测试；缓存二进制不能替代最新完整构建。早期final/validation为历史阶段。

### validation

动态槽位：5；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-5", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-5", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo test -p vw-shared --lib model_protocol_ -- --nocapture`|0|24|
|`cargo test -p vw-provider-resolver --lib model_protocol_ -- --nocapture`|0|23|
|`cargo test -p vw-agent --lib model_protocol_ -- --nocapture`|101|0|
|`cargo clippy --all-targets --all-features -- -D warnings`|101|0|

### build

动态槽位：7；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-7", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-7", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo fmt --all`|0|0|
|`cargo --config profile.dev.package.vw-agent.opt-level=0 build -p vw-cli --bin shiju`|0|0|
|`python3 scripts/verify_model_protocol_routes.py compile-relay`|0|0|
|`python3 -m unittest discover -s scripts -p test_model_protocol_bridge.py`|1|12|

### factory-red

动态槽位：8；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-8", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-8", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo fmt --all`|0|0|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib model_protocol_023_opencode -- --nocapture`|101|2|

### wire-red

动态槽位：7；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-7", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-7", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 -c 'import os,shutil;from pathlib import Path;source=Path('"'"'target/rust-slot-5/debug/model-protocol-service-relay'"'"');target=Path(os.environ['"'"'CARGO_TARGET_DIR'"'"'])/'"'"'debug/model-protocol-service-relay'"'"';target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)'`|0|0|
|`python3 -m unittest discover -s scripts -p test_model_protocol_bridge.py`|1|12|

### wire-green

动态槽位：5；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-5", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-5", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 -c 'import os,shutil;from pathlib import Path;s=Path('"'"'target/rust-slot-7/debug/model-protocol-service-relay'"'"').resolve();t=Path(os.environ['"'"'CARGO_TARGET_DIR'"'"'])/'"'"'debug/model-protocol-service-relay'"'"';t.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(s,t) if s!=t.resolve() else None'`|0|0|
|`python3 -m unittest discover -s scripts -p test_model_protocol_bridge.py`|0|12|

### final

动态槽位：7；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-7", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-7", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo fmt --all`|0|0|
|`cargo test -p vw-shared --lib model_protocol_ -- --nocapture`|0|24|
|`env SHIJU_UPDATE_PROTOCOL_MATRIX=1 cargo test -p vw-provider-resolver --lib model_protocol_020_ -- --nocapture`|0|1|
|`cargo test -p vw-provider-resolver --lib model_protocol_ -- --nocapture`|0|23|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib model_protocol_ -- --nocapture`|101|72|
|`cargo --config profile.dev.package.vw-agent.opt-level=0 build -p vw-cli --bin shiju`|0|0|
|`python3 scripts/verify_model_protocol_routes.py compile-relay`|0|0|
|`python3 -m unittest discover -s scripts -p test_model_protocol_bridge.py`|0|12|
|`python3 -m unittest discover -s scripts -p test_model_protocol_service_relay.py`|0|6|
|`python3 scripts/verify_model_protocol_routes.py boundary-tests`|0|14|
|`cargo clippy --all-targets --all-features -- -D warnings`|0|0|

### regression

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo fmt --all`|0|0|
|`env SHIJU_UPDATE_PROTOCOL_MATRIX=1 cargo test -p vw-provider-resolver --lib model_protocol_020_ -- --nocapture`|0|1|
|`cargo test -p vw-provider-resolver --lib model_protocol_ -- --nocapture`|0|23|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib model_protocol_ -- --nocapture`|0|74|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib acp_model_source_opencode_ -- --nocapture`|0|8|
|`cargo --config profile.dev.package.vw-agent.opt-level=0 build -p vw-cli --bin shiju`|0|0|
|`python3 scripts/verify_model_protocol_routes.py compile-relay`|0|0|
|`python3 -m unittest discover -s scripts -p test_model_protocol_bridge.py`|0|12|
|`python3 -m unittest discover -s scripts -p test_model_protocol_service_relay.py`|0|6|
|`python3 scripts/verify_model_protocol_routes.py boundary-tests`|0|14|
|`cargo clippy --all-targets --all-features -- -D warnings`|101|0|

### last-test

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo fmt --all`|0|0|
|`cargo --config profile.test.package.vw-agent.opt-level=0 test -p vw-agent --lib model_protocol_ -- --nocapture`|101|0|

### last-test-retry

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo fmt --all`|0|0|
|`cargo --config profile.test.package.vw-agent.opt-level=0 --config profile.test.package.vw-agent.codegen-units=256 --config profile.test.package.vw-agent.debug=0 test -p vw-agent --lib model_protocol_ -- --nocapture`|101|0|

### relay-baseline

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
来源说明：Uses the successfully compiled regression-3 binary (74 passed), prior to the final nullable-history-only change; not a claim that either SIGKILL compilation completed.

|`/Users/xiangmin/worker/code/shiju/target/rust-slot-6/debug/deps/vw_agent-4ce8574a348fdb7c acp_model_source_relay_ --nocapture`|101|19|

### scoped-clippy

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`cargo clippy -p vw-agent -p vw-provider-resolver --all-targets --all-features -- -D warnings`|0|0|

### live

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 scripts/verify_model_protocol_routes.py service --inventory docs/dev/model-protocols/catalog-inventory.json --output docs/dev/model-protocols/service-matrix.json`|0|0|
|`python3 scripts/verify_model_protocol_routes.py acp --inventory docs/dev/model-protocols/catalog-inventory.json --output docs/dev/model-protocols/acp-acceptance.json`|1|0|

### service-final

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 scripts/verify_model_protocol_routes.py service --inventory docs/dev/model-protocols/catalog-inventory.json --output docs/dev/model-protocols/service-matrix.json`|0|0|

### acp-final

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 scripts/verify_model_protocol_routes.py acp --inventory docs/dev/model-protocols/catalog-inventory.json --output docs/dev/model-protocols/acp-acceptance.json`|1|0|

### sdk-host-red

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 -m unittest discover -s scripts -p test_model_protocol_service_relay.py`|1|7|

### sdk-host-green

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 -m unittest discover -s scripts -p test_model_protocol_service_relay.py`|0|7|

### sdk-wire-green

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 -m unittest discover -s scripts -p test_model_protocol_bridge.py`|0|12|

### relay-expiry-retry

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`/Users/xiangmin/worker/code/shiju/target/rust-slot-6/debug/deps/vw_agent-4ce8574a348fdb7c acp_model_source_relay_binding_http_isolation_limits_and_lifecycle --nocapture --test-threads=1`|101|1|

### inverse-live

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 /tmp/shiju-task23-inverse-live.py`|0|0|

### inverse-none

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 /tmp/shiju-task23-inverse-none.py`|0|0|

### inverse-none-full

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 /tmp/shiju-task23-inverse-none-full.py`|0|0|

### current-agent-binary

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`/Users/xiangmin/worker/code/shiju/target/rust-slot-6/debug/deps/vw_agent-4ce8574a348fdb7c model_protocol_ --nocapture`|101|75|

### current-components

动态槽位：6；环境：`{"CARGO_TARGET_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_BUILD_BUILD_DIR": "/Users/xiangmin/worker/code/shiju/target/rust-slot-6", "CARGO_INCREMENTAL": "0"}`。

|命令|退出码|有效测试数|
|---|---:|---:|
|`python3 /tmp/shiju-task23-aux-final-source.py final-source-green`|0|19|

轻量检查：6组研究、catalog --check、Python审计24项、目录生成器3项、服务/ACP验收器35项均通过；主coverage完整性8949条/35796格通过；全部研究索引/证据ID、产物与生产文件哈希、实测二进制绑定核对通过，plan.md与17daf9535相比无修改。早期Cron DTO编译错误、主动终止的零测试与旧拒绝断言失败保留为历史，不能替代最新结果。build与Clippy的有效行为测试数为0，仅证明构建/静态检查；真实行为计数另列。

上一轮遗留状态（本轮已关闭）：已完成，待回归。最新完整Agent套件两次在codegen阶段被SIGKILL（101、0项），原因未明，降低内存参数的重试仍失败。此前74项完整聚焦通过；缓存二进制补跑为74项通过、1项nullable用例失败，不能用作最新源码成功证据。最后nullable修正由当前源码独立重新编译的组件19项及新产物HTTP回环验证。当前相关crate全targets/features Clippy通过；全仓Clippy因无关桌面工作树设置的两个未跟踪文件报8项lib/12项lib-test dead-code错误而退出101。旧74项套件二进制的relay专项17项通过、2项失败、1项忽略：server_tests.rs:278与messages_tests.rs:491首个流块缺失触发unwrap；server过期案例单独重跑仍失败。未修改这些无关源码，不以历史Clippy成功替代本次结果。下一步是在编译环境稳定、桌面设置连线完成并查清既有流式定时测试后，重跑完整Agent聚焦和全仓Clippy；协议实现本身无已知普通文本转换缺口。

本次自有槽位在所有使用子进程退出后已释放，编译缓存保留。

最终回归证据齐全；任务23已完成，按要求删除任务文件。
