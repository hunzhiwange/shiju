# Relax 本地兼容修复

这组补丁用于验证 ShiJu 的 gateway 和 Iced 桌面端，目标为 `makepad/relax` 提交 `0e2a536309c5ee7c7a47d8806d603a9c12b9a586`。它修改 Relax，不修改 ShiJu 业务代码或依赖 features。运行结果与限制见 [开发验证报告](../../docs/dev/Relax开发验证.md)。

`compatibility.patch` 包含：

- 同步 Makepad `work` 分支已有的 `vec!` 条件元素、静态项内异步闭包修复。
- 保留宏的 `stmt` 片段类型与边界；片段标记扩为 `u64`，保留完整 `u32` 偏移。
- 补齐 `Weak` 原始指针转换、切片 `split_off`/`split_off_first`、`Chain: BufRead` 和 `PhantomPinned: Debug`。
- 正确收集函数泛型参数约束内的匿名 `impl Trait` 参数，保留原有 trait 与关联类型约束。
- 为 macOS 原生静态库的隐藏符号生成地址访问器，沿用 Linux 的查找机制；不修改原符号可见性，不为 TLS 符号生成访问器，并升级原生链接缓存版本。
- 支持 AArch64 语句起始处的 `#` 汇编注释，保留指令内的 `#` 立即数以及 asm 输入、选项和屏障语义。

新增 Tower 修复保留代码生成所属 crate：实例槽、虚表、ahead lowering、CTFE 证书和持久化缓存均携带上下文；按该上下文读取原有递归限制，缓存格式升级为 35。没有全局提高限制或绕过 `Service`、`Send` 等约束。最终同一二进制已通过 gateway `/v1/health`、桌面项目打开及真实模型对话回传，详见 [本轮证据](../../docs/dev/evidence/relax-tower-2026-11-13.json)。跨 crate 上游泛型实例共享等限制仍保留。

文件清单、基线和补丁校验值见 [manifest.json](manifest.json)。修改先在 Makepad `work` 源码中完成，再同步至本地 Relax；未对外推送或提交 PR。

## 应用和构建

在上述基线的干净 Relax checkout 中执行，先把补丁复制到能稳定访问的位置：

```bash
git apply --check /absolute/path/to/compatibility.patch
git apply /absolute/path/to/compatibility.patch
cargo build --release -p relax-runner --bin relax -j 4
```

显式指定 `--target-dir` 可避免外层 Cargo 配置把产物放入其他项目目录。保留 Relax checkout，运行器仍需读取其中的标准库源码。

回滚前先检查是否存在后续修改：

```bash
git apply --reverse --check /absolute/path/to/compatibility.patch
git apply --reverse /absolute/path/to/compatibility.patch
```

## 运行 ShiJu

在完整 ShiJu 工作区中运行，`RELAX` 指向修复后的绝对可执行文件路径：

```bash
"$RELAX" run --verbose --allow-c --cfg debug_assertions \
  vw-cli --bin shiju -- gateway
"$RELAX" run --verbose --allow-c --cfg debug_assertions \
  vw-desktop --bin shi-ju
```

`--cfg debug_assertions` 对齐用户要求的 `cargo run` 开发配置；Relax 的默认依赖图为 release 配置，ShiJu 的本机服务引导在两种配置下行为不同。`--allow-c` 供已有 C/C++ 依赖构建使用。保持进程常驻后保存函数体改动即可尝试热更新；不要加 `--unattended`，该选项会禁用 live session。完整本机配置与验收步骤见[开发验证报告](../../docs/dev/Relax开发验证.md#如何在本机获得这个效果)。

本机 rsproxy sparse registry 替换配置需要独立处理：本次在用户目录之外复制完整工作区，临时 `CARGO_HOME` 复用已有 registry/git 缓存，并保留 source registry 配置供委托 Cargo 构建。没有修改全局 Cargo 配置。具体复现环境见开发验证报告。

该补丁是本机兼容性实验，不能据此推定整个 Rust 生态、所有 ShiJu 功能或其他平台均兼容。

## 团队开发入口补充

`vendor-resolution.patch` 在上述兼容补丁之后应用，使 directory-source 按调用方版本约束筛选同名依赖。`scripts/relax_dev.py` 自动校验和应用两份补丁，并管理安装及依赖副本；团队使用 `cargo dev-setup`、`cargo dev-gateway`、`cargo dev-desktop`。新增补丁哈希见 manifest 的 `dev_vendor_patch_sha256`，原有二进制哈希只对应历史测量版本。当前一键入口见[报告](../../docs/dev/Relax开发验证.md#团队一键入口当前仓库)。
