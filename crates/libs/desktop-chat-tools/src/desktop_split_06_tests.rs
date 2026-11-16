//! 从桌面宿主迁入的聊天工具局部行为测试。
use crate::tools::view_fixture::TestApp;
fn card(app: &TestApp) -> crate::card_input::CardInput<'_> {
    app.files().card
}
mod advanced;
mod bash;
mod command;
mod header;
mod patch;
