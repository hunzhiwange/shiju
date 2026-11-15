//! Git diff 局部渲染辅助。
//!
//! 本模块负责 diff 行、行号、选区、上下文菜单和配色的局部组合。

use iced::{Color, Element};

use crate::git_panel::diff_view::DiffRenderCtx;
use crate::DiffTheme;
use crate::git_panel::diff_view::{DiffViewState, DiffMessage as Message};
use similar::DiffOp;

use super::standard;
use crate::git_panel::utils::Lang;

/// deletes 子模块承载当前组件的一部分独立职责。
mod deletes;
/// inserts 子模块承载当前组件的一部分独立职责。
mod inserts;
/// replaces 子模块承载当前组件的一部分独立职责。
mod replaces;


/// 渲染 hunk ops 对应的 diff 行、工具卡片或控件内容。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn render_hunk_ops(
    app: &DiffViewState<'_>,
    // render_ctx 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    render_ctx: &DiffRenderCtx<'_>,
    // file 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    file: &str,
    // group 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    group: &[DiffOp],
    // _hunk_index 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    _hunk_index: usize,
    // old_lines 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    old_lines: &[&str],
    // new_lines 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    new_lines: &[&str],
    // lang 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    lang: Lang,
    // effective_theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    effective_theme: DiffTheme,
    // bg_default 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    bg_default: Color,
    // add_line_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    add_line_bg: Color,
    // add_word_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    add_word_bg: Color,
    // del_line_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    del_line_bg: Color,
    // del_word_bg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    del_word_bg: Color,
    // hover_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    hover_color: Color,
    // hover_mix 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    hover_mix: f32,
    // hover_tint 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    hover_tint: Color,
    // has_selection 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    has_selection: bool,
    // is_modified 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    is_modified: bool,
) -> Vec<Element<'static, Message>> {
    let mut col: Vec<Element<'static, Message>> = Vec::new();
    for op in group.iter() {
        match op {
            // DiffOp 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            DiffOp::Equal { old_index, new_index, len } => {
                for k in 0..*len {
                    let old_idx = old_index + k;
                    let new_idx = new_index + k;
                    let content = old_lines.get(old_idx).unwrap_or(&"");
                    col.push(standard::render_equal_line(
                        app,
                        render_ctx,
                        file,
                        old_idx,
                        new_idx,
                        content,
                        lang,
                        effective_theme,
                        bg_default,
                        hover_tint,
                        has_selection,
                    ));
                }
            }
            DiffOp::Delete { old_index, old_len, new_index } => {
                let elems = deletes::render_delete_ops(
                    app,
                    render_ctx,
                    file,
                    *old_index,
                    *old_len,
                    *new_index,
                    old_lines,
                    lang,
                    effective_theme,
                    del_line_bg,
                    del_word_bg,
                    hover_color,
                    hover_mix,
                    hover_tint,
                    has_selection,
                    is_modified,
                );
                col.extend(elems);
            }
            DiffOp::Insert { new_index, new_len, .. } => {
                let elems = inserts::render_insert_ops(
                    app,
                    render_ctx,
                    file,
                    *new_index,
                    *new_len,
                    new_lines,
                    lang,
                    effective_theme,
                    add_line_bg,
                    add_word_bg,
                    hover_color,
                    hover_mix,
                    hover_tint,
                    has_selection,
                    is_modified,
                );
                col.extend(elems);
            }
            DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                let elems = replaces::render_replace_ops(
                    app,
                    render_ctx,
                    file,
                    *old_index,
                    *old_len,
                    *new_index,
                    *new_len,
                    old_lines,
                    new_lines,
                    lang,
                    effective_theme,
                    add_line_bg,
                    add_word_bg,
                    del_line_bg,
                    del_word_bg,
                    hover_color,
                    hover_mix,
                    hover_tint,
                    has_selection,
                    is_modified,
                );
                col.extend(elems);
            }
        }
    }
    col
}

#[cfg(test)]
mod inserts_tests;

#[cfg(test)]
mod mod_tests;

#[cfg(test)]
mod replaces_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod deletes_tests;
