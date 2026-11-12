# Auggie 0.37.1 会话模型绑定

补丁在固定版本原生 CLI 的 `chatStream` 与模型目录入口接入本机会话中继，保留其 ACP 协议、原生工具、权限和会话历史。不会替换为其它代理引擎。全局 npm 安装不变，安装器在独立受校验目录应用补丁。

`SHIJU_AUGGIE_MODEL_CONFIG` 仅传入私有配置文件路径；实际模型、回环端点与短期令牌由 Rust 会话拥有者写入，原始上游密钥不交给 CLI。模型目录只公布当前选定模型；请求不匹配时拒绝，不回退官方模型。该模式关闭远程代码索引，官方远程检索和服务接口没有被模拟为可用。原生文件工具可能在客户端回调外执行，因此桌面使用必须有当前完全访问授权。

支持文本、工作目录元数据、原生工具调用与结果、并行工具参数流、中文流和 OpenAI `reasoning_content` 的历史往返。图片、未知内容类型、未完成工具结果和不支持的终态明确拒绝；截断或错误流不会产生成功终态。当前证据范围为 macOS arm64。

验证：

```sh
node --test patches/acp-model-source/auggie/session-model.test.mjs
python3 scripts/acp_model_source_auggie_tests.py /absolute/path/to/reviewed/augment.mjs
```

安装输入归档完整性、原 bundle SHA-256 和补丁后文件 SHA-256 均由受管安装器检查；`patch.json` 是原 bundle 的证据边界，`contracts.json` 是发布产物的证据边界。

## 历史审查记录

此前对 0.36.0 与 `1.2.0-prerelease.202611031510` 的调查未制作可应用补丁：0.36.0 没有 `--extension` 参数；1.x 的公开扩展工厂虽被调用，`registerProvider` 仍抛出 `Extension provider registration is not supported`。这些旧版本的探测证据保留在仓库 `docs/shiju/acp-model-source/evidence/auggie/`，不作为当前 0.37.1 补丁的支持证据。

旧版发行包的 `LICENSE.md` 对 CLI 自有代码包含 `The CLI must not be reverse-engineered, decompiled, or modified.` 条款；1.x 的第三方许可不能自动推定适用于 Auggie 自有实现。此处分发条件记录继续保留，不代表本项目授予上游许可。本轮实现依据用户明确提出的驱动修复请求，固定为独立安装产物；发布前仍需维护者核实对应版本的许可与分发条件。
