# 提示词条目与激活 SDK

任务 27 消费任务 21–23 的实际路由及固定来源 ca58c0313360eae7c851fbd7553c1614c7af03e7。GatewayClient 提供目录搜索、当前文件、位置、保存、删除、复制、导入、激活与停用。公开类型从 client 导出。

每次调用显式传入 AgentAppPromptsScope（projectId、targetHostId、appId）和独立应用凭据；不缓存当前应用或主机。GET 用 query 序列化编码；POST 按实际端点发送 snake_case scope 与 operation。目录 revision 保留 u64，激活与停用原样传递 expected_content，null 与空字符串不同。目录条目不复用旧 PromptEntry 的 enabled/revision 合同，activeId 属于目录；复制结果逐目标保留 entry 或 code/status。

使用既有禁止重定向、禁止重试的 native transport 及认证请求上下文；wasm 明确拒绝。HTTP 状态及原始错误 JSON 保留在 AgentAppPromptsError::Http，Display 不输出正文。SDK 不访问本机文件，不使用显示路径作为写入输入。

按本轮要求未运行 Cargo、单元测试、实机测试或其他测试，未领取构建槽位，未启动最终回归。后续统一验证需以本地 HTTP 记录器覆盖参数编码、scope 切换、所有操作请求结构、null/空文件、revision 冲突、401/403/404/409/413/422/503、复制部分失败、畸形响应以及请求上下文继承。
