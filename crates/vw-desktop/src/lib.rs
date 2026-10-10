#![allow(
	// 当前阶段优先保证工作区 Clippy 收敛，桌面端历史风格告警后续再分批治理
	clippy::all,
	// UI 模块以可维护性为先，暂不将 pedantic 告警作为阻塞项
	clippy::pedantic,
	// 允许复杂类型直接内联，避免为 UI 消息类型引入大量别名跳转
	clippy::type_complexity,
	// 允许递归辅助参数仅用于递归传递，保持遍历函数简单
	clippy::only_used_in_recursion,
	// 允许 Default 后再赋值字段，便于逐步组装 UI 状态
	clippy::field_reassign_with_default,
	// 允许基于索引的循环，便于多数组/多缓冲区同步处理
	clippy::needless_range_loop,
	// 允许参数较多的 UI 构建函数，避免过度拆分状态传递
	clippy::too_many_arguments,
	// 允许局部 `from_str` 风格方法，兼容既有模型接口
	clippy::should_implement_trait,
	// 允许结构体更新语法保留，即使当前字段已显式列出
	clippy::needless_update,
	// 允许模块名与目录同名，保持功能分层稳定
	clippy::module_inception,
	// 允许保留当前布尔表达式形状，减少视觉噪音式改写
	clippy::nonminimal_bool,
	// 允许相同 if 分支体，避免为消除 lint 引入额外分支变量
	clippy::if_same_then_else,
	// 允许文档列表延续格式，避免大规模重排现有说明文档
	clippy::doc_lazy_continuation,
	// 允许显式计数器循环，便于 UI 序号和布局索引同步
	clippy::explicit_counter_loop,
	// 允许保留重绑定，便于逐步收敛中间布局变量
	clippy::redundant_locals,
	// 允许 `&Vec` 参数以兼容既有调用面
	clippy::ptr_arg,
	// 允许手写范围判断，保持条件表达可读性
	clippy::manual_range_contains
)]

pub mod app;
pub mod apps;
pub mod fonts;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;


#[cfg(test)]
include!("legacy_task_board_switches_exact_tests.rs");
