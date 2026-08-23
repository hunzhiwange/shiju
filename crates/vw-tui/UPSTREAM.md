# OpenAI Codex TUI 来源记录

`vw-tui` 是 OpenAI Codex TUI 的完整复制件，后续在副本内按批次裁剪和改写。

## 来源快照

- 来源路径：`/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui`
- 目标路径：`/Users/Shared/work/dir/data/codes/vibe-window/crates/vw-tui`
- 复制日期：2026-08-22（Asia/Shanghai；目标目录创建时间为 22:05:11 +0800）
- 上游版本：来源快照无可读取的 Git 元数据，因此不记录或推测 commit、tag、release 版本。
- 源文件统计：1550 个普通文件，其中 492 个 Rust 文件、272084 行 Rust。
- 初始内容校验：递归 `diff` 无输出；源/目标清单聚合 SHA-256 均为 `23bf2b2142236ef046fe7970317702a1ead57d99b7a1b640580cd44d780eb7c9`。

统计和校验值取自复制完成后的原始副本、写入本文件之前。可用以下命令重新生成来源清单和内容差异：

```bash
source_tui=/Users/Shared/work/dir/data/codes/codex-main/codex-rs/tui
target_tui=/Users/Shared/work/dir/data/codes/vibe-window/crates/vw-tui

find "$source_tui" -type f | wc -l
find "$source_tui" -type f -name '*.rs' | wc -l
find "$source_tui" -type f -name '*.rs' -print0 | xargs -0 wc -l | tail -n 1
diff -qr "$source_tui" "$target_tui"

(
  cd "$source_tui"
  find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256
) | shasum -a 256
```

后续 `diff` 会包含已审查的 VibeWindow 移植改动以及本目录新增的来源管理文件；每个顶层模块的处置以 `PORTING_STATUS.md` 为准。

## 许可

- `LICENSE-CODEX` 原样复制自 `/Users/Shared/work/dir/data/codes/codex-main/LICENSE`，SHA-256 为 `d17f227e4df5da1600391338865ce0f3055211760a36688f816941d58232d8dc`。
- `NOTICE-CODEX` 原样复制自 `/Users/Shared/work/dir/data/codes/codex-main/NOTICE`，SHA-256 为 `9d71575ecfd9a843fc1677b0efb08053c6ba9fd686a0de1a6f5382fd3c220915`。
- 修改上游文件时须保留适用的版权和归属声明，并增加简短、醒目的 VibeWindow 改动说明。

## 产品文案允许差异

以下差异仅允许改变展示数据或产品文案，不允许改变上游组件结构、布局算法、样式层级、键位、终端行为或快照尺寸：

| 类别 | 允许替换 | 边界 |
| --- | --- | --- |
| 品牌 | `OpenAI Codex`、`Codex` 改为 `VibeWindow` | 只改用户可见品牌，不批量替换来源、版权、许可、模块名或历史快照说明 |
| 命令 | Codex 命令名、提示和快捷入口改为 VibeWindow 已支持的命令 | 不伪造尚未实现的命令，不改变通用列表和 composer 布局 |
| URL | OpenAI 产品链接改为对应的 VibeWindow 文档或支持链接 | 来源与许可链接保持原样 |
| 会话数据 | 模型、工作目录、权限、endpoint、token 等展示值改由 VibeWindow 提供 | UI crate 不读取 Gateway、登录、模型或配置服务 |
| 状态文案 | 登录、额度、更新等 Codex 业务文案删除或替换为已有 VibeWindow 状态 | 无替代能力时明确不支持，不保留伪状态 |

任何新类别须先补入此白名单；产品文案以外的快照差异必须在对应批次单独审查，不能借本白名单接受。
