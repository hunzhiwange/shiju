# WebDAV 下载契约

入口为 `webdav_transport::download::WebDavDownloadTarget`。Gateway 负责授权主机网络访问、解析凭据及提供共享快照兼容版本。传输仅获取固定原生制品，不应用快照、不写入持久配置。

- `head` 返回 ETag 与可选 Content-Length；`download_manifest` 返回最多 1 MiB 原始 manifest，调用方必须交给 manifest 所有者解析。
- `download` 消费已验证 manifest 的 `ArtifactDeclaration`，在成功 EOF、精确长度和 SHA256 均符合后返回原始暂存字节。单制品上限 512 MiB；失败丢弃缓冲区。
- HEAD/GET 仅 404 返回 `None`，401/403 等保留 HTTP 状态；缺失或空 ETag、非法 Content-Length 显式报错。拒绝重定向和非 200 成功响应，不自动重试。
- 有、无 Content-Length 均逐块核查真实字节上限；声明长度不扩大预算。GET 总预算为 300 秒加可信制品长度（manifest 使用上限）除以 16 KiB/s 向上取整，包含响应头等待；120 秒未收到非空数据即超时。HEAD 响应头等待上限 120 秒。
- 返回值持有暂存内存，不包含凭据；ETag 不应直接显示或写入日志。调用方须继续完成 manifest 协议校验和安全恢复事务。

本轮按用户指令仅实现及源码审阅，未运行 Cargo、单元测试、实机测试或其它测试，也未领取构建槽位。后续统一验证须覆盖分块停滞、持续慢速进展、伪造/缺失长度、认证和缺失 ETag 等边界；本文件不作为行为通过证据。
