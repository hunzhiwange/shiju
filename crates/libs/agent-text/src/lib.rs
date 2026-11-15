//! 颜色、快捷键、token 估算与通配符匹配工具。

pub mod color;
pub mod keybind;
pub mod token;
pub mod wildcard;

#[cfg(test)]
mod color_tests;
#[cfg(test)]
mod keybind_tests;
#[cfg(test)]
mod token_tests;
#[cfg(test)]
mod wildcard_tests;
