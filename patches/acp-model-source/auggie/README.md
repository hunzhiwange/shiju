# Auggie 固定版本补丁审查

未提供可应用补丁，未修改或重新分发上游 CLI。

审查对象为官方 `@augmentcode/auggie@0.36.0` 与独立调查的
`@augmentcode/auggie@1.2.0-prerelease.202611031510`。固定版本、完整性及可复现
探测见 [Auggie 证据](../../../docs/vibewindow/acp-model-source/evidence/auggie.md)。

0.36.0 没有 `--extension` 参数；1.x 的公开扩展工厂被成功调用，但在
`registerProvider` 调用时明确抛出 `Extension provider registration is not supported`。
因此只修改本仓库 driver、缓存根或 fixture 不能完成接入；需要改变上游模型注册路径，
并确认 ACP、主/辅助模型及恢复都使用同一短期授权模型服务。

两个 npm 发行包的 `LICENSE.md` 对 CLI 自有代码明确规定：
`The CLI must not be reverse-engineered, decompiled, or modified.`
1.x 另行保留第三方组件各自的许可，但没有证据表明上述 Auggie 注册限制属于
可独立修改的第三方组件。不能凭借 pi 的开源许可推定有权移除此发行包的限制。
此处记录已观察到的分发条件，不替代法律判断，也不假定维护者拥有未提供的授权。

本任务的原计划明确排除绕过厂商权限，故没有对发布包做逆向、反编译、字节替换，
也没有通过运行时注入替换注册实现。可继续的条件是上游新增公开 Provider 注入 API，
或维护者提供适用于对应版本及改动的授权、可审查源码/补丁基线。

获得该条件后再制作最小补丁，固定上游版本及校验值，并补齐：

- ACP 扩展注册与模型选择，缺失/撤销凭据时直接失败。
- 主/辅助模型全部走短期中继，禁用 Augment 账号回退。
- 不把秘密放入 argv，隔离目录与恢复锚点不跨来源复用。
- 原 CLI 在两种来源下实际请求离线 fixture，包含工具、恢复、取消和切模证据。

在此之前，本目录仅是补丁审查记录，不构成支持声明或可安装补丁。
