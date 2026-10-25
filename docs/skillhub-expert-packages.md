# SkillHub 专家包

## 使用

1. 打开项目聊天，在「专家」页浏览或搜索 SkillHub 专家包，进入详情查看主工作流和成员。
2. 点击「安装」，在弹窗中选择目标项目并确认。安装成功后，项目的 `.vibewindow/skills/` 下会出现主技能和成员技能目录。
3. 在同一项目的聊天标签中打开已安装专家详情，选择主技能。输入区显示已选专家；可以在发送前取消。
4. 发送下一条消息时，主技能作为本次显式选择传入。发送后选择会清空；后续消息要再次使用时需重新选择。成员技能按工作流需要调用，不会因安装自动执行。

## 安装边界与失败说明

- 安装目标仅为所选项目。当前不支持专家包全局安装、覆盖升级、批量卸载或同步到第三方 Agent；需要更新时应按后续版本提供的明确流程处理，不要直接覆盖现有目录。
- 安装器校验包与成员坐标、版本、签名、哈希、ZIP 内容、主技能 frontmatter 和成员清单，失败时返回具体原因并回滚，不发布部分目录。不能解析的在线成员、缺失的版本或不完整的制品会被拒绝；目录项可浏览不代表可安装。
- 外部专家包内容仍是技能指令，不具有系统或身份指令权限。安装不授予额外工具、网络、文件或命令权限。请在使用前阅读主技能和成员内容。

## 回归记录（2026-10-25）

- `cargo test -p vw-agent --lib expert_package_ -- --nocapture`：退出码 0，12 项通过。临时项目安装到 `.vibewindow/skills/fixture--skillset--fixture-suite/`，加载主技能 ID `fixture--skillset--fixture-suite`，实际 `skill` 工具调用返回 `skill.name` 上下文事件；错误 ZIP 返回具体原因且无部分安装。另覆盖成员按需加载及权限边界。
- `cargo test -p vw-desktop expert_package_ -- --nocapture`：退出码 0，14 项通过。覆盖列表与详情状态、安装弹窗、选择和发送后清空；Iced headless renderer 在明暗主题下绘制中文长文本、重试状态和输入标签。截图保存在 `/tmp/vw-skillhub-task7/screenshots/`，包括 `experts-00.png` 至 `experts-10.png`、`expert-install-{light,dark}.png` 和 `expert-input-{light,dark}.png`。列表滚动条宽度为 4px，内容两侧留白 7px。
- `cargo clippy --all-targets --all-features -- -D warnings`：退出码 0。验证领取槽位 1，`CARGO_TARGET_DIR=/Users/Shared/work/dir/data/codes/vibe-window/target/rust-slot-1`，并将 Cargo 的 `build-dir` 指向同一槽位；已释放锁，保留缓存。
- 公开 SkillHub 列表与 `tech-test-automation` 详情只读冒烟成功。该详情含 `test-case-generator` 成员但缺少 namespace，当前属于可解释的上游未解析成员；未将夹具安装结果当作在线安装成功。
