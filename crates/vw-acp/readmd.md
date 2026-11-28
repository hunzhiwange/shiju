# ACP 架构

ShiJu ACP 按领域拆分为 `crates/libs/acp-*`。`vw-acp` 保留兼容公共导出，二进制只设置运行环境并调用 `acp-cli-app`；客户端、会话编排、服务端、存储及 CLI 实现分别由领域库持有，新包不依赖门面。

详见[分包说明](分包说明.md)：逐文件归属、实际依赖、测试与资源位置及后续统一验收范围。本轮未编译、未测试、未实机验收，锁文件未更新，`--locked` CI 未验收。

## 原有记录

参考 https://github.com/openclaw/acpx 实现
