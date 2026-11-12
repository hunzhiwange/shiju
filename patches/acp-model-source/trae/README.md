# TRAE 0.121.3 独立本地模型入口

仅适用于固定 Darwin arm64 官方归档。安装器核对归档、原始二进制、独立本地二进制与启动器的 SHA-256；不覆盖用户的 `traecli`，不修改平台登录、账号类型或配额结果。

`apply.py` 使用固定 Go arm64 发行物的函数地址与 ABI，生成单独的 `native/trae-local`，经 ad-hoc codesign 签名。原始 `native/trae-cli` 保持逐字节一致，未绑定的版本查询及普通 ACP 诊断仍调用原始入口。模型循环、工具、原生权限中间件、ACP 服务均来自同一 TRAE 可执行文件，没有换引擎。

本地组合仅改变三个位置：

- `query.init` 在 `0x101462458` 将 fx option 列表长度从 4 改为 2，保留 FlowTrace 和 MapTraeModelError，不装配平台 CommercializationPreCheck 与 ReportCustomModelTokenUsage。其实现和真实返回值完全保留；本地组合不声明平台登录成功、配额充足或拥有企业功能。
- `conf.PathConfig.UserDotDir`（`0x100f055b0`）与 `CacheDir`（`0x100f05670`）使用 Go 字符串返回 ABI，尾调用原生 `os.Getenv`（`0x1001039b0`）读取专属目录。未修改 HOME，不借用官方账号状态。固定包实际配置名是 `trae_cli.yaml`，与该包内置手册的 `traecli.yaml` 不同。

启动器要求 0700 请求目录、0600 绑定文件和匹配的会话引用，强制仅有一个选定模型、关闭模型回退、压缩和视觉模型引用同一模型。短期中继令牌仅通过私有环境传递，配置只保留环境变量占位符。请求拥有者销毁时撤销中继并删除目录。

原生本地进程启动前，由 Darwin `sandbox-exec` 编译网络策略：只允许所绑定的回环 TCP 端口，拒绝上游平台、遥测、DNS 和无关回环连接；策略同时覆盖原生后台任务与子进程。Darwin 策略的地址语法使用 `localhost:port`，模型配置及 Rust 边界仅接受 `http://127.0.0.1:port`。这项网络限制不等于所有原生工具的完全访问能力均已验证。

创建前验证命令：

```sh
VW_TRAE_TEST_RUNTIME=/absolute/path/to/trae-0.121.3-model-source-v1 \
  python3 scripts/acp_model_source_trae_tests.py -v
```

探针使用真实原生 ACP 与本机 HTTP 400 响应，仅证明原生模型请求到达私有端点；不把探针错误当作编辑器验收成功。实际指定模型创建证据见 `docs/dev/acp/ACP验收.md`。
