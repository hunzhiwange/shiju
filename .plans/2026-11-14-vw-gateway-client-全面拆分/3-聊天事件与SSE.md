# 任务 3：聊天事件与SSE

先阅读 [公共规则](plan.md)、[全文件审计](source-map.md) 和 [依赖审计](audit.md)。

状态：未开始

批次：3，线性顺序 3；依赖：前一批次 2。符号依赖卡：无额外前置。不并行。

目标：crates/libs/gateway-chat-stream。

## 来源与迁移边界

- `crates/vw-gateway-client/src/stream.rs`
- `crates/vw-gateway-client/src/stream_tests.rs`

真实函数入口（包括需要一起迁移的私有 helper）：stream.rs: stream_http_failure_message, legacy_delta, normalize_chat_tool_event, stream_json_sse, normalize_chat_stream_event, stream_chat, stream_chat_resumable, stream_chat_with_idle_timeout, stream_chat_inner, stream_chat_attempt, chat_uses_acp, stream_chat_blocking, stream_chat_blocking_with_idle_timeout, parse_stream_event, take_next_sse_event, parse_sse_frame, apply_stream_resume, required_event_id, event_name, is_terminal_stream_event, normalize_chat_usage, parse_step_start_payload, parse_step_finish_payload, parse_post_tool_round_payload。

跨域调用：未发现跨域 self 方法调用。

980 行跨协议与网络：本任务先迁移类型、normalize_*、SSE帧解析到独立模块，网络函数仍同领域库的 transport 模块；不把980行原样放入lib.rs。异步、阻塞、resumable、idle timeout、413文案及事件ID必须保留。

## 实施步骤

1. 阅读上述完整文件及关联测试，重新搜索 consumer-refs.md 所列四包的直接调用者；核实前置源码落地，禁止假设计划包已经存在。逐文件归属表覆盖测试与私有符号，不能只迁移 pub 方法。
2. 将 SSE 自由函数、客户端事件类型与归一化迁入聊天库；保留现有自由函数 API。使用端点及 HTTP helper，不为无状态订阅强造领域客户端，保持返回类型、错误文案、URL/query/body、认证与租户校验。跨域调用按上表显式组合所有者；不能依赖聚合 GatewayClient 或复制另一领域实现。
3. 更新 workspace members、原包/直接消费者 Cargo.toml 及 lib.rs 导出和显式转发；只引入本卡真实使用依赖，保留原 cfg、path 属性、原生/wasm 分支与错误拒绝路径。既有类型优先复用 vw-api-types、vw-shared 实际导出，不引入臆测类型库。
4. 测试源码随行为迁移至独立测试文件；共享 test_support 的客户端工厂改为本领域类型，保留请求捕获和所有安全断言。门面兼容测试留原包。include 路径按新相对位置核对，当前包未发现 include_str!/include_bytes! 资源，不新增资源或 build.rs。
5. 静态审查新库不得引用 vw_gateway_client；核对源模块已经移除或只剩转发，消费者直达领域库。完成后按公共契约提交并删除本卡，不执行动态验证。

## 静态证据与后续验收

允许命令（仓库根）：`git diff --check`、`git diff --stat`、`rg -n 'vw_gateway_client|pub use|mod |path =' crates/libs/gateway-chat-stream crates/vw-gateway-client`（门面任务改查原包）；阅读实际调用者并核对清单路径。rg 无匹配按搜索目的判断。

后续另行授权统一验收：本卡来源中既有测试的全部行为、线协议方法/路径/查询/正文、错误类型与文案、空值/404、认证与租户隔离、原生/wasm cfg 及门面兼容。涉及流式接口另验 SSE 帧边界、断流/取消/恢复与 WebSocket 关闭；涉及宿主请求另验禁重定向/重试及授权拒绝。当前编译、测试、性能与实机均未验证，不宣称通过。
