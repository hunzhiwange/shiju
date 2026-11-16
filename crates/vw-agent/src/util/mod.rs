//! # Agent 工具模块
//!
//! 本模块提供 Agent 运行时所需的各种通用工具函数、类型和辅助设施。
//!
//! ## 模块结构
//!
//! - `abort` - 任务中止相关工具
//! - `archive` - 归档与压缩工具
//! - `color` - 终端颜色输出工具
//! - `context` - 上下文管理工具
//! - `defer` - 延迟执行工具
//! - `eventloop` - 事件循环抽象
//! - `filesystem` - 文件系统操作工具
//! - `fn` - 函数式编程工具
//! - `format` - 数据格式化工具
//! - `iife` - 立即执行函数表达式工具
//! - `keybind` - 键盘快捷键绑定
//! - `lazy` - 延迟初始化工具
//! - `locale` - 本地化与国际化工具
//! - `lock` - 锁与同步原语
//! - `log` - 日志记录工具
//! - `proxied` - 代理相关工具
//! - `queue` - 队列数据结构
//! - `rpc` - 远程过程调用工具
//! - `scrap` - 数据抓取工具
//! - `signal` - 信号处理工具
//! - `task_set` - 任务集合管理
//! - `timeout` - 超时处理工具
//! - `token` - 令牌与计数工具
//! - `wildcard` - 通配符匹配工具

pub use agent_async::{abort, eventloop, lock, queue, task_set, timeout};
pub use agent_context::{context, defer, r#fn, iife, lazy, proxied, scrap};
pub use agent_text::{color, keybind, token, wildcard};

pub use agent_util::archive as archive;
pub use agent_util::filesystem as filesystem;
pub use agent_util::format as format;
pub use agent_util::locale as locale;
pub use agent_log as log;
pub use agent_util::rpc as rpc;
pub use agent_async::signal as signal;

pub use agent_util::{spawn, truncate_with_ellipsis, floor_utf8_char_boundary, MaybeSet};
