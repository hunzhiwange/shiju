//! 线程本地上下文、延迟执行与同步辅助工具。

pub mod context;
pub mod defer;
pub mod r#fn;
pub mod iife;
pub mod lazy;
pub mod proxied;
pub mod scrap;

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod defer_tests;
#[cfg(test)]
mod fn_tests;
#[cfg(test)]
mod iife_tests;
#[cfg(test)]
mod lazy_tests;
#[cfg(test)]
mod proxied_tests;
#[cfg(test)]
mod scrap_tests;
