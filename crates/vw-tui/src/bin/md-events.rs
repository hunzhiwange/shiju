//! 从标准输入读取 Markdown，并逐行输出解析器事件，供渲染问题调试使用。
//! 该小工具只观察解析结果，不经过 TUI 的展示或状态管理路径。

use std::io::Read;
use std::io::{self};

fn main() {
    // 标准输入必须完整读入后再创建解析器；读取失败以非零状态明确退出。
    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("failed to read stdin: {err}");
        std::process::exit(1);
    }

    let parser = pulldown_cmark::Parser::new(&input);
    // 使用 Debug 表示保留事件种类和载荷，便于与 Markdown 流式渲染行为对照。
    for event in parser {
        println!("{event:?}");
    }
}
