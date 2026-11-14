# S3 下载契约

入口为 `device_sync::s3_transport::S3DownloadTarget`，提供 `head`、`download_manifest` 和 `download`。Gateway 须先授权主机网络访问并解析凭据；构造目标不会授予网络权限。兼容版本由共享快照所有者提供，不允许为零。

对象路径与上传保持一致，仅接受 manifest.json、config.json 和 skills.zip。GET/HEAD 使用现有 SigV4 签名器，直接发送签名请求；禁止重定向和自动重试。仅 HTTP 404 返回 None，403、206 及其它非 200 状态均返回错误。成功响应必须提供非空 ETag，原样返回该值。

manifest 上限沿用 MAX_MANIFEST_BYTES；制品由 ArtifactDeclaration 限定大小及 SHA256，并受 MAX_ARTIFACT_SIZE 限制。逐块校验，成功 EOF、长度与摘要全部满足后才返回暂存字节，不写文件或应用配置。空闲时限 120 秒，总时限为 300 秒加每 16 KiB 一秒，与 WebDAV 下载一致。禁用透明解压并拒绝非 identity 内容编码，避免摘要与传输字节不一致。

本轮按用户指令仅实现与静态审阅，未运行 Cargo、单元测试、loopback 或实机验证；未领取槽位，CARGO_TARGET_DIR 未设置。后续统一回归需覆盖签名 GET/HEAD、ETag、404/403/206、超限、截断、摘要不符及超时。
